mod languages;
mod selection;
mod hotkey;
mod database;
mod translation;
mod settings;
mod history_ui;
mod settings_ui;
mod window_manager; 
mod ui_helpers;  

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, ComboBoxText, ScrolledWindow};
use gtk::{Label, Orientation, TextView, TextBuffer, HeaderBar, Statusbar, Frame};
use gtk::{MenuButton, PopoverMenu, gio, Notebook, Popover, ResponseType};
use gtk::glib;
use tokio::runtime::Runtime;
use languages::LANGUAGES;
use selection::get_selected_text;
use hotkey::start_global_hotkey_service;
use database::Database;
use translation::TranslationManager;
use settings::Settings;
use history_ui::HistoryPanel;
use settings_ui::SettingsDialog;
use window_manager::WindowManager;
use ui_helpers::*;

// No command import needed here
use std::fs;
use std::env;
use std::time::{SystemTime, Duration};
use std::rc::Rc;
use std::cell::RefCell;
use gtk::glib::source::Continue;
use gtk::Inhibit;

// The exact window title to match in both files
const WINDOW_TITLE: &str = "SwiftLingo";
const APP_ID: &str = "org.swiftlingo.app";

/// Structure to hold all the shared state and reduce cloning
struct AppState {
    translation_manager: TranslationManager,
    db: Database,
    input_buffer: TextBuffer,
    output_buffer: TextBuffer,
    source_lang: ComboBoxText,
    target_lang: ComboBoxText,
    status_bar: Statusbar,
    history_panel: HistoryPanel,
    settings: Settings,
    window_manager: WindowManager,
}

/// Asynchronously translates text using the selected translation service
async fn translate_text(
    text: String, 
    source_lang: String, 
    target_lang: String,
    translation_manager: &TranslationManager,
) -> String {
    if text.is_empty() {
        return String::from("Please enter some text to translate");
    }
    
    // Use the translation manager to translate the text
    match translation_manager.translate(&text, &source_lang, &target_lang).await {
        Ok(translation) => translation,
        Err(error) => {
            // Try with fallback if the primary service fails
            match translation_manager.translate_with_fallback(&text, &source_lang, &target_lang).await {
                Ok(translation) => translation,
                Err(_) => format!("Translation error: {}", error),
            }
        }
    }
}

/// Helper function to perform translation and update UI accordingly
fn perform_translation(
    text: String,
    app_state: &Rc<RefCell<AppState>>,
) {
    let state = app_state.borrow();
    
    if text.is_empty() {
        state.status_bar.push(0, "No text to translate");
        return;
    }
    
    // Get languages
    let source_lang = state.source_lang.active_id()
        .unwrap_or_else(|| gtk::glib::GString::from("auto"))
        .to_string();
    let target_lang = state.target_lang.active_id()
        .unwrap_or_else(|| gtk::glib::GString::from("es"))
        .to_string();
    
    // Show "Translating..." in the output field
    state.output_buffer.set_text("Translating...");
    state.status_bar.push(0, "Translating...");
    
    // Drop the borrow before async operation
    drop(state);
    
    // Clone app_state for the async task
    let app_state_clone = app_state.clone();
    let text_to_translate = text.clone();
    let source_lang_clone = source_lang.clone();
    let target_lang_clone = target_lang.clone();
    
    // Spawn the translation task
    spawn_local_task(move || async move {
        let translation = {
            let state = app_state_clone.borrow();
            translate_text(
                text_to_translate.clone(), 
                source_lang_clone.clone(), 
                target_lang_clone.clone(),
                &state.translation_manager
            ).await
        };
        
        // Now update UI
        let state = app_state_clone.borrow();
        state.output_buffer.set_text(&translation);
        state.status_bar.push(0, "Translation complete");
        
        // Add to database
        let _ = state.db.add_translation(
            &text_to_translate,
            &source_lang_clone,
            &translation,
            &target_lang_clone
        );
        
        // Update history panel
        state.history_panel.on_translation_added();
    });
}

