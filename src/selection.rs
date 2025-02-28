use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use gtk::glib;
use gtk::prelude::*;

pub enum SelectionSource {
    X11,
    Wayland,
    Unknown,
}

/// Detect which display server we're running on (still useful for logging)
pub fn detect_display_server() -> SelectionSource {
    match env::var("XDG_SESSION_TYPE") {
        Ok(session) => {
            if session.to_lowercase() == "wayland" {
                SelectionSource::Wayland
            } else {
                SelectionSource::X11
            }
        },
        Err(_) => {
            // Fallback detection
            if env::var("WAYLAND_DISPLAY").is_ok() {
                SelectionSource::Wayland
            } else if env::var("DISPLAY").is_ok() {
                SelectionSource::X11
            } else {
                SelectionSource::Unknown
            }
        }
    }
}

/// Get the currently selected text using GTK's clipboard API
pub fn get_selected_text() -> String {
    let display_server = detect_display_server();
    println!("Getting selected text using native GTK4 API (display: {})", match display_server {
        SelectionSource::X11 => "X11",
        SelectionSource::Wayland => "Wayland",
        SelectionSource::Unknown => "Unknown display server"
    });

    // Initialize an empty result string inside an Arc<Mutex> for thread safety
    let result = Arc::new(Mutex::new(String::new()));
    let result_clone = result.clone();
    
    // Flag to track if the operation is done
    let done = Arc::new(Mutex::new(false));
    let done_clone = done.clone();
    
    // Need to ensure we're on the main thread for GTK operations
    glib::MainContext::default().invoke(move || {
        // Get the default display
        if let Some(display) = gtk::gdk::Display::default() {
            // Get primary selection clipboard
            let primary = display.primary_clipboard();
            
            // Get text asynchronously
            primary.read_text_async(None::<&gio::Cancellable>, move |text_result| {
                match text_result {
                    Ok(Some(text)) => {
                        let mut result = result_clone.lock().unwrap();
                        *result = text.to_string();
                    },
                    Ok(None) => {
                        println!("No text in primary selection");
                    },
                    Err(e) => {
                        println!("Error reading primary selection: {}", e);
                    }
                }
                
                // Mark as done
                let mut done = done_clone.lock().unwrap();
                *done = true;
            });
        } else {
            // No display available
            println!("No GTK display available");
            let mut done = done_clone.lock().unwrap();
            *done = true;
        }
    });
    
    // Wait for the operation to complete (with timeout)
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(2); // 2 second timeout
    
    while !*done.lock().unwrap() {
        // Process GTK events to ensure callbacks run
        while glib::MainContext::default().iteration(false) {}
        
        // Check for timeout
        if start_time.elapsed() > timeout {
            println!("Timeout waiting for clipboard response");
            break;
        }
        
        // Small sleep to prevent CPU spinning
        std::thread::sleep(Duration::from_millis(10));
    }
    
    // Return the result
    let text = {
        let guard = result.lock().unwrap();
        guard.clone()
    };
    text
}

/// Set text to clipboard
pub fn set_clipboard_text(text: &str) -> bool {
    let success = Arc::new(Mutex::new(false));
    let success_clone = success.clone();
    let text = text.to_string();
    
    glib::MainContext::default().invoke(move || {
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&text);
            let mut success = success_clone.lock().unwrap();
            *success = true;
        }
    });
    
    // Process events to ensure the clipboard operation completes
    for _ in 0..10 {
        while glib::MainContext::default().iteration(false) {}
        std::thread::sleep(Duration::from_millis(10));
    }
    
    // Fix the lifetime issue by accessing the lock result and then returning
    let result = {
        let guard = success.lock().unwrap();
        *guard
    };
    result
}