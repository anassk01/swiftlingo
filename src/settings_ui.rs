use gtk::prelude::*;
use gtk::{
    Box as GtkBox, ComboBoxText, Entry, Label,
    Orientation, ScrolledWindow, Switch, Frame, Notebook, Separator, Dialog, ResponseType, Window
};
use std::rc::Rc;
use std::cell::RefCell;

use crate::settings::Settings;
use crate::translation::{TranslationService, ServiceConfig};
use crate::languages::LANGUAGES;
use crate::apply_theme;

pub struct SettingsDialog {
    dialog: Dialog,
    settings: Rc<RefCell<Settings>>,
    
    // API Configuration widgets
    api_entries: Rc<RefCell<Vec<(TranslationService, Entry, Entry)>>>,
    
    // Appearance widgets
    dark_mode_switch: Switch,
    
    // Default language widgets
    default_source_lang: ComboBoxText,
    default_target_lang: ComboBoxText,
}

impl SettingsDialog {
    pub fn new(parent: &impl IsA<Window>, settings: Rc<RefCell<Settings>>) -> Self {
        // Create the dialog
        let dialog = Dialog::new();
        dialog.set_title(Some("Settings"));
        dialog.set_default_width(500);
        dialog.set_default_height(400);
        dialog.set_modal(true);
        dialog.set_transient_for(Some(parent));
        
        // Add buttons
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Apply", ResponseType::Apply);
        dialog.add_button("OK", ResponseType::Ok);
        
        // Get the content area
        let content_area = dialog.content_area();
        content_area.set_margin_start(10);
        content_area.set_margin_end(10);
        content_area.set_margin_top(10);
        content_area.set_margin_bottom(10);
        
        // Create a notebook (tabbed interface)
        let notebook = Notebook::new();
        notebook.set_vexpand(true);
        notebook.set_hexpand(true);
        
        // ---- General Tab ----
        let general_page = GtkBox::new(Orientation::Vertical, 10);
        general_page.set_margin_start(10);
        general_page.set_margin_end(10);
        general_page.set_margin_top(10);
        general_page.set_margin_bottom(10);
        
        // Appearance section
        let appearance_frame = Frame::new(Some("Appearance"));
        let appearance_box = GtkBox::new(Orientation::Vertical, 5);
        appearance_box.set_margin_start(10);
        appearance_box.set_margin_end(10);
        appearance_box.set_margin_top(10);
        appearance_box.set_margin_bottom(10);
        
        // Dark mode switch
        let dark_mode_box = GtkBox::new(Orientation::Horizontal, 10);
        let dark_mode_label = Label::new(Some("Dark Mode"));
        dark_mode_label.set_halign(gtk::Align::Start);
        dark_mode_label.set_hexpand(true);
        
        let dark_mode_switch = Switch::new();
        dark_mode_switch.set_halign(gtk::Align::End);
        
        dark_mode_box.append(&dark_mode_label);
        dark_mode_box.append(&dark_mode_switch);
        
        
        // Startup minimized switch
        let startup_box = GtkBox::new(Orientation::Horizontal, 10);
        let startup_label = Label::new(Some("Start Minimized"));
        startup_label.set_halign(gtk::Align::Start);
        startup_label.set_hexpand(true);
        
        let startup_switch = Switch::new();
        startup_switch.set_halign(gtk::Align::End);
        
        startup_box.append(&startup_label);
        startup_box.append(&startup_switch);
        
        appearance_box.append(&dark_mode_box);
        appearance_box.append(&startup_box);
        
        appearance_frame.set_child(Some(&appearance_box));
        
        // Default languages section
        let lang_frame = Frame::new(Some("Default Languages"));
        let lang_box = GtkBox::new(Orientation::Vertical, 5);
        lang_box.set_margin_start(10);
        lang_box.set_margin_end(10);
        lang_box.set_margin_top(10);
        lang_box.set_margin_bottom(10);
        
        // Source language
        let source_box = GtkBox::new(Orientation::Horizontal, 10);
        let source_label = Label::new(Some("Default Source Language:"));
        source_label.set_halign(gtk::Align::Start);
        
        let default_source_lang = ComboBoxText::new();
        default_source_lang.set_hexpand(true);
        default_source_lang.append(Some("auto"), "Detect language");
        for (code, name) in LANGUAGES.iter() {
            if *code != "auto" {
                default_source_lang.append(Some(code), name);
            }
        }
        
        source_box.append(&source_label);
        source_box.append(&default_source_lang);
        
        // Target language
        let target_box = GtkBox::new(Orientation::Horizontal, 10);
        let target_label = Label::new(Some("Default Target Language:"));
        target_label.set_halign(gtk::Align::Start);
        
        let default_target_lang = ComboBoxText::new();
        default_target_lang.set_hexpand(true);
        for (code, name) in LANGUAGES.iter() {
            if *code != "auto" {
                default_target_lang.append(Some(code), name);
            }
        }
        
        target_box.append(&target_label);
        target_box.append(&default_target_lang);
        
        lang_box.append(&source_box);
        lang_box.append(&target_box);
        
        lang_frame.set_child(Some(&lang_box));
        
        // Add sections to general page
        general_page.append(&appearance_frame);
        general_page.append(&lang_frame);
        
        
        // ---- API Settings Tab ----
        let api_page = GtkBox::new(Orientation::Vertical, 10);
        api_page.set_margin_start(10);
        api_page.set_margin_end(10);
        api_page.set_margin_top(10);
        api_page.set_margin_bottom(10);
        
        // Service selection section
        let service_frame = Frame::new(Some("Active Translation Service"));
        let service_box = GtkBox::new(Orientation::Vertical, 5);
        service_box.set_margin_start(10);
        service_box.set_margin_end(10);
        service_box.set_margin_top(10);
        service_box.set_margin_bottom(10);
        
        // Service selector
        let selector_box = GtkBox::new(Orientation::Horizontal, 10);
        let service_label = Label::new(Some("Use Translation Service:"));
        service_label.set_halign(gtk::Align::Start);
        
        let service_selector = ComboBoxText::new();
        service_selector.set_hexpand(true);
        
        // Add all available services
        service_selector.append(Some("GoogleBeta"), "Google Translate (Beta)");
        service_selector.append(Some("GoogleOfficial"), "Google Translate (Official)");
        service_selector.append(Some("LibreTranslate"), "LibreTranslate");
        service_selector.append(Some("Bing"), "Bing Translator");
        service_selector.append(Some("DeepL"), "DeepL");
        
        selector_box.append(&service_label);
        selector_box.append(&service_selector);
        
        service_box.append(&selector_box);
        
        service_frame.set_child(Some(&service_box));
        
        // API configuration section
        let config_frame = Frame::new(Some("API Configuration"));
        
        let config_scroll = ScrolledWindow::new();
        config_scroll.set_vexpand(true);
        config_scroll.set_min_content_height(200);
        
        let config_box = GtkBox::new(Orientation::Vertical, 15);
        config_box.set_margin_start(10);
        config_box.set_margin_end(10);
        config_box.set_margin_top(10);
        config_box.set_margin_bottom(10);
        
        // Create API configuration entries for each service
        let api_entries = Rc::new(RefCell::new(Vec::new()));
        
        // Google Beta
        let google_beta_box = Self::create_api_config_section(
            "Google Translate (Beta)",
            "No API key required for testing. Limited usage.",
            None
        );
        config_box.append(&google_beta_box.0);
        api_entries.borrow_mut().push((TranslationService::GoogleBeta, google_beta_box.1, google_beta_box.2));
        
        // Google Official
        let google_official_box = Self::create_api_config_section(
            "Google Translate (Official)",
            "Requires Google Cloud API key.",
            Some("API Key:")
        );
        config_box.append(&google_official_box.0);
        api_entries.borrow_mut().push((TranslationService::GoogleOfficial, google_official_box.1, google_official_box.2));
        
        // LibreTranslate
        let libre_box = Self::create_api_config_section(
            "LibreTranslate",
            "Open-source translation API. Free to use with your own instance.",
            Some("API Key:")
        );
        config_box.append(&libre_box.0);
        api_entries.borrow_mut().push((TranslationService::LibreTranslate, libre_box.1, libre_box.2));
        
        // Bing
        let bing_box = Self::create_api_config_section(
            "Bing Translator",
            "Requires Microsoft Azure Cognitive Services API key.",
            Some("API Key:")
        );
        config_box.append(&bing_box.0);
        api_entries.borrow_mut().push((TranslationService::Bing, bing_box.1, bing_box.2));
        
        // DeepL
        let deepl_box = Self::create_api_config_section(
            "DeepL",
            "Professional translation service. Requires paid API key.",
            Some("API Key:")
        );
        config_box.append(&deepl_box.0);
        api_entries.borrow_mut().push((TranslationService::DeepL, deepl_box.1, deepl_box.2));
        
        config_scroll.set_child(Some(&config_box));
        config_frame.set_child(Some(&config_scroll));
        
        // Add sections to API page
        api_page.append(&service_frame);
        api_page.append(&config_frame);
        
        
        // ---- Add tabs to notebook ----
        notebook.append_page(&general_page, Some(&Label::new(Some("General"))));
        notebook.append_page(&api_page, Some(&Label::new(Some("Translation APIs"))));
        
        // Add notebook to dialog
        content_area.append(&notebook);
        
        // Load current settings
        let current_settings = settings.borrow();
        
        // Set appearance widgets
        dark_mode_switch.set_active(current_settings.dark_mode);
        startup_switch.set_active(current_settings.startup_minimized);
        
        // Set default language widgets
        default_source_lang.set_active_id(Some(&current_settings.default_source_lang));
        default_target_lang.set_active_id(Some(&current_settings.default_target_lang));
        
        // Set active service
        let _ = match current_settings.active_service {
            TranslationService::GoogleBeta => service_selector.set_active_id(Some("GoogleBeta")),
            TranslationService::GoogleOfficial => service_selector.set_active_id(Some("GoogleOfficial")),
            TranslationService::LibreTranslate => service_selector.set_active_id(Some("LibreTranslate")),
            TranslationService::Bing => service_selector.set_active_id(Some("Bing")),
            TranslationService::DeepL => service_selector.set_active_id(Some("DeepL")),
        };
        
        // Set API configuration entries
        for (service, key_entry, endpoint_entry) in api_entries.borrow().iter() {
            let service_name = match service {
                TranslationService::GoogleBeta => "GoogleBeta",
                TranslationService::GoogleOfficial => "GoogleOfficial",
                TranslationService::LibreTranslate => "LibreTranslate",
                TranslationService::Bing => "Bing",
                TranslationService::DeepL => "DeepL",
            };
            
            if let Some(config) = current_settings.service_configs.get(service_name) {
                if let Some(api_key) = &config.api_key {
                    key_entry.set_text(api_key);
                }
                
                if let Some(endpoint) = &config.endpoint {
                    endpoint_entry.set_text(endpoint);
                }
            }
        }
        
        // Create the settings dialog
        let settings_dialog = SettingsDialog {
            dialog,
            settings: settings.clone(),
            api_entries,
            dark_mode_switch,
            default_source_lang,
            default_target_lang,
        };
        
        // Connect response signal
        let dialog_clone = settings_dialog.dialog.clone();
        let settings_dialog_clone = settings_dialog.clone();
        settings_dialog.dialog.connect_response(move |_, response| {
            if response == ResponseType::Ok || response == ResponseType::Apply {
                // Save settings
                settings_dialog_clone.save_settings();
            }
            
            if response == ResponseType::Ok || response == ResponseType::Cancel {
                dialog_clone.hide();
            }
        });
        
        settings_dialog
    }
    
