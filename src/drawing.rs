use crate::args::MergedConfig;

/// Parse hex color string to RGB
fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (0, 0, 0)
    }
}

/// Smooth falloff function for shadows (approximates Gaussian)
fn shadow_falloff(distance: f64, blur_radius: f64) -> f64 {
    if distance <= 0.0 {
        return 1.0;
    }
    if distance >= blur_radius {
        return 0.0;
    }
    
    // Smoothstep-based falloff for soft edges
    let t = distance / blur_radius;
    let smooth = 1.0 - (3.0 * t * t - 2.0 * t * t * t);
    
    // Add extra softness with exponential decay
    let exp_factor = (-3.0 * t).exp();
    
    (smooth * 0.7 + exp_factor * 0.3).clamp(0.0, 1.0)
}

pub fn draw_snug(
    canvas: &mut [u8],
    width: i32,
    height: i32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    config: &MergedConfig
) {
    let w = width as f64;
    let h = height as f64;
    let radius = config.radius as f64;
    
    // premultiplied color
    let af = a as f32 / 255.0;
    let pr = (r as f32 * af).round() as u8;
    let pg = (g as f32 * af).round() as u8;
    let pb = (b as f32 * af).round() as u8;
    let pa = a;
    
    // Fill background with premultiplied color
    for chunk in canvas.chunks_exact_mut(4) {
        chunk.copy_from_slice(&[pb, pg, pr, pa]);
    }
    
    // Inner rectangle coordinates (local buffer coords)
    let ix0 = config.left as f64;
    let iy0 = config.top as f64;
    let ix1 = (w - config.right as f64).max(ix0);
    let iy1 = (h - config.bottom as f64).max(iy0);
    
    if radius <= 0.0 {
        for y in config.top..(height - config.bottom) {
            for x in config.left..(width - config.right) {
                let idx = ((y * width + x) * 4) as usize;
                canvas[idx..idx + 4].fill(0);
            }
        }
        return;
    }
    
    let aa = 1.0_f64;
    
    // Get shadow config with clamping
    let shadow_enabled = config.shadow_enabled.unwrap_or(false);
    let shadow_color_str = config.shadow_color.as_ref()
        .map(|s| s.as_str())
        .unwrap_or("000000");
    let (sr, sg, sb) = parse_hex_color(shadow_color_str);
    let shadow_opacity = config.shadow_opacity.unwrap_or(0.5).clamp(0.0, 1.0);
    
    // Clamp shadow_blur: config value is 0.0-1.0, map to 1.0-15.0 pixels
    let shadow_blur_config = config.shadow_blur.unwrap_or(0.5).clamp(0.0, 1.0);
    let shadow_blur = 1.0 + (shadow_blur_config * 14.0); // Maps 0.0->1.0, 1.0->15.0
    
    // Iterate pixels
    for y in 0..height {
        for x in 0..width {
            let xf = x as f64 + 0.5;
            let yf = y as f64 + 0.5;
            let idx = ((y * width + x) * 4) as usize;
            
            // Distance to rectangle edges
            let dx = if xf < ix0 {
                ix0 - xf
            } else if xf > ix1 {
                xf - ix1
            } else {
                0.0
            };
            
            let dy = if yf < iy0 {
                iy0 - yf
            } else if yf > iy1 {
                yf - iy1
            } else {
                0.0
            };
            
            let dist = (dx * dx + dy * dy).sqrt();
            let drr = dist - radius;
            
            if drr <= -aa {
                // Inside the cutout
                if shadow_enabled {
                    // Distance from the inner edge (positive = inside, away from edge)
                    let inner_dist = -drr;
                    
                    // Only draw shadow within blur radius
                    if inner_dist <= shadow_blur {
                        let falloff = shadow_falloff(inner_dist, shadow_blur);
                        let shadow_strength = shadow_opacity * falloff;
                        
                        if shadow_strength > 0.001 {
                            let sa = (shadow_strength as f32).min(1.0);
                            
                            // Premultiply shadow color
                            let sr_pm = (sr as f32 / 255.0) * sa;
                            let sg_pm = (sg as f32 / 255.0) * sa;
                            let sb_pm = (sb as f32 / 255.0) * sa;
                            
                            canvas[idx] = (sb_pm * 255.0).round() as u8;
                            canvas[idx + 1] = (sg_pm * 255.0).round() as u8;
                            canvas[idx + 2] = (sr_pm * 255.0).round() as u8;
                            canvas[idx + 3] = (sa * 255.0).round() as u8;
                            continue;
                        }
                    }
                }
                
                // No shadow or outside shadow range
                canvas[idx..idx + 4].fill(0);
                
            } else if drr < aa {
                // AA band at the border edge
                let t = (drr + aa) / (2.0 * aa);
                let coverage = 1.0 - t.clamp(0.0, 1.0);
                
                if shadow_enabled && coverage > 0.001 {
                    // At the edge, blend shadow with border
                    let shadow_strength = shadow_opacity;
                    let sa = (shadow_strength as f32 * coverage as f32).min(1.0);
                    
                    let sr_pm = (sr as f32 / 255.0) * sa;
                    let sg_pm = (sg as f32 / 255.0) * sa;
                    let sb_pm = (sb as f32 / 255.0) * sa;
                    
                    let border_factor = (1.0 - coverage) as f32;
                    let border_a = (pa as f32 / 255.0) * border_factor;
                    
                    let out_alpha = sa + border_a;
                    
                    if out_alpha > 0.001 {
                        let out_r_pm = sr_pm + (pr as f32 / 255.0) * border_factor;
                        let out_g_pm = sg_pm + (pg as f32 / 255.0) * border_factor;
                        let out_b_pm = sb_pm + (pb as f32 / 255.0) * border_factor;
                        
                        canvas[idx] = (out_b_pm * 255.0).round() as u8;
                        canvas[idx + 1] = (out_g_pm * 255.0).round() as u8;
                        canvas[idx + 2] = (out_r_pm * 255.0).round() as u8;
                        canvas[idx + 3] = (out_alpha * 255.0).round() as u8;
                    } else {
                        canvas[idx..idx + 4].fill(0);
                    }
                } else {
                    // No shadow - original AA
                    let out_alpha = (1.0 - coverage) * (pa as f64 / 255.0);
                    if out_alpha <= 0.0 {
                        canvas[idx..idx + 4].fill(0);
                    } else {
                        let out_a_u8 = (out_alpha * 255.0).round() as u8;
                        let out_r = ((pr as f32) * (out_alpha as f32 / (pa as f32 / 255.0))).round() as u8;
                        let out_g = ((pg as f32) * (out_alpha as f32 / (pa as f32 / 255.0))).round() as u8;
                        let out_b = ((pb as f32) * (out_alpha as f32 / (pa as f32 / 255.0))).round() as u8;
                        canvas[idx..idx + 4].copy_from_slice(&[out_b, out_g, out_r, out_a_u8]);
                    }
                }
            }
            // else: outside border, keep as-is
        }
    }
}
