use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use urlencoding::encode;

/// Represents available translation services
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TranslationService {
    GoogleBeta,    // Current implementation (free)
    GoogleOfficial, // Official Google API (requires key)
    LibreTranslate, // Open-source alternative
    Bing,          // Microsoft Translator
    DeepL,         // DeepL API
}

impl fmt::Display for TranslationService {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TranslationService::GoogleBeta => write!(f, "Google Translate (Beta)"),
            TranslationService::GoogleOfficial => write!(f, "Google Translate (Official)"),
            TranslationService::LibreTranslate => write!(f, "LibreTranslate"),
            TranslationService::Bing => write!(f, "Bing Translator"),
            TranslationService::DeepL => write!(f, "DeepL"),
        }
    }
}

impl TranslationService {
    /// Get service name for configuration lookup
    #[allow(dead_code)]
    pub fn config_name(&self) -> &'static str {
        match self {
            TranslationService::GoogleBeta => "GoogleBeta",
            TranslationService::GoogleOfficial => "GoogleOfficial",
            TranslationService::LibreTranslate => "LibreTranslate",
            TranslationService::Bing => "Bing",
            TranslationService::DeepL => "DeepL",
        }
    }
    
    /// Get all available services
    pub fn all_services() -> Vec<TranslationService> {
        vec![
            TranslationService::GoogleBeta,
            TranslationService::GoogleOfficial,
            TranslationService::LibreTranslate,
            TranslationService::Bing,
            TranslationService::DeepL,
        ]
    }
}

/// Configuration for a translation service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub timeout_seconds: Option<u64>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        ServiceConfig {
            api_key: None,
            endpoint: None,
            timeout_seconds: Some(5),
        }
    }
}

/// Common translation request parameters
struct TranslationRequest<'a> {
    text: &'a str,
    source_lang: &'a str,
    target_lang: &'a str,
    config: &'a ServiceConfig,
    client: &'a Client,
}

/// Manages translation services and their configurations
pub struct TranslationManager {
    client: Client,
    active_service: TranslationService,
    configs: HashMap<TranslationService, ServiceConfig>,
}

impl TranslationManager {
    /// Create a new translation manager with default configuration
    pub fn new() -> Self {
        // Create a client with default timeouts
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());
        
        // Set up default configurations
        let mut configs = HashMap::new();
        
        configs.insert(TranslationService::GoogleBeta, ServiceConfig::default());
        configs.insert(TranslationService::GoogleOfficial, ServiceConfig::default());
        configs.insert(TranslationService::LibreTranslate, ServiceConfig {
            endpoint: Some("https://libretranslate.com/translate".to_string()),
            ..ServiceConfig::default()
        });
        configs.insert(TranslationService::Bing, ServiceConfig::default());
        configs.insert(TranslationService::DeepL, ServiceConfig::default());
        
