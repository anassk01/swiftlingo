use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use crate::translation::{TranslationService, ServiceConfig};

/// Application settings including appearance, defaults, and API configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // General settings
    pub dark_mode: bool,
    pub default_source_lang: String,
    pub default_target_lang: String,
    pub window_width: i32,
    pub window_height: i32,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub startup_minimized: bool,
    
    // Translation service settings
    pub active_service: TranslationService,
    pub service_configs: HashMap<String, ServiceConfig>,
    
    // History settings
    pub max_history_entries: i32,
    pub auto_save_history: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mut service_configs = HashMap::new();
        
        // Default configuration for Google Beta (no API key needed)
        service_configs.insert(
            "GoogleBeta".to_string(),
            ServiceConfig {
                api_key: None,
                endpoint: None,
                timeout_seconds: Some(5),
            },
        );
        
        // Default configuration for other services (need API keys)
        service_configs.insert(
            "GoogleOfficial".to_string(),
            ServiceConfig {
                api_key: None,
                endpoint: None,
                timeout_seconds: Some(5),
            },
        );
        
        service_configs.insert(
            "LibreTranslate".to_string(),
            ServiceConfig {
                api_key: None,
                endpoint: Some("https://libretranslate.com/translate".to_string()),
                timeout_seconds: Some(5),
            },
        );
        
        service_configs.insert(
            "Bing".to_string(),
            ServiceConfig {
                api_key: None,
                endpoint: None,
                timeout_seconds: Some(5),
            },
        );
        
        service_configs.insert(
            "DeepL".to_string(),
            ServiceConfig {
                api_key: None,
                endpoint: None,
                timeout_seconds: Some(5),
            },
        );
        
        Settings {
            dark_mode: false,
            default_source_lang: "auto".to_string(),
            default_target_lang: "en".to_string(),
            window_width: 800,
            window_height: 500,
            window_x: None,
            window_y: None,
            startup_minimized: false,
            active_service: TranslationService::GoogleBeta,
            service_configs,
            max_history_entries: 100,
            auto_save_history: true,
        }
    }
}

impl Settings {
    /// Load settings from file or create with defaults
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        
        if Path::new(&config_path).exists() {
            match fs::File::open(&config_path) {
                Ok(mut file) => {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        if let Ok(settings) = serde_json::from_str(&contents) {
                            return settings;
                        }
                    }
                },
                _ => {}
            }
        }
        
        // If loading fails, create default settings
        let default_settings = Settings::default();
        let _ = default_settings.save(); // Save defaults
        default_settings
    }
    
    /// Save settings to file
    pub fn save(&self) -> bool {
        let config_path = Self::get_config_path();
        let config_dir = Path::new(&config_path).parent().unwrap();
        
        if let Err(_) = fs::create_dir_all(config_dir) {
            return false;
        }
        
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                match fs::File::create(&config_path) {
                    Ok(mut file) => {
                        if file.write_all(json.as_bytes()).is_ok() {
                            return true;
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
        
        false
    }
    
    /// Get configuration file path
    fn get_config_path() -> String {
        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_dir = format!("{}/.config/translator-app", home_dir);
        format!("{}/settings.json", config_dir)
    }
    
    #[allow(dead_code)]
    pub fn update_service_config(&mut self, service_name: &str, config: ServiceConfig) {
        self.service_configs.insert(service_name.to_string(), config);
    }
    
    #[allow(dead_code)]
    pub fn get_service_config(&self, service: &TranslationService) -> Option<ServiceConfig> {
        let service_name = match service {
            TranslationService::GoogleBeta => "GoogleBeta",
            TranslationService::GoogleOfficial => "GoogleOfficial",
            TranslationService::LibreTranslate => "LibreTranslate",
            TranslationService::Bing => "Bing",
            TranslationService::DeepL => "DeepL",
        };
        
        self.service_configs.get(service_name).cloned()
    }
    
    #[allow(dead_code)]
    pub fn update_window_geometry(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.window_x = Some(x);
        self.window_y = Some(y);
        self.window_width = width;
        self.window_height = height;
        self.save();
    }
}