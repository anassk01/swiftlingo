use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, ComboBoxText, Entry, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow, SearchEntry, Frame, Popover, TextBuffer,
    MessageType, ButtonsType
};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::database::{Database, Translation, TranslationList};

const LIST_OPTION_CREATE_NEW: &str = "CREATE_NEW_LIST";

/// Helper function to create and show a message dialog
fn show_message_dialog(
    parent: Option<&gtk::Window>,
    message_type: MessageType,
    buttons_type: ButtonsType,
    message: &str,
) -> gtk::MessageDialog {
    let dialog = gtk::MessageDialog::new(
        parent,
        gtk::DialogFlags::MODAL,
        message_type,
        buttons_type,
        message,
    );
    
    dialog.connect_response(|dialog, _| {
        dialog.destroy();
    });
    
    dialog.show();
    dialog
}

pub struct HistoryPanel {
    main_box: GtkBox,
    translation_list: ListBox,
    list_selector: ComboBoxText,
    lists: Rc<RefCell<Vec<TranslationList>>>,
    db: Rc<RefCell<Database>>,
    active_list_id: Rc<RefCell<Option<i64>>>,
    input_buffer: Rc<RefCell<TextBuffer>>,
    output_buffer: Rc<RefCell<TextBuffer>>,
    source_lang: Rc<RefCell<ComboBoxText>>,
    target_lang: Rc<RefCell<ComboBoxText>>,
    translation_id_map: Rc<RefCell<HashMap<u32, i64>>>,
}