    pub fn show(&self) {
        self.dialog.show();
    }
    
    pub fn connect_response<F: Fn(ResponseType) + 'static>(&self, callback: F) {
        self.dialog.connect_response(move |_, response| {
            callback(response);
        });
    }
    
    pub fn clone(&self) -> Self {
        SettingsDialog {
            dialog: self.dialog.clone(),
            settings: self.settings.clone(),
            api_entries: self.api_entries.clone(),
            dark_mode_switch: self.dark_mode_switch.clone(),
            default_source_lang: self.default_source_lang.clone(),
            default_target_lang: self.default_target_lang.clone(),
        }
    }
    
    fn save_settings(&self) {
        let mut settings = self.settings.borrow_mut();
        
        // Save appearance settings
        let dark_mode_changed = settings.dark_mode != self.dark_mode_switch.is_active();
        settings.dark_mode = self.dark_mode_switch.is_active();
        
        // Save default languages
        if let Some(source_lang) = self.default_source_lang.active_id() {
            settings.default_source_lang = source_lang.to_string();
        }
        
        if let Some(target_lang) = self.default_target_lang.active_id() {
            settings.default_target_lang = target_lang.to_string();
        }
        
        // Save API configurations
        for (service, key_entry, endpoint_entry) in self.api_entries.borrow().iter() {
            let service_name = match service {
                TranslationService::GoogleBeta => "GoogleBeta",
                TranslationService::GoogleOfficial => "GoogleOfficial",
                TranslationService::LibreTranslate => "LibreTranslate",
                TranslationService::Bing => "Bing",
                TranslationService::DeepL => "DeepL",
            };
            
            let api_key = key_entry.text().to_string();
            let endpoint = endpoint_entry.text().to_string();
            
            let config = ServiceConfig {
                api_key: if api_key.is_empty() { None } else { Some(api_key) },
                endpoint: if endpoint.is_empty() { None } else { Some(endpoint) },
                timeout_seconds: Some(5),
            };
            
            settings.service_configs.insert(service_name.to_string(), config);
        }
        
        // Save settings to file
        settings.save();
        
        // Apply theme immediately if dark mode changed
        if dark_mode_changed {
            apply_theme(settings.dark_mode);
        }
    }
    
    fn create_api_config_section(
        title: &str,
        description: &str,
        key_label: Option<&str>,
    ) -> (GtkBox, Entry, Entry) {
        let section = GtkBox::new(Orientation::Vertical, 5);
        section.set_margin_bottom(15);
        
        // Title
        let title_label = Label::new(Some(title));
        title_label.add_css_class("title-4");
        title_label.set_halign(gtk::Align::Start);
        title_label.set_margin_bottom(5);
        
        // Description with proper wrapping
        let desc_label = Label::new(Some(description));
        desc_label.set_halign(gtk::Align::Start);
        desc_label.add_css_class("dim-label");
        desc_label.set_wrap(true);
        desc_label.set_max_width_chars(40);
        desc_label.set_margin_bottom(10);
        
        section.append(&title_label);
        section.append(&desc_label);
        
        // API key input if needed
        let key_entry = Entry::new();
        
        if let Some(label_text) = key_label {
            let key_box = GtkBox::new(Orientation::Horizontal, 10);
            let key_label = Label::new(Some(label_text));
            key_label.set_halign(gtk::Align::Start);
            key_label.set_width_chars(10);
            
            key_entry.set_hexpand(true);
            key_entry.set_input_purpose(gtk::InputPurpose::Password);
            
            key_box.append(&key_label);
            key_box.append(&key_entry);
            key_box.set_margin_bottom(5);
            
            section.append(&key_box);
        }
        
        // Endpoint input (for self-hosted services)
        let endpoint_box = GtkBox::new(Orientation::Horizontal, 10);
        let endpoint_label = Label::new(Some("Endpoint:"));
        endpoint_label.set_halign(gtk::Align::Start);
        endpoint_label.set_width_chars(10);
        
        let endpoint_entry = Entry::new();
        endpoint_entry.set_hexpand(true);
        endpoint_entry.set_placeholder_text(Some("Leave empty for default"));
        
        endpoint_box.append(&endpoint_label);
        endpoint_box.append(&endpoint_entry);
        endpoint_box.set_margin_bottom(10);
        
        section.append(&endpoint_box);
        
        // Add separator
        let separator = Separator::new(Orientation::Horizontal);
        separator.set_margin_top(5);
        section.append(&separator);
        
        (section, key_entry, endpoint_entry)
    }
}