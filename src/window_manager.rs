use std::process::{Command, Stdio};
use std::env;
use std::io::Write;
use std::fs;
use gtk::prelude::*;

// The window title to match
const WINDOW_TITLE: &str = "Instant Translator";
const APP_ID: &str = "org.example.translator";

/// Centralizes window management functionality for consistent behavior
pub struct WindowManager {
    is_wayland: bool,
}

impl WindowManager {
    /// Create a new WindowManager with auto-detected display server type
    pub fn new() -> Self {
        let is_wayland = WindowManager::detect_wayland();
        
        WindowManager {
            is_wayland,
        }
    }
    
    /// Detect if we're running under Wayland
    fn detect_wayland() -> bool {
        env::var("XDG_SESSION_TYPE").map(|s| s.to_lowercase() == "wayland").unwrap_or(false) 
            || env::var("WAYLAND_DISPLAY").is_ok()
    }
    
    /// Check if required tools are installed and install them if needed
    pub fn install_tools_if_needed(&self) {
        // Only install X11 tools if not running under Wayland
        if !self.is_wayland {
            // Check if wmctrl is installed
            if Command::new("which").arg("wmctrl").stdout(Stdio::null()).status().map(|s| !s.success()).unwrap_or(true) {
                println!("Installing wmctrl for better window management...");
                let _ = Command::new("pkexec")
                    .args(["dnf", "install", "-y", "wmctrl"])
                    .status();
            }

            // Check if xdotool is installed
            if Command::new("which").arg("xdotool").stdout(Stdio::null()).status().map(|s| !s.success()).unwrap_or(true) {
                println!("Installing xdotool for better window management...");
                let _ = Command::new("pkexec")
                    .args(["dnf", "install", "-y", "xdotool"])
                    .status();
            }
        }
    }
    
    /// Focus this window using the most appropriate method for the environment
    pub fn focus_window(&self, window: &impl IsA<gtk::Window>) {
        // First use GTK's native mechanism
        window.present_with_time(gtk::gdk::CURRENT_TIME);
        
        // Then use additional methods for X11 if needed
        if !self.is_wayland {
            self.focus_x11_window();
        }
    }
    
    /// Focus X11 window using external tools
    fn focus_x11_window(&self) {
        // Method 1: Try by window ID
        if let Ok(output) = Command::new("xdotool")
            .args(["search", "--class", APP_ID])
            .output() {
            if output.status.success() {
                if let Ok(wid_str) = String::from_utf8(output.stdout) {
                    if !wid_str.trim().is_empty() {
                        // Use the window ID to focus
                        let _ = Command::new("xdotool")
                            .args(["windowactivate", wid_str.trim()])
                            .status();
                        return;
                    }
                }
            }
        }
        
        // Method 2: Try by window title
        let _ = Command::new("xdotool")
            .args(["search", "--name", WINDOW_TITLE, "windowactivate"])
            .status();
        
        // Method 3: Try wmctrl
        let _ = Command::new("wmctrl")
            .args(["-a", WINDOW_TITLE])
            .status();
    }
    
    /// Create a temporary script for focusing if needed
    #[allow(dead_code)]
    pub fn create_focus_script(&self) -> Option<String> {
        if self.is_wayland {
            return None; // Not needed for Wayland
        }
        
        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_dir = format!("{}/.config/translator-app", home_dir);
        let script_path = format!("{}/focus.sh", config_dir);
        
        // Create directory if needed
        let _ = fs::create_dir_all(&config_dir);
        
        if let Ok(mut file) = fs::File::create(&script_path) {
            let script_content = format!(
                "#!/bin/sh\nwmctrl -a \"{}\" || true\nxdotool search --name \"{}\" windowactivate || true\n",
                WINDOW_TITLE, WINDOW_TITLE
            );
            
            if file.write_all(script_content.as_bytes()).is_ok() {
                // Make executable
                let _ = Command::new("chmod")
                    .args(["+x", &script_path])
                    .status();
                
                return Some(script_path);
            }
        }
        
        None
    }
    
    /// Minimize the window
    pub fn minimize_window(&self, window: &impl IsA<gtk::Window>) {
        // Use GTK's API if available
        window.minimize();
        
        // Fallback for older GTK versions using xdotool
        if !self.is_wayland {
            let _ = Command::new("xdotool")
                .args(["search", "--name", WINDOW_TITLE, "windowminimize"])
                .status();
        }
    }
    
    /// Check if clipboard tools are installed
    pub fn check_clipboard_tools(&self) -> bool {
        let has_xclip = Command::new("which").arg("xclip").status().map(|s| s.success()).unwrap_or(false);
        let has_wl_paste = Command::new("which").arg("wl-paste").status().map(|s| s.success()).unwrap_or(false);
        
        if !has_xclip && !self.is_wayland {
            println!("xclip not found. For X11 selection support, install with: sudo dnf install xclip");
        }
        
        if !has_wl_paste && self.is_wayland {
            println!("wl-paste not found. For Wayland selection support, install with: sudo dnf install wl-clipboard");
        }
        
        has_xclip || has_wl_paste
    }
}