impl HistoryPanel {
    pub fn new(
        db: Rc<RefCell<Database>>,
        input_buffer: Rc<RefCell<TextBuffer>>,
        output_buffer: Rc<RefCell<TextBuffer>>,
        source_lang: Rc<RefCell<ComboBoxText>>,
        target_lang: Rc<RefCell<ComboBoxText>>,
    ) -> Self {
        // Main vertical box
        let main_box = GtkBox::new(Orientation::Vertical, 10);
        main_box.set_margin_start(16);
        main_box.set_margin_end(16);
        main_box.set_margin_top(16);
        main_box.set_margin_bottom(16);

        
        // Header section
        let header_box = GtkBox::new(Orientation::Horizontal, 10);
        header_box.set_margin_bottom(10);
        
        let title = Label::new(Some("Translation History"));
        title.set_hexpand(true);
        title.set_halign(gtk::Align::Start);
        title.add_css_class("title-3");
        
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search translations..."));
        
        header_box.append(&title);
        header_box.append(&search_entry);
        
        main_box.append(&header_box);
        
        // List management section
        let list_box = GtkBox::new(Orientation::Horizontal, 10);
        list_box.set_margin_bottom(10);
        
        let list_label = Label::new(Some("Save to List:"));
        
        let list_selector = ComboBoxText::new();
        list_selector.set_margin_end(10);
        list_selector.set_hexpand(true);
        
        let refresh_button = Button::with_label("Refresh");
        let delete_list_button = Button::with_label("Delete List");
        delete_list_button.add_css_class("destructive-action");
        let export_button = Button::with_label("Export");
        
        list_box.append(&list_label);
        list_box.append(&list_selector);
        list_box.append(&refresh_button);
        list_box.append(&delete_list_button);
        list_box.append(&export_button);
        
        main_box.append(&list_box);
        
        // Translations list
        let list_frame = Frame::new(Some("Recent Translations"));
        
        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_hexpand(true);
        scroll.set_min_content_height(300);
        
        let translation_list = ListBox::new();
        translation_list.set_selection_mode(gtk::SelectionMode::Single);
        translation_list.set_show_separators(true);
        
        scroll.set_child(Some(&translation_list));
        list_frame.set_child(Some(&scroll));
        
        main_box.append(&list_frame);
        
        // Action buttons
        let action_box = GtkBox::new(Orientation::Horizontal, 10);
        action_box.set_halign(gtk::Align::End);
        action_box.set_margin_top(10);
        
        let reuse_button = Button::with_label("Reuse Selected");
        let add_to_list_button = Button::with_label("Add to List");
        let delete_button = Button::with_label("Delete");
        delete_button.add_css_class("destructive-action");
        
        action_box.append(&reuse_button);
        action_box.append(&add_to_list_button);
        action_box.append(&delete_button);
        
        main_box.append(&action_box);
        
        let lists = Rc::new(RefCell::new(Vec::new()));
        let active_list_id = Rc::new(RefCell::new(None));
        
        let history_panel = HistoryPanel {
            main_box,
            translation_list,
            list_selector,
            lists: lists.clone(),
            db: db.clone(),
            active_list_id,
            input_buffer,
            output_buffer,
            source_lang,
            target_lang,
            translation_id_map: Rc::new(RefCell::new(HashMap::new())),
        };
        
        // Load lists and refresh history
        history_panel.load_lists();
        history_panel.refresh_history();
        
        // Connect search entry
        let history_panel_ref = history_panel.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            history_panel_ref.search_translations(&query);
        });
        
        // Connect refresh button
        let history_panel_ref = history_panel.clone();
        refresh_button.connect_clicked(move |_| {
            history_panel_ref.refresh_history();
        });
        
        // Connect list selector
        let history_panel_ref = history_panel.clone();
        history_panel.list_selector.connect_changed(move |selector| {
            if let Some(list_id) = selector.active_id() {
                if list_id == LIST_OPTION_CREATE_NEW {
                    // Show dialog to create new list
                    history_panel_ref.show_create_list_dialog();
                } else if let Ok(id) = list_id.to_string().parse::<i64>() {
                    // Set active list
                    *history_panel_ref.active_list_id.borrow_mut() = Some(id);
                    history_panel_ref.load_list_translations(id);
                } else {
                    // "All Translations" selected (no list ID)
                    *history_panel_ref.active_list_id.borrow_mut() = None;
                    history_panel_ref.load_all_translations();
                }
            } else {
                // No ID means "All Translations" is selected
                *history_panel_ref.active_list_id.borrow_mut() = None;
                history_panel_ref.load_all_translations();
            }
        });
        
        // Connect reuse button
        let history_panel_ref = history_panel.clone();
        reuse_button.connect_clicked(move |_| {
            history_panel_ref.reuse_selected_translation();
        });
        
        // Connect add to list button
        let history_panel_ref = history_panel.clone();
        add_to_list_button.connect_clicked(move |_| {
            history_panel_ref.add_selected_to_list();
        });
        
        // Connect delete button
        let history_panel_ref = history_panel.clone();
        delete_button.connect_clicked(move |_| {
            history_panel_ref.delete_selected_translation();
        });
        
        // Connect export button
        let history_panel_ref = history_panel.clone();
        export_button.connect_clicked(move |_| {
            history_panel_ref.export_selected_list();
        });
        
        // Connect delete list button
        let history_panel_ref = history_panel.clone();
        delete_list_button.connect_clicked(move |_| {
            history_panel_ref.delete_current_list();
        });
        
        history_panel
    }
    
    pub fn get_widget(&self) -> &GtkBox {
        &self.main_box
    }
    
    pub fn clone(&self) -> Self {
        HistoryPanel {
            main_box: self.main_box.clone(),
            translation_list: self.translation_list.clone(),
            list_selector: self.list_selector.clone(),
            lists: self.lists.clone(),
            db: self.db.clone(),
            active_list_id: self.active_list_id.clone(),
            input_buffer: self.input_buffer.clone(),
            output_buffer: self.output_buffer.clone(),
            source_lang: self.source_lang.clone(),
            target_lang: self.target_lang.clone(),
            translation_id_map: self.translation_id_map.clone(),
        }
    }
    
    fn load_lists(&self) {
        // Clear the combo box
        self.list_selector.remove_all();
        
        // Add "All Translations" option
        self.list_selector.append(None, "All Translations");
        
        // Add all lists from the database
        if let Ok(lists) = self.db.borrow().get_lists() {
            *self.lists.borrow_mut() = lists.clone();
            
            for list in lists {
                self.list_selector.append(Some(&list.id.to_string()), &list.name);
            }
        }
        
        // Add "Create New List" option
        self.list_selector.append(Some(LIST_OPTION_CREATE_NEW), "Create New List...");
        
        // Set active to "All Translations"
        self.list_selector.set_active(Some(0));
    }
    
    fn refresh_history(&self) {
        // Clear the list
        while let Some(child) = self.translation_list.first_child() {
            self.translation_list.remove(&child);
        }
        
        // Check if we have an active list
        if let Some(list_id) = *self.active_list_id.borrow() {
            // Load translations from the active list
            if let Ok(translations) = self.db.borrow().get_list_translations(list_id) {
                for translation in translations {
                    self.add_translation_to_list(&translation);
                }
            }
        } else {
            // No active list means we should show all translations
            self.load_all_translations();
        }
    }
    
    fn load_all_translations(&self) {
        // Clear the list first
        while let Some(child) = self.translation_list.first_child() {
            self.translation_list.remove(&child);
        }
        
        // Load all translations from database
        if let Ok(translations) = self.db.borrow().get_translations(100) {
            for translation in translations {
                self.add_translation_to_list(&translation);
            }
        }
    }
    
    fn load_list_translations(&self, list_id: i64) {
        // Clear the list
        while let Some(child) = self.translation_list.first_child() {
            self.translation_list.remove(&child);
        }
        
        // Load translations from database
        if let Ok(translations) = self.db.borrow().get_list_translations(list_id) {
            for translation in translations {
                self.add_translation_to_list(&translation);
            }
        }
    }
    
    fn search_translations(&self, query: &str) {
        // Clear the list
        while let Some(child) = self.translation_list.first_child() {
            self.translation_list.remove(&child);
        }
        
        if query.is_empty() {
            // If query is empty, refresh normal history
            if let Some(list_id) = *self.active_list_id.borrow() {
                self.load_list_translations(list_id);
            } else {
                self.refresh_history();
            }
            return;
        }
        
        // Search translations in database
        if let Ok(translations) = self.db.borrow().search_translations(query) {
            for translation in translations {
                self.add_translation_to_list(&translation);
            }
        }
    }
    
    fn add_translation_to_list(&self, translation: &Translation) {
        // Format timestamp nicely
        let dt = chrono::DateTime::parse_from_rfc3339(&translation.timestamp);
        let formatted_date = match dt {
            Ok(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
            Err(_) => translation.timestamp.clone(),
        };
        
        // Create a row for the translation
        let row = ListBoxRow::new();
        row.set_selectable(true);
        
        // Store the translation ID in our HashMap
        let widget_id = row.widget_name().to_string().parse::<u32>().unwrap_or(0);
        self.translation_id_map.borrow_mut().insert(widget_id, translation.id);
                
        // Create a container for the row
        let row_box = GtkBox::new(Orientation::Vertical, 5);
        row_box.set_margin_start(8);
        row_box.set_margin_end(8);
        row_box.set_margin_top(8);
        row_box.set_margin_bottom(8);

        
        // Header section of the row
        let header_box = GtkBox::new(Orientation::Horizontal, 5);
        
        // Source and target languages
        let lang_label = Label::new(Some(&format!("{} → {}", 
                                               translation.source_lang, 
                                               translation.target_lang)));
        lang_label.add_css_class("caption-heading");
        lang_label.set_halign(gtk::Align::Start);
        
        // Timestamp
        let time_label = Label::new(Some(&formatted_date));
        time_label.add_css_class("caption");
        time_label.set_halign(gtk::Align::End);
        time_label.set_hexpand(true);
        
        header_box.append(&lang_label);
        header_box.append(&time_label);
        
        // Source and target text sections
        let source_label = Label::new(Some(&translation.source_text));
        source_label.set_halign(gtk::Align::Start);
        source_label.set_wrap(true);
        source_label.set_max_width_chars(50);
        source_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        
        let target_label = Label::new(Some(&translation.target_text));
        target_label.set_halign(gtk::Align::Start);
        target_label.add_css_class("dim-label");
        target_label.set_wrap(true);
        target_label.set_max_width_chars(50);
        target_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        
        // Add everything to the row
        row_box.append(&header_box);
        row_box.append(&source_label);
        row_box.append(&target_label);
        
        row.set_child(Some(&row_box));
        
        // Add the row to the list
        self.translation_list.append(&row);
    }
    
    pub fn show_create_list_dialog(&self) {
        // Get the parent window first
        let parent_window = gtk::Window::list_toplevels()
            .into_iter()
            .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
            .and_then(|w| w.downcast::<gtk::Window>().ok());
        
        if parent_window.is_none() {
            println!("Error: Could not find parent window for dialog");
            return;
        }
        
        // Create a standalone dialog instead of a popover
        let dialog = gtk::Dialog::new();
        dialog.set_title(Some("Create New List"));
        dialog.set_modal(true);
        dialog.set_transient_for(parent_window.as_ref());
        dialog.set_default_width(350);
        
        // Get the content area of the dialog
        let content_area = dialog.content_area();
        content_area.set_margin_start(16);
        content_area.set_margin_end(16);
        content_area.set_margin_top(16);
        content_area.set_margin_bottom(16);
        content_area.set_spacing(16);
        
        // Create the content
        let title = Label::new(Some("Create New List"));
        title.add_css_class("title-4");
        title.set_halign(gtk::Align::Start);
        
        let name_entry = Entry::new();
        name_entry.set_placeholder_text(Some("Enter list name"));
        name_entry.set_activates_default(true);
        
        // Add to content area
        content_area.append(&title);
        content_area.append(&name_entry);
        
        // Add buttons
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Create", gtk::ResponseType::Ok);
        dialog.set_default_response(gtk::ResponseType::Ok);
        
        // Connect response signal
        let self_ref = self.clone();
        
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Ok {
                let name = name_entry.text().to_string();
                if name.is_empty() {
                    // Show error for empty name
                    let parent_window = gtk::Window::list_toplevels()
                        .into_iter()
                        .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                        .and_then(|w| w.downcast::<gtk::Window>().ok());
                        
                    show_message_dialog(
                        parent_window.as_ref(),
                        gtk::MessageType::Error,
                        gtk::ButtonsType::Ok,
                        "Please enter a list name"
                    );
                    return;
                }
                
                // Check if list name already exists
                match self_ref.db.borrow().list_name_exists(&name) {
                    Ok(exists) => {
                        if exists {
                            // Show error for duplicate name
                            let parent_window = gtk::Window::list_toplevels()
                                .into_iter()
                                .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                                .and_then(|w| w.downcast::<gtk::Window>().ok());
                                
                            show_message_dialog(
                                parent_window.as_ref(),
                                gtk::MessageType::Error,
                                gtk::ButtonsType::Ok,
                                &format!("A list named \"{}\" already exists. Please choose a different name.", name)
                            );
                            return;
                        } else {
                            // Create the list
                            if let Ok(list_id) = self_ref.db.borrow().create_list(&name) {
                                // Reload lists
                                self_ref.load_lists();
                                
                                // Select the new list
                                self_ref.list_selector.set_active_id(Some(&list_id.to_string()));
                            }
                        }
                    },
                    Err(e) => {
                        // Show error for database error
                        let parent_window = gtk::Window::list_toplevels()
                            .into_iter()
                            .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                            .and_then(|w| w.downcast::<gtk::Window>().ok());
                            
                        show_message_dialog(
                            parent_window.as_ref(),
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Ok,
                            &format!("Database error: {}", e)
                        );
                        return;
                    }
                }
            }
            
            dialog.destroy();
        });
        
        // Show the dialog
        dialog.show();
    }
    
    fn reuse_selected_translation(&self) {
        if let Some(row) = self.translation_list.selected_row() {
            let widget_id = row.widget_name().to_string().parse::<u32>().unwrap_or(0);
            if let Some(translation_id) = self.translation_id_map.borrow().get(&widget_id) {
                if let Ok(translations) = self.db.borrow().get_translations(100) {
                    if let Some(translation) = translations.iter().find(|t| t.id == *translation_id) {
                        self.input_buffer.borrow().set_text(&translation.source_text);
                        self.output_buffer.borrow().set_text(&translation.target_text);
                        
                        // Set the language combo boxes
                        self.source_lang.borrow().set_active_id(Some(&translation.source_lang));
                        self.target_lang.borrow().set_active_id(Some(&translation.target_lang));
                    }
                }
            }
        }
    }
    
    fn add_selected_to_list(&self) {
        if let Some(row) = self.translation_list.selected_row() {
            let widget_id = row.widget_name().to_string().parse::<u32>().unwrap_or(0);
            if let Some(translation_id) = self.translation_id_map.borrow().get(&widget_id) {
                let translation_id_actual = *translation_id;
                // Create a popover for list selection
                let popover = Popover::new();
                popover.set_position(gtk::PositionType::Bottom);
                popover.set_parent(&self.translation_list);
                
                // Create the dialog content
                let dialog_box = GtkBox::new(Orientation::Vertical, 10);
                dialog_box.set_margin_start(10);
                dialog_box.set_margin_end(10);
                dialog_box.set_margin_top(10);
                dialog_box.set_margin_bottom(10);
                
                let title = Label::new(Some("Add to List"));
                title.add_css_class("title-4");
                
                let list_combo = ComboBoxText::new();
                
                // Add lists from the database
                if let Ok(lists) = self.db.borrow().get_lists() {
                    for list in lists {
                        list_combo.append(Some(&list.id.to_string()), &list.name);
                    }
                }
                
                let button_box = GtkBox::new(Orientation::Horizontal, 5);
                button_box.set_halign(gtk::Align::End);
                
                let cancel_button = Button::with_label("Cancel");
                let add_button = Button::with_label("Add");
                add_button.add_css_class("suggested-action");
                
                button_box.append(&cancel_button);
                button_box.append(&add_button);
                
                dialog_box.append(&title);
                dialog_box.append(&list_combo);
                dialog_box.append(&button_box);
                
                popover.set_child(Some(&dialog_box));
                
                // Connect cancel button
                let popover_ref = popover.clone();
                cancel_button.connect_clicked(move |_| {
                    popover_ref.popdown();
                });
                
                // Connect add button
                let self_ref = self.clone();
                let popover_ref = popover.clone();
                let list_combo_ref = list_combo.clone();
                let translation_id_val = translation_id_actual; 
                add_button.connect_clicked(move |_| {
                    if let Some(list_id_str) = list_combo_ref.active_id() {
                        if let Ok(list_id) = list_id_str.to_string().parse::<i64>() {
                            // Add the translation to the list
                            let _ = self_ref.db.borrow().add_to_list(list_id, translation_id_val);
                            
                            // If we're currently viewing this list, refresh it
                            if let Some(active_list_id) = *self_ref.active_list_id.borrow() {
                                if active_list_id == list_id {
                                    self_ref.load_list_translations(list_id);
                                }
                            }
                        }
                    }
                    popover_ref.popdown();
                });
                
                // Show the popover
                popover.popup();
            }
        }
    }
    
    fn delete_selected_translation(&self) {
        if let Some(row) = self.translation_list.selected_row() {
            let widget_id = row.widget_name().to_string().parse::<u32>().unwrap_or(0);
            
            // Use a scope to limit the lifetime of the borrow
            let translation_id_opt = {
                let map = self.translation_id_map.borrow();
                map.get(&widget_id).copied()
            };
            
            if let Some(translation_id) = translation_id_opt {
                // Now we can safely borrow mutably
                let result = self.db.borrow().delete_translation(translation_id);
                
                if result.is_ok() {
                    self.translation_id_map.borrow_mut().remove(&widget_id);
                    self.translation_list.remove(&row);
                } else {
                    // Show error dialog using our helper function
                    let parent_window = gtk::Window::list_toplevels()
                        .into_iter()
                        .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                        .and_then(|w| w.downcast::<gtk::Window>().ok());
                    
                    show_message_dialog(
                        parent_window.as_ref(),
                        MessageType::Error,
                        ButtonsType::Ok,
                        "Error deleting translation"
                    );
                }
            }
        }
    }
    
    fn export_selected_list(&self) {
        // Check if a list is selected
        if let Some(list_id) = *self.active_list_id.borrow() {
            // First, prepare the CSV data before showing any dialog
            let csv_result = self.db.borrow().export_list_for_anki(list_id);
            
            match csv_result {
                Ok(csv_data) => {
                    // Get list name for filename suggestion
                    let mut list_name = "translations".to_string();
                    for list in self.lists.borrow().iter() {
                        if list.id == list_id {
                            list_name = list.name.clone();
                            break;
                        }
                    }
                    let suggested_filename = format!("{}.csv", list_name.replace(" ", "_"));
                    
                    // Find a parent window
                    let parent_window = gtk::Window::list_toplevels()
                        .into_iter()
                        .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                        .and_then(|w| w.downcast::<gtk::Window>().ok());
                    
                    if let Some(parent) = parent_window {
                        // Create file chooser dialog
                        let dialog = gtk::FileChooserDialog::new(
                            Some("Export List"),
                            Some(&parent),
                            gtk::FileChooserAction::Save,
                            &[
                                ("Cancel", gtk::ResponseType::Cancel),
                                ("Save", gtk::ResponseType::Accept),
                            ],
                        );

                        // Set default filename
                        dialog.set_current_name(&suggested_filename);

                        // Add CSV file filter
                        let filter = gtk::FileFilter::new();
                        filter.set_name(Some("CSV Files"));
                        filter.add_pattern("*.csv");
                        dialog.add_filter(&filter);

                        // Add "All Files" filter
                        let all_filter = gtk::FileFilter::new();
                        all_filter.set_name(Some("All Files"));
                        all_filter.add_pattern("*");
                        dialog.add_filter(&all_filter);

                        // Try to set initial folder to user's home directory
                        if let Ok(home) = std::env::var("HOME") {
                            let _ = dialog.set_current_folder(Some(&gtk::gio::File::for_path(home)));
                        }

                        // Set modal
                        dialog.set_modal(true);

                        let csv_content = csv_data.clone();
                        let parent_clone = parent.clone();
                        
                        dialog.connect_response(move |dialog, response| {
                            if response == gtk::ResponseType::Accept {
                                if let Some(file) = dialog.file() {
                                    if let Some(path) = file.path() {
                                        // Check if file exists
                                        if path.exists() {
                                            let confirm_dialog = gtk::MessageDialog::new(
                                                Some(&parent_clone),
                                                gtk::DialogFlags::MODAL,
                                                gtk::MessageType::Question,
                                                gtk::ButtonsType::YesNo,
                                                &format!("File '{}' already exists. Do you want to overwrite it?", 
                                                    path.file_name().unwrap_or_default().to_string_lossy())
                                            );
                                            
                                            let path_clone = path.clone();
                                            let csv_content_clone = csv_content.clone();
                                            let parent_clone_inner = parent_clone.clone();
                                            
                                            confirm_dialog.connect_response(move |d, r| {
                                                d.destroy();
                                                if r == gtk::ResponseType::Yes {
                                                    // Write the file
                                                    match std::fs::write(&path_clone, &csv_content_clone) {
                                                        Ok(_) => show_success_dialog(&parent_clone_inner, &path_clone),
                                                        Err(e) => show_error_dialog(&parent_clone_inner, &e.to_string()),
                                                    }
                                                }
                                            });
                                            
                                            confirm_dialog.show();
                                        } else {
                                            // Write the file directly if it doesn't exist
                                            match std::fs::write(&path, &csv_content) {
                                                Ok(_) => show_success_dialog(&parent_clone, &path),
                                                Err(e) => show_error_dialog(&parent_clone, &e.to_string()),
                                            }
                                        }
                                    }
                                }
                            }
                            dialog.destroy();
                        });
                        
                        dialog.show();
                    } else {
                        println!("No suitable parent window found");
                    }
                },
                Err(err) => {
                    // Show error dialog for database error
                    let top_window = gtk::Window::list_toplevels()
                        .into_iter()
                        .find(|w| w.is_visible())
                        .and_then(|w| w.downcast::<gtk::Window>().ok());
                    
                    if let Some(parent) = top_window {
                        show_message_dialog(
                            Some(&parent),
                            MessageType::Error,
                            ButtonsType::Ok,
                            &format!("Error exporting data: {}", err)
                        );
                    } else {
                        eprintln!("Error exporting data: {}", err);
                    }
                }
            }
        } else {
            // No list selected, show an info message
            let top_window = gtk::Window::list_toplevels()
                .into_iter()
                .find(|w| w.is_visible())
                .and_then(|w| w.downcast::<gtk::Window>().ok());
                
            if let Some(parent) = top_window {
                show_message_dialog(
                    Some(&parent),
                    MessageType::Info,
                    ButtonsType::Ok,
                    "Please select a list to export"
                );
            } else {
                eprintln!("Please select a list to export");
            }
        }
    }

    // Add a method to update the view when translation state changes
    pub fn update_view(&self) {
        // Reset active list when switching to "all translations" mode
        *self.active_list_id.borrow_mut() = None;
        self.list_selector.set_active(Some(0));
        self.refresh_history();
    }

    // Add a method to handle new translations
    pub fn on_translation_added(&self) {
        // If no list is selected, refresh to show all translations
        // If a list is selected, keep showing only that list's translations
        self.refresh_history();
    }

    pub fn delete_current_list(&self) {
        // Only allow deletion if a list is selected (not All Translations)
        if let Some(list_id) = *self.active_list_id.borrow() {
            // Find the list name
            let list_name = {
                let lists = self.lists.borrow();
                lists.iter()
                    .find(|list| list.id == list_id)
                    .map(|list| list.name.clone())
                    .unwrap_or_else(|| "Unknown List".to_string())
            };
            
            // Get a reference to the parent window for dialogs
            let parent_window = gtk::Window::list_toplevels()
                .into_iter()
                .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                .and_then(|w| w.downcast::<gtk::Window>().ok());
                
            // Create confirmation dialog
            let dialog = gtk::MessageDialog::new(
                parent_window.as_ref(),
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Question,
                gtk::ButtonsType::YesNo,
                &format!("Are you sure you want to delete the list \"{}\"?\nThis action cannot be undone.", list_name)
            );
            
            // Store required data to avoid borrowing conflicts in the callback
            let db_clone = self.db.clone();
            let active_list_id_clone = self.active_list_id.clone();
            let self_clone = self.clone();
            let list_id_value = list_id;
            let list_name_value = list_name.clone();
            
            dialog.connect_response(move |dialog, response| {
                if response == gtk::ResponseType::Yes {
                    // User confirmed deletion
                    match db_clone.borrow().delete_list(list_id_value) {
                        Ok(_) => {
                            // Reset active list ID - use a separate scope for this borrow
                            {
                                *active_list_id_clone.borrow_mut() = None;
                            }
                            
                            // Reload lists
                            self_clone.load_lists();
                            
                            // Show all translations
                            self_clone.load_all_translations();
                            
                            // Show success message
                            let parent_window = gtk::Window::list_toplevels()
                                .into_iter()
                                .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                                .and_then(|w| w.downcast::<gtk::Window>().ok());
                                
                            show_message_dialog(
                                parent_window.as_ref(),
                                gtk::MessageType::Info,
                                gtk::ButtonsType::Ok,
                                &format!("List \"{}\" deleted successfully", list_name_value)
                            );
                        },
                        Err(e) => {
                            // Show error message
                            let parent_window = gtk::Window::list_toplevels()
                                .into_iter()
                                .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                                .and_then(|w| w.downcast::<gtk::Window>().ok());
                                
                            show_message_dialog(
                                parent_window.as_ref(),
                                gtk::MessageType::Error,
                                gtk::ButtonsType::Ok,
                                &format!("Error deleting list: {}", e)
                            );
                        }
                    }
                }
                dialog.destroy();
            });
            
            dialog.show();
        } else {
            // If "All Translations" is selected, show a message that no list is selected
            let parent_window = gtk::Window::list_toplevels()
                .into_iter()
                .find(|w| w.is_visible() && w.widget_name().as_str() != "GtkPopoverWindow")
                .and_then(|w| w.downcast::<gtk::Window>().ok());
                
            show_message_dialog(
                parent_window.as_ref(),
                gtk::MessageType::Info,
                gtk::ButtonsType::Ok,
                "Please select a list to delete"
            );
        }
    }
}

fn show_success_dialog(parent: &gtk::Window, path: &std::path::Path) {
    let message = format!(
        "List exported successfully to:\n\n{}\n\nWould you like to open the folder?", 
        path.display()
    );
    
    let success_dialog = gtk::MessageDialog::new(
        Some(parent),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Info,
        gtk::ButtonsType::YesNo,
        &message
    );
    
    let parent_dir = path.parent().unwrap_or(std::path::Path::new("")).to_path_buf();
    
    success_dialog.connect_response(move |d, r| {
        if r == gtk::ResponseType::Yes {
            let _ = std::process::Command::new("xdg-open")
                .arg(&parent_dir)
                .spawn();
        }
        d.destroy();
    });
    
    success_dialog.show();
}

fn show_error_dialog(parent: &gtk::Window, error_message: &str) {
    show_message_dialog(
        Some(parent),
        MessageType::Error,
        ButtonsType::Ok,
        &format!("Error writing file: {}", error_message)
    );
}