use crate::args::MergedConfig;

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
    let ix1 = (w - config.right as f64).max(ix0); // defensive
    let iy1 = (h - config.bottom as f64).max(iy0);

    if radius <= 0.0 {
        for y in config.top..(height - config.bottom) {
            for x in config.left..(width - config.right) {
                let idx = ((y * width + x) * 4) as usize;
                canvas[idx..idx + 4].fill(0);
            }
        }
        return; // skip rounded-corner logic
    }
    
    // Optional: small antialias band (in pixels)
    let aa = 1.0_f64;
    
    // Iterate pixels, compute distance to rounded rect:
    // distance to rounded box = length(max( (px - [ix0,ix1]) , 0 )) - radius
    // If <= 0 => inside rounded rect (we want transparent).
    for y in 0..height {
        for x in 0..width {
            let xf = x as f64 + 0.5; // sample at pixel center
            let yf = y as f64 + 0.5;
            let idx = ((y * width + x) * 4) as usize;
            
            // distance in X
            let dx = if xf < ix0 {
                ix0 - xf
            } else if xf > ix1 {
                xf - ix1
            } else {
                0.0
            };
            
            // distance in Y
            let dy = if yf < iy0 {
                iy0 - yf
            } else if yf > iy1 {
                yf - iy1
            } else {
                0.0
            };
            
            let dist = (dx * dx + dy * dy).sqrt();
            
            // dist_to_rounded_rect = dist - radius
            let drr = dist - radius;
            
            if drr <= -aa {
                // well inside rounded inner rect -> fully transparent
                canvas[idx..idx + 4].fill(0);
            } else if drr < aa {
                // partially inside the AA band -> compute coverage (0..1)
                // coverage = clamp(0.5 - drr/(2*aa), 0..1)  (a soft step)
                let t = (drr + aa) / (2.0 * aa); // maps [-aa, +aa] -> [0,1]
                let coverage = 1.0 - t.clamp(0.0, 1.0); // 1 => fully transparent, 0 => fully opaque
                
                // we are premultiplied. existing pixel is pr,pg,pb,pa.
                // set pixel alpha = (1 - coverage) * pa
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
            } else {
                // drr >= aa -> outside rounded inner rect (keep border color) -> nothing to do
            }
        }
    }
}
