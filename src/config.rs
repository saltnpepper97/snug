use std::path::PathBuf;
use std::collections::HashMap;
use eyre::{Result, eyre};
use rune_cfg::RuneConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub radius: i32,
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub color: String,
    pub opacity: Option<f64>,
    // Shadow properties
    pub shadow_enabled: Option<bool>,
    pub shadow_color: Option<String>,
    pub shadow_opacity: Option<f64>,
    pub shadow_blur: Option<f64>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            radius: 15,
            left: 30,
            right: 30,
            top: 30,
            bottom: 30,
            color: "000000".to_string(),
            opacity: None,
            shadow_enabled: None,
            shadow_color: None,
            shadow_opacity: None,
            shadow_blur: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnugConfig {
    pub displays: HashMap<String, DisplayConfig>,
}

impl SnugConfig {
    pub fn get_display_config(&self, display_name: &str) -> DisplayConfig {
        self.displays
            .get(display_name)
            .cloned()
            .unwrap_or_default()
    }
}

/// Expands ~ to the home directory
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

pub fn load_config(path: &str) -> Result<SnugConfig> {
    load_config_internal(path, false)
}

pub fn load_config_silent(path: &str) -> Result<SnugConfig> {
    load_config_internal(path, true)
}

fn load_config_internal(path: &str, silent: bool) -> Result<SnugConfig> {
    let expanded_path = expand_tilde(path);
    
    let config = RuneConfig::from_file(expanded_path.to_str().unwrap())
        .map_err(|e| eyre!("Failed to load config: {}", e))?;
    
    let mut displays = HashMap::new();

    let possible_displays = vec![
        "DP-1", "DP-2", "DP-3", "DP-4", 
        "DP-5", "DP-6", "DP-7", "DP-8",
        
        "HDMI-A-1", "HDMI-A-2", "HDMI-A-3", "HDMI-A-4",
        "HDMI-1", "HDMI-2", "HDMI-3", "HDMI-4",
        
        "eDP-1", "eDP-2",
        
        "DVI-D-1", "DVI-D-2",
        "DVI-I-1", "DVI-I-2", 
        
        "HEADLESS-1", "HEADLESS-2",
        "VIRTUAL1", "VIRTUAL2",
    ];
    
    for display in possible_displays {
        if let Ok(_) = config.get::<i32>(&format!("{}.radius", display)) {
            let display_config = DisplayConfig {
                radius: config.get_or(&format!("{}.radius", display), 15),
                left: config.get_or(&format!("{}.left", display), 30),
                right: config.get_or(&format!("{}.right", display), 30),
                top: config.get_or(&format!("{}.top", display), 30),
                bottom: config.get_or(&format!("{}.bottom", display), 30),
                color: config.get_or(&format!("{}.color", display), "000000".to_string()),
                opacity: config.get(&format!("{}.opacity", display)).ok(),
                shadow_enabled: config.get(&format!("{}.shadow_enabled", display)).ok(),
                shadow_color: config.get(&format!("{}.shadow_color", display)).ok(),
                shadow_opacity: config.get(&format!("{}.shadow_opacity", display)).ok(),
                shadow_blur: config.get(&format!("{}.shadow_blur", display)).ok(),
            };
            displays.insert(display.to_string(), display_config);
            if !silent {
                eprintln!("✓ Loaded config for display: {}", display);
            }
        }
    }
    
    if displays.is_empty() {
        displays.insert("default".to_string(), DisplayConfig::default());
    }
    
    Ok(SnugConfig { displays })
}

pub fn find_config() -> Option<PathBuf> {
    if let Some(home) = dirs::config_dir() {
        let user_config = home.join("snug").join("snug.rune");
        if user_config.exists() {
            return Some(user_config);
        }
    }
    let default_config = PathBuf::from("/usr/share/doc/snug/snug.rune");
    if default_config.exists() {
        return Some(default_config);
    }
    None
}

pub fn load_config_or_default() -> SnugConfig {
    match find_config() {
        Some(path) => match load_config(&path.to_string_lossy()) {
            Ok(cfg) => cfg,
            Err(err) => {
                eprintln!("❌ Configuration error: {}\nUsing defaults.", err);
                SnugConfig {
                    displays: {
                        let mut map = HashMap::new();
                        map.insert("default".to_string(), DisplayConfig::default());
                        map
                    }
                }
            }
        },
        None => {
            SnugConfig {
                displays: {
                    let mut map = HashMap::new();
                    map.insert("default".to_string(), DisplayConfig::default());
                    map
                }
            }
        }
    }
}

pub fn get_config_path() -> PathBuf {
    find_config().unwrap_or_else(|| {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("snug").join("snug.rune")
        } else {
            PathBuf::from("snug.rune")
        }
    })
}