/// Builds the GTK user interface, sets up translation logic, and attaches the hotkey receiver.
fn build_ui(app: &Application) {
    // Create window manager
    let window_manager = WindowManager::new();
    
    // Try to install tools if needed
    window_manager.install_tools_if_needed();
    
    // Load settings
    let settings = Settings::load();
    
    // Initialize database
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error initializing database: {}", e);
            // Continue with a dummy database
            Database::new().unwrap_or_else(|_| panic!("Failed to create database"))
        }
    };
    
    // Initialize translation manager
    let mut translation_manager = TranslationManager::new();
    
    // Set active service from settings
    translation_manager.set_active_service(settings.active_service.clone());
    
    // Create the main window with a header bar
    let window = ApplicationWindow::builder()
        .application(app)
        .title(WINDOW_TITLE)
        .default_width(settings.window_width)
        .default_height(settings.window_height)
        .build();
        
    // Set up window properties for better desktop integration
    window_manager.setup_window(&window);
    
    // Set application ID for better window manager integration
    app.set_application_id(Some(APP_ID));
    
    // Set window position if it was saved
    if let (Some(_x), Some(_y)) = (settings.window_x, settings.window_y) {
        window.set_default_size(settings.window_width, settings.window_height);
    }
    
    // Apply theme based on settings
    apply_theme(settings.dark_mode);
    
    // Create a header bar with title and menu
    let header = HeaderBar::new();
    
    // Title
    let title_label = Label::new(Some(WINDOW_TITLE));
    title_label.add_css_class("title");
    header.set_title_widget(Some(&title_label));
    header.set_show_title_buttons(true);
    
    // Menu button
    let menu_button = MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    
    // Create menu model
    let menu_model = gio::Menu::new();
    
    // Settings menu item
    let settings_menu = gio::MenuItem::new(Some("Settings"), None);
    settings_menu.set_detailed_action("app.settings");
    menu_model.append_item(&settings_menu);
    
    // About menu item
    let about_menu = gio::MenuItem::new(Some("About"), None);
    about_menu.set_detailed_action("app.about");
    menu_model.append_item(&about_menu);
    
    // Create popover menu from model
    let popover = PopoverMenu::from_model(Some(&menu_model));
    menu_button.set_popover(Some(&popover));
    
    header.pack_end(&menu_button);
    
    window.set_titlebar(Some(&header));
    
    // Create actions for the menu
    let settings_action = gio::SimpleAction::new("settings", None);
    let about_action = gio::SimpleAction::new("about", None);
    
    // About dialog action
    let window_clone = window.clone();
    about_action.connect_activate(move |_, _| {
        let about_dialog = gtk::AboutDialog::new();
        about_dialog.set_transient_for(Some(&window_clone));
        about_dialog.set_modal(true);
        about_dialog.set_program_name(Some("Instant Translator"));
        about_dialog.set_version(Some(env!("CARGO_PKG_VERSION")));
        about_dialog.set_comments(Some("A fast and convenient translator app"));
        about_dialog.set_authors(&["Anassk"]);
        about_dialog.set_license_type(gtk::License::Gpl30);
        about_dialog.present();
    });
    app.add_action(&about_action);
    
    // Main container with notebook/tabs
    let main_notebook = Notebook::new();
    
    // ---- Main Translation Tab ----
    let main_tab_content = GtkBox::new(Orientation::Vertical, 0);
    
    // Info bar at the top
    let info_box = GtkBox::new(Orientation::Horizontal, 10);
    info_box.set_margin_start(16);
    info_box.set_margin_end(16);
    info_box.set_margin_top(16);
    info_box.set_margin_bottom(8);
    
    let hotkey_info = Label::new(Some("Select text anywhere and press Ctrl+Alt+T"));
    hotkey_info.add_css_class("info-label");
    info_box.append(&hotkey_info);
    
    main_tab_content.append(&info_box);
    
    // Language selection area
    let lang_frame = Frame::new(Some("Language Options"));
    lang_frame.set_margin_start(16);
    lang_frame.set_margin_end(16);
    lang_frame.set_margin_bottom(16);
    
    let lang_box = GtkBox::new(Orientation::Horizontal, 16);
    lang_box.set_margin_start(16);
    lang_box.set_margin_end(16);
    lang_box.set_margin_top(16);
    lang_box.set_margin_bottom(16);
    
    let source_lang_label = Label::new(Some("From:"));
    let source_lang = ComboBoxText::new();
    source_lang.add_css_class("language-selector");
    source_lang.append(Some("auto"), "Detect language");
    for (code, name) in LANGUAGES.iter() {
        if *code != "auto" {
            source_lang.append(Some(code), name);
        }
    }
    
    // Set default source language from settings
    source_lang.set_active_id(Some(&settings.default_source_lang));
    
    let target_lang_label = Label::new(Some("To:"));
    let target_lang = ComboBoxText::new();
    target_lang.add_css_class("language-selector");
    for (code, name) in LANGUAGES.iter() {
        if *code != "auto" {
            target_lang.append(Some(code), name);
        }
    }
    
    // Set default target language from settings
    target_lang.set_active_id(Some(&settings.default_target_lang));
    
    lang_box.append(&source_lang_label);
    lang_box.append(&source_lang);
    lang_box.append(&target_lang_label);
    lang_box.append(&target_lang);
    
    lang_frame.set_child(Some(&lang_box));
    main_tab_content.append(&lang_frame);
    
    // Changed to horizontal layout for side-by-side text areas
    let content_box = GtkBox::new(Orientation::Horizontal, 16);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);
    content_box.set_margin_bottom(16);
    content_box.set_vexpand(true);
    content_box.set_hexpand(true);
    
    // Left side: Input text area with frame
    let left_box = GtkBox::new(Orientation::Vertical, 8);
    left_box.set_hexpand(true);
    left_box.set_vexpand(true);
    
    let input_frame = Frame::new(Some("Text to Translate"));
    input_frame.set_vexpand(true);
    input_frame.set_hexpand(true);
    
    let input_scroll = ScrolledWindow::new();
    input_scroll.set_vexpand(true);
    input_scroll.set_hexpand(true);
    
    let input_buffer = TextBuffer::new(None);
    let input_text = TextView::with_buffer(&input_buffer);
    input_text.set_wrap_mode(gtk::WrapMode::Word);
    input_text.add_css_class("text-area");
    input_text.set_vexpand(true);
    input_text.set_hexpand(true);
    input_scroll.set_child(Some(&input_text));
    input_frame.set_child(Some(&input_scroll));
    
    left_box.append(&input_frame);
    
    // Button to get selection in the left box
    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_halign(gtk::Align::End);
    button_box.set_margin_top(8);
    
    let get_selection_button = create_button("Get Selection", true, false);
    let translate_button = create_button("Translate", true, false);
    
    button_box.append(&get_selection_button);
    button_box.append(&translate_button);
    
    left_box.append(&button_box);
    
    // Right side: Output text area with frame
    let right_box = GtkBox::new(Orientation::Vertical, 8);
    right_box.set_hexpand(true);
    right_box.set_vexpand(true);
    
    let output_frame = Frame::new(Some("Translation"));
    output_frame.set_vexpand(true);
    output_frame.set_hexpand(true);
    
    let output_scroll = ScrolledWindow::new();
    output_scroll.set_vexpand(true);
    output_scroll.set_hexpand(true);
    
    let output_buffer = TextBuffer::new(None);
    let output_text = TextView::with_buffer(&output_buffer);
    output_text.set_wrap_mode(gtk::WrapMode::Word);
    output_text.set_editable(false);
    output_text.add_css_class("text-area");
    output_text.add_css_class("output-area");
    output_text.set_vexpand(true);
    output_text.set_hexpand(true);
    output_scroll.set_child(Some(&output_text));
    output_frame.set_child(Some(&output_scroll));
    
    right_box.append(&output_frame);
    
    // Add save and copy button for the translation
    let action_box = GtkBox::new(Orientation::Horizontal, 8);
    action_box.set_halign(gtk::Align::End);
    action_box.set_margin_top(8);
    
    let save_button = Button::with_label("Save to List");
    let copy_button = create_button("Copy Translation", true, false);
    
    action_box.append(&save_button);
    action_box.append(&copy_button);
    
    right_box.append(&action_box);
    
    // Add left and right boxes to the content box
    content_box.append(&left_box);
    content_box.append(&right_box);
    
    main_tab_content.append(&content_box);
    
    // Status bar at the bottom
    let status_bar = Statusbar::new();
    status_bar.push(0, "Ready");
    main_tab_content.append(&status_bar);
    
    // Create history panel with input/output buffer references
    let history_panel = HistoryPanel::new(
        Rc::new(RefCell::new(db.clone())),
        Rc::new(RefCell::new(input_buffer.clone())),
        Rc::new(RefCell::new(output_buffer.clone())),
        Rc::new(RefCell::new(source_lang.clone())),
        Rc::new(RefCell::new(target_lang.clone())),
    );
    
    // Create AppState to hold shared widgets and references
    let app_state = Rc::new(RefCell::new(AppState {
        translation_manager,
        db,
        input_buffer: input_buffer.clone(),
        output_buffer: output_buffer.clone(),
        source_lang: source_lang.clone(),
        target_lang: target_lang.clone(),
        status_bar: status_bar.clone(),
        history_panel: history_panel.clone(),
        settings,
        window_manager,
    }));
    
    // Add tabs to notebook
    main_notebook.append_page(&main_tab_content, Some(&Label::new(Some("Translate"))));
    main_notebook.append_page(history_panel.get_widget(), Some(&Label::new(Some("History"))));
    
    window.set_child(Some(&main_notebook));
    
    // Now that history_panel_rc is defined, set up the settings dialog action
    let window_clone = window.clone();
    let app_state_clone = app_state.clone();
    settings_action.connect_activate(move |_, _| {
        let settings_rc = Rc::new(RefCell::new(app_state_clone.borrow().settings.clone()));
        let settings_dialog = SettingsDialog::new(&window_clone, settings_rc.clone());
        
        // Connect to dialog response to update history when settings are applied
        let history_panel_clone = app_state_clone.borrow().history_panel.clone();
        settings_dialog.connect_response(move |response| {
            if response == ResponseType::Ok || response == ResponseType::Apply {
                // Update history panel to reflect any changes in translation settings
                history_panel_clone.update_view();
            }
        });
        
        settings_dialog.show();
    });
    app.add_action(&settings_action);
    
    // Add keyboard shortcut for getting selection (Ctrl+Alt+T within the app)
    let app_state_clone = app_state.clone();
    let window_clone = window.clone();
    
    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _keycode, state| {
        // Check for Ctrl+Alt+T
        if key == gtk::gdk::Key::t && 
           state.contains(gtk::gdk::ModifierType::CONTROL_MASK) && 
           state.contains(gtk::gdk::ModifierType::ALT_MASK) {
            println!("Ctrl+Alt+T pressed within app");
            
            // Update status
            app_state_clone.borrow().status_bar.push(0, "Getting selection...");
            
            let selection = get_selected_text();
            if !selection.is_empty() {
                // Set the text in the input field
                app_state_clone.borrow().input_buffer.set_text(&selection);
                
                // Make sure window comes to front
                app_state_clone.borrow().window_manager.focus_window(&window_clone);
                
                // Trigger translation
                perform_translation(selection, &app_state_clone);
            } else {
                app_state_clone.borrow().status_bar.push(0, "No text selected");
            }
            return Inhibit(true);
        }
        Inhibit(false)
    });
    window.add_controller(key_controller);
    
    // Connect translate button signal
    let app_state_clone = app_state.clone();
    translate_button.connect_clicked(move |_| {
        let text = {
            let state = app_state_clone.borrow();
            state.input_buffer.text(
                &state.input_buffer.start_iter(),
                &state.input_buffer.end_iter(),
                false
            ).to_string()
        };
        
        perform_translation(text, &app_state_clone);
    });
    
    // Connect get selection button
    let app_state_clone = app_state.clone();
    get_selection_button.connect_clicked(move |_| {
        app_state_clone.borrow().status_bar.push(0, "Getting selection...");
        let selection = get_selected_text();
        println!("Got selection: {}", selection);
        
        if !selection.is_empty() && selection != "Failed to get X11 selection" && selection != "Failed to get Wayland selection" {
            app_state_clone.borrow().input_buffer.set_text(&selection);
            perform_translation(selection, &app_state_clone);
        } else {
            app_state_clone.borrow().status_bar.push(0, "No text selected");
        }
    });
    
    // Connect save button
    let app_state_clone = app_state.clone();
    save_button.connect_clicked(move |button| {
        let (input_text, output_text, source_lang, target_lang, status_bar, db) = {
            let state = app_state_clone.borrow();
            let input_text = state.input_buffer.text(
                &state.input_buffer.start_iter(),
                &state.input_buffer.end_iter(),
                false
            ).to_string();
            
            let output_text = state.output_buffer.text(
                &state.output_buffer.start_iter(),
                &state.output_buffer.end_iter(),
                false
            ).to_string();
            
            let source_lang = state.source_lang.active_id()
                .unwrap_or_else(|| gtk::glib::GString::from("auto"))
                .to_string();
            
            let target_lang = state.target_lang.active_id()
                .unwrap_or_else(|| gtk::glib::GString::from("es"))
                .to_string();
                
            (
                input_text,
                output_text,
                source_lang,
                target_lang,
                state.status_bar.clone(),
                state.db.clone()
            )
        };
        
        if input_text.is_empty() || output_text.is_empty() || output_text == "Translating..." {
            status_bar.push(0, "No translation to save");
            return;
        }
        
        // First add to database history
        let translation_id = match db.add_translation(
            &input_text, &source_lang, &output_text, &target_lang
        ) {
            Ok(id) => id,
            Err(_) => {
                status_bar.push(0, "Failed to save translation");
                return;
            }
        };
        
        // Now show dialog to select a list
        let popover = Popover::new();
        popover.set_parent(button);
        
        let dialog_box = GtkBox::new(Orientation::Vertical, 10);
        dialog_box.set_margin_start(10);
        dialog_box.set_margin_end(10);
        dialog_box.set_margin_top(10);
        dialog_box.set_margin_bottom(10);
        
        let title = Label::new(Some("Save to List"));
        title.add_css_class("title-4");
        
        let list_combo = ComboBoxText::new();
        list_combo.append(None, "Select a list");
        list_combo.append(Some("new"), "Create New List...");
        
        // Add lists from the database
        if let Ok(lists) = db.get_lists() {
            for list in lists {
                list_combo.append(Some(&list.id.to_string()), &list.name);
            }
        }
        
        list_combo.set_active(Some(0));
        
        // New list entry (initially hidden)
        let new_list_box = GtkBox::new(Orientation::Horizontal, 5);
        new_list_box.set_visible(false);
        
        let new_list_entry = gtk::Entry::new();
        new_list_entry.set_placeholder_text(Some("Enter list name"));
        new_list_entry.set_hexpand(true);
        
        new_list_box.append(&new_list_entry);
        
        // Buttons
        let button_box = GtkBox::new(Orientation::Horizontal, 5);
        button_box.set_halign(gtk::Align::End);
        
        let cancel_button = Button::with_label("Cancel");
        let save_list_button = Button::with_label("Save");
        save_list_button.add_css_class("suggested-action");
        
        button_box.append(&cancel_button);
        button_box.append(&save_list_button);
        
        dialog_box.append(&title);
        dialog_box.append(&list_combo);
        dialog_box.append(&new_list_box);
        dialog_box.append(&button_box);
        
        popover.set_child(Some(&dialog_box));
        
        // Show new list entry when "Create New List" is selected
        let new_list_box_clone = new_list_box.clone();
        list_combo.connect_changed(move |combo| {
            if let Some(id) = combo.active_id() {
                if id == "new" {
                    new_list_box_clone.set_visible(true);
                } else {
                    new_list_box_clone.set_visible(false);
                }
            }
        });
        
        // Connect cancel button
        let popover_ref = popover.clone();
        cancel_button.connect_clicked(move |_| {
            popover_ref.popdown();
        });
        
        // Connect save button
        let popover_ref = popover.clone();
        let status_bar_clone = status_bar.clone();
        
        save_list_button.connect_clicked(move |_| {
            if let Some(id) = list_combo.active_id() {
                if id == "new" {
                    // Create new list
                    let list_name = new_list_entry.text().to_string();
                    if list_name.is_empty() {
                        status_bar_clone.push(0, "Please enter a list name");
                        return;
                    }
                    
                    match db.create_list(&list_name) {
                        Ok(list_id) => {
                            let _ = db.add_to_list(list_id, translation_id);
                            status_bar_clone.push(0, "Translation saved to new list");
                        },
                        Err(_) => {
                            status_bar_clone.push(0, "Failed to create list");
                        }
                    }
                } else {
                    // Add to existing list
                    if let Ok(list_id) = id.to_string().parse::<i64>() {
                        let _ = db.add_to_list(list_id, translation_id);
                        status_bar_clone.push(0, "Translation saved to list");
                    }
                }
            } else {
                status_bar_clone.push(0, "Please select a list");
                return;
            }
            
            popover_ref.popdown();
        });
        
        popover.popup();
    });
    
    // Connect copy button
    let app_state_clone = app_state.clone();
    copy_button.connect_clicked(move |_| {
        let (text, status_bar) = {
            let state = app_state_clone.borrow();
            let text = state.output_buffer.text(
                &state.output_buffer.start_iter(),
                &state.output_buffer.end_iter(),
                false
            ).to_string();
            
            (text, state.status_bar.clone())
        };
        
        if text.is_empty() || text == "Translating..." {
            status_bar.push(0, "No translation to copy");
            return;
        }
        
        // Use our native clipboard implementation
        if selection::set_clipboard_text(&text) {
            status_bar.push(0, "Translation copied to clipboard");
        } else {
            status_bar.push(0, "Failed to copy to clipboard");
        }
    });

    // Set up monitoring for the global hotkey trigger file
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let selection_path = format!("{}/.config/translator-app/selection.txt", home_dir);
    let focus_path = format!("{}/.config/translator-app/focus-window", home_dir);
    
    // Additional clones for the global hotkey handler
    let app_state_clone = app_state.clone();
    let window_clone = window.clone();
    
    // Store the last modification time to avoid processing the same event multiple times
    let mut last_mod_time = SystemTime::now();
    let mut last_focus_time = SystemTime::now();
    
    // Add a timeout to check for the selection and focus files
    glib::source::timeout_add_local(Duration::from_millis(100), move || {
        // First check the focus-window file
        if let Ok(metadata) = fs::metadata(&focus_path) {
            if let Ok(mod_time) = metadata.modified() {
                if mod_time > last_focus_time {
                    last_focus_time = mod_time;
                    
                    // Remove the focus file
                    let _ = fs::remove_file(&focus_path);
                    
                    // Bring window to front using the WindowManager
                    app_state_clone.borrow().window_manager.focus_window(&window_clone);
                }
            }
        }
        
        // Then check for selection changes
        if let Ok(metadata) = fs::metadata(&selection_path) {
            if let Ok(mod_time) = metadata.modified() {
                if mod_time > last_mod_time {
                    last_mod_time = mod_time;
                    
                    // Read the selection from the file
                    if let Ok(selection) = fs::read_to_string(&selection_path) {
                        if !selection.is_empty() {
                            // Set the input text
                            app_state_clone.borrow().input_buffer.set_text(&selection);
                            
                            // Make sure window comes to front with focus
                            app_state_clone.borrow().window_manager.focus_window(&window_clone);
                            
                            // Trigger translation
                            perform_translation(selection, &app_state_clone);
                        } else {
                            app_state_clone.borrow().status_bar.push(0, "No text selected");
                        }
                    }
                }
            }
        }
        
        // Return Continue to keep the timeout active
        Continue(true)
    });
    
    // Handle startup minimized
    let app_state_clone = app_state.clone();
    let window_clone = window.clone();
    if app_state_clone.borrow().settings.startup_minimized {
        // Use glib timeout to allow the window to initialize first
        glib::source::timeout_add_local(Duration::from_millis(100), move || {
            app_state_clone.borrow().window_manager.minimize_window(&window_clone);
            Continue(false)
        });
    }
    
    // Create clones for the close request handler
    let app_state_clone = app_state.clone();
    let window_clone = window.clone();
    
    window.connect_close_request(move |_| {
        window_clone.hide();
        if app_state_clone.borrow().settings.startup_minimized {
            // Use glib timeout to allow the window to initialize first
            let window_clone_inner = window_clone.clone();
            let app_state_clone_inner = app_state_clone.clone();
            glib::source::timeout_add_local(Duration::from_millis(100), move || {
                app_state_clone_inner.borrow().window_manager.minimize_window(&window_clone_inner);
                Continue(false)
            });
        }
        Inhibit(false)
    });
    
    // Show everything
    window.present();
}