        TranslationManager {
            client,
            active_service: TranslationService::GoogleBeta,
            configs,
        }
    }
    
    /// Set the active translation service
    pub fn set_active_service(&mut self, service: TranslationService) {
        self.active_service = service;
    }
    
    /// Get the current active service
    #[allow(dead_code)]
    pub fn get_active_service(&self) -> TranslationService {
        self.active_service.clone()
    }
    
    /// Update configuration for a service
    #[allow(dead_code)]
    pub fn update_config(&mut self, service: TranslationService, config: ServiceConfig) {
        self.configs.insert(service, config);
    }
    
    /// Get configuration for a service
    pub fn get_config(&self, service: &TranslationService) -> ServiceConfig {
        self.configs.get(service)
            .cloned()
            .unwrap_or_else(ServiceConfig::default)
    }
    
    /// Get all available services
    #[allow(dead_code)]
    pub fn get_available_services(&self) -> Vec<TranslationService> {
        TranslationService::all_services()
    }
    
    /// Translate text using the active service
    pub async fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String, String> {
        if text.is_empty() {
            return Ok("Please enter text to translate".to_string());
        }
        
        let config = self.get_config(&self.active_service);
        let request = TranslationRequest {
            text,
            source_lang,
            target_lang,
            config: &config,
            client: &self.client,
        };
        
        match self.active_service {
            TranslationService::GoogleBeta => self.translate_google_beta(&request).await,
            TranslationService::GoogleOfficial => self.translate_google_official(&request).await,
            TranslationService::LibreTranslate => self.translate_libre(&request).await,
            TranslationService::Bing => self.translate_bing(&request).await,
            TranslationService::DeepL => self.translate_deepl(&request).await,
        }
    }
    
    /// Fall back to another service if the primary one fails
    pub async fn translate_with_fallback(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String, String> {
        // Try the active service first
        let result = self.translate(text, source_lang, target_lang).await;
        
        if result.is_ok() {
            return result;
        }
        
        // On failure, try Google Beta as a fallback (if it's not already the active service)
        if self.active_service != TranslationService::GoogleBeta {
            println!("Primary translation service failed, falling back to Google Beta");
            let config = self.get_config(&TranslationService::GoogleBeta);
            let request = TranslationRequest {
                text,
                source_lang,
                target_lang,
                config: &config,
                client: &self.client,
            };
            return self.translate_google_beta(&request).await;
        }
        
        // If Google Beta is already the active service and it failed, return the error
        result
    }
    
    // IMPLEMENTATION OF TRANSLATION SERVICES
    
    /// Helper function to process HTTP responses
    async fn process_response(&self, response: reqwest::Response) -> Result<serde_json::Value, String> {
        if !response.status().is_success() {
            return Err(format!("Error: Server returned status {}", response.status()));
        }
        
        match response.json().await {
            Ok(json) => Ok(json),
            Err(e) => Err(format!("Error: Could not parse response: {}", e)),
        }
    }
    
    /// Google Translate (Beta/Free) implementation
    async fn translate_google_beta(&self, request: &TranslationRequest<'_>) -> Result<String, String> {
        // Properly URL encode the text
        let encoded_text = encode(request.text);
        
        // Format the URL
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            request.source_lang, request.target_lang, encoded_text
        );
        
        // Get the timeout from config
        let timeout = Duration::from_secs(request.config.timeout_seconds.unwrap_or(10));
        
        // Make the request
        let response = match request.client.get(&url)
            .timeout(timeout)
            .send()
            .await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Error: Could not connect to translation service: {}", e)),
        };
        
        // Parse the response
        let json = self.process_response(response).await?;
        
        // Build the complete translation from all segments
        let mut full_translation = String::new();
        
        // The response structure is an array of arrays, where the first array contains
        // all the translation segments
        if let Some(segments) = json[0].as_array() {
            for segment in segments {
                if let Some(text) = segment[0].as_str() {
                    full_translation.push_str(text);
                }
            }
        } else {
            return Err("Translation error: Unexpected response format".to_string());
        }
        
        Ok(full_translation)
    }
    
    /// Google Translate (Official API) implementation
    async fn translate_google_official(&self, request: &TranslationRequest<'_>) -> Result<String, String> {
        // Get API key from config
        let api_key = match &request.config.api_key {
            Some(key) => key,
            None => return Err("Google Translate API key not configured".to_string()),
        };
        
        // Properly URL encode the text
        let encoded_text = encode(request.text);
        
        // Format the URL with API key
        let url = format!(
            "https://translation.googleapis.com/language/translate/v2?key={}&source={}&target={}&q={}",
            api_key, request.source_lang, request.target_lang, encoded_text
        );
        
        // Make the request
        let response = match request.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Error: Could not connect to translation service: {}", e)),
        };
        
        // Parse the response
        let json = self.process_response(response).await?;
        
        // Extract the translation from the response
        if let Some(data) = json.get("data") {
            if let Some(translations) = data.get("translations") {
                if let Some(translation) = translations[0].get("translatedText") {
                    if let Some(text) = translation.as_str() {
                        return Ok(text.to_string());
                    }
                }
            }
        }
        
        Err("Translation error: Unexpected response format".to_string())
    }
    
    /// LibreTranslate implementation
    async fn translate_libre(&self, request: &TranslationRequest<'_>) -> Result<String, String> {
        // Get API endpoint from config
        let endpoint = match &request.config.endpoint {
            Some(ep) => ep,
            None => return Err("LibreTranslate API endpoint not configured".to_string()),
        };
        
        // Prepare request body
        let mut request_body = serde_json::json!({
            "q": request.text,
            "source": request.source_lang,
            "target": request.target_lang,
        });
        
        // Add API key if present
        if let Some(api_key) = &request.config.api_key {
            request_body["api_key"] = serde_json::Value::String(api_key.clone());
        }
        
        // Make the request
        let response = match request.client.post(endpoint)
            .json(&request_body)
            .send()
            .await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Error: Could not connect to translation service: {}", e)),
        };
        
        // Parse the response
        let json = self.process_response(response).await?;
        
        // Extract the translation
        if let Some(translation) = json.get("translatedText") {
            if let Some(text) = translation.as_str() {
                return Ok(text.to_string());
            }
        }
        
        Err("Translation error: Unexpected response format".to_string())
    }
    
    /// Bing Translator implementation
    async fn translate_bing(&self, request: &TranslationRequest<'_>) -> Result<String, String> {
        // Get API key from config
        let api_key = match &request.config.api_key {
            Some(key) => key,
            None => return Err("Bing Translator API key not configured".to_string()),
        };
        
        // Prepare the endpoint
        let endpoint = "https://api.cognitive.microsofttranslator.com/translate";
        
        // Prepare the request
        let mut request_builder = request.client.post(endpoint)
            .header("Ocp-Apim-Subscription-Key", api_key)
            .header("Content-Type", "application/json");
        
        // Add region if provided
        if let Some(region) = &request.config.endpoint {
            request_builder = request_builder.header("Ocp-Apim-Subscription-Region", region);
        }
        
        // Add query parameters
        let query_params = [
            ("api-version", "3.0"),
            ("from", request.source_lang),
            ("to", request.target_lang),
        ];
        
        request_builder = request_builder.query(&query_params);
        
        // Prepare body
        let body = serde_json::json!([{"text": request.text}]);
        
        // Make the request
        let response = match request_builder
            .json(&body)
            .send()
            .await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Error: Could not connect to translation service: {}", e)),
        };
        
        // Parse the response
        let json = self.process_response(response).await?;
        
        // Extract the translation
        if let Some(translations) = json[0].get("translations") {
            if let Some(translation) = translations[0].get("text") {
                if let Some(text) = translation.as_str() {
                    return Ok(text.to_string());
                }
            }
        }
        
        Err("Translation error: Unexpected response format".to_string())
    }
    
    /// DeepL implementation
    async fn translate_deepl(&self, request: &TranslationRequest<'_>) -> Result<String, String> {
        // Get API key from config
        let api_key = match &request.config.api_key {
            Some(key) => key,
            None => return Err("DeepL API key not configured".to_string()),
        };
        
        // Determine endpoint based on API key type (free vs pro)
        let is_free_api = api_key.ends_with(":fx");
        let endpoint = if is_free_api {
            "https://api-free.deepl.com/v2/translate"
        } else {
            "https://api.deepl.com/v2/translate"
        };
        
        // Map language codes (DeepL uses different codes for some languages)
        let source_lang_mapped = match request.source_lang {
            "auto" => "auto".to_string(),
            "en" => "EN".to_string(),
            "zh-CN" => "ZH".to_string(),
            "ja" => "JA".to_string(),
            // Add more mappings as needed
            _ => request.source_lang.to_uppercase(),
        };
        
        let target_lang_mapped = match request.target_lang {
            "en" => "EN-US".to_string(), // DeepL distinguishes between EN-US and EN-GB
            "zh-CN" => "ZH".to_string(),
            "pt" => "PT-BR".to_string(), // DeepL distinguishes between PT-PT and PT-BR
            // Add more mappings as needed
            _ => request.target_lang.to_uppercase(),
        };
        
        // Prepare request body
        let mut params = vec![
            ("text", request.text.to_string()),
            ("target_lang", target_lang_mapped),
        ];
        
        // Only add source language if not auto
        if request.source_lang != "auto" {
            params.push(("source_lang", source_lang_mapped));
        }
        
        // Make the request
        let response = match request.client.post(endpoint)
            .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
            .form(&params)
            .send()
            .await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Error: Could not connect to translation service: {}", e)),
        };
        
        // Parse the response
        let json = self.process_response(response).await?;
        
        // Extract the translation
        if let Some(translations) = json.get("translations") {
            if let Some(translation) = translations[0].get("text") {
                if let Some(text) = translation.as_str() {
                    return Ok(text.to_string());
                }
            }
        }
        
        Err("Translation error: Unexpected response format".to_string())
    }
}