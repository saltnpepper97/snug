pub fn parse_colour(hex: &str, opacity_override: Option<f64>) -> (u8, u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    
    // Parse RGB components
    let (r, g, b) = if hex.len() >= 6 {
        (
            u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };
    
    // Parse alpha component if present (8 hex digits = RGBA)
    let a = if let Some(opacity) = opacity_override {
        // Opacity override takes precedence
        (opacity.clamp(0.0, 1.0) * 255.0) as u8
    } else if hex.len() >= 8 {
        // Parse alpha from hex string
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
    } else {
        // Default to fully opaque
        255
    };
    
    (r, g, b, a)
}