/// Apply theme based on dark mode setting
fn apply_theme(dark_mode: bool) {
    let css_provider = gtk::CssProvider::new();
    
    // Load the external CSS file
    if let Some(css_path) = get_css_path() {
        css_provider.load_from_path(&css_path);
        
        // Apply the CSS provider to the default display
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    // Apply dark mode if enabled
    if let Some(settings) = gtk::Settings::default() {
        settings.set_property("gtk-application-prefer-dark-theme", dark_mode);
        
        // Apply dark mode class to all windows
        for window in gtk::Window::list_toplevels() {
            let style_context = window.style_context();
            if dark_mode {
                style_context.add_class("dark-mode");
            } else {
                style_context.remove_class("dark-mode");
            }
        }
    }
}

/// Get the path to the CSS file
fn get_css_path() -> Option<String> {
    if let Ok(current_exe) = env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let css_path = exe_dir.join("style.css");
            if css_path.exists() {
                return css_path.to_str().map(String::from);
            }
            
            // Try looking in the source directory during development
            let source_css_path = exe_dir.join("src").join("style.css");
            if source_css_path.exists() {
                return source_css_path.to_str().map(String::from);
            }
        }
    }
    None
}

fn main() {
    // Create the window manager to check for window management tools
    let window_manager = WindowManager::new();
    let has_window_tools = window_manager.check_clipboard_tools();
    
    if !has_window_tools {
        eprintln!("Warning: Missing window management tools. Some window focusing features may not work properly.");
    }
    
    // Create and run the application first
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::ALLOW_REPLACEMENT | gio::ApplicationFlags::REPLACE | gio::ApplicationFlags::NON_UNIQUE)
        .build();
    
    // Create a Tokio runtime for async tasks.
    let rt = Runtime::new().expect("Unable to create Runtime");
    let _enter = rt.enter();
    
    // Start the global hotkey service
    start_global_hotkey_service();
    
    // Connect activate signal before running the application
    app.connect_startup(|_| {
        // Initialize GTK
        gtk::init().expect("Failed to initialize GTK");
    });
    
    app.connect_activate(move |app| {
        build_ui(app);
    });
    
    app.run();
}