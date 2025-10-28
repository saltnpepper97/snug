use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Rounded corner border overlay for Wayland")]
pub struct Args {
    /// Specify a custom configuration file
    #[arg(short, long)]
    pub config: Option<String>,
    
    /// Target display name (e.g., DP-1, HDMI-A-1)
    #[arg(short, long)]
    pub display: Option<String>,
    
    /// Corner radius in pixels (overrides config)
    #[arg(short, long)]
    pub radius: Option<i32>,
    
    /// Border width for left edge (overrides config)
    #[arg(long)]
    pub left: Option<i32>,
    
    /// Border width for right edge (overrides config)
    #[arg(long)]
    pub right: Option<i32>,
    
    /// Border width for top edge (overrides config)
    #[arg(long)]
    pub top: Option<i32>,
    
    /// Border width for bottom edge (overrides config)
    #[arg(long)]
    pub bottom: Option<i32>,
    
    /// Color in hex format (RGB: 000000 or RGBA: 000000ff) (overrides config)
    #[arg(long)]
    pub color: Option<String>,
    
    /// Opacity (0.0 to 1.0) - overrides alpha channel if present in color
    #[arg(long)]
    pub opacity: Option<f64>,
    
    /// Enable shadow (overrides config)
    #[arg(long)]
    pub shadow_enabled: Option<bool>,
    
    /// Shadow color in hex format (overrides config)
    #[arg(long)]
    pub shadow_color: Option<String>,
    
    /// Shadow opacity (0.0 to 1.0) (overrides config)
    #[arg(long)]
    pub shadow_opacity: Option<f64>,
    
    /// Shadow blur radius (overrides config)
    #[arg(long)]
    pub shadow_blur: Option<f64>,
}

impl Args {
    /// Merge CLI args with config, CLI takes precedence
    pub fn merge_with_config(&self, config: &crate::config::DisplayConfig) -> MergedConfig {
        MergedConfig {
            radius: self.radius.unwrap_or(config.radius),
            left: self.left.unwrap_or(config.left),
            right: self.right.unwrap_or(config.right),
            top: self.top.unwrap_or(config.top),
            bottom: self.bottom.unwrap_or(config.bottom),
            color: self.color.clone().unwrap_or_else(|| config.color.clone()),
            opacity: self.opacity.or(config.opacity),
            shadow_enabled: self.shadow_enabled.or(config.shadow_enabled),
            shadow_color: self.shadow_color.clone().or_else(|| config.shadow_color.clone()),
            shadow_opacity: self.shadow_opacity.or(config.shadow_opacity),
            shadow_blur: self.shadow_blur.or(config.shadow_blur),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MergedConfig {
    pub radius: i32,
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub color: String,
    pub opacity: Option<f64>,
    pub shadow_enabled: Option<bool>,
    pub shadow_color: Option<String>,
    pub shadow_opacity: Option<f64>,
    pub shadow_blur: Option<f64>,
}
