use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use x11::xlib;
use std::ptr;
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};

// For tracking hotkey timing to prevent duplicate triggers
static LAST_TRIGGER: AtomicU64 = AtomicU64::new(0);

// The exact window title to search for when focusing
const WINDOW_TITLE: &str = "SwiftLingo";

/// Start a background thread that monitors for global hotkey presses (Ctrl+Alt+T)
/// When detected, it writes a trigger file that the main app can watch for
pub fn start_global_hotkey_service() {
    // Create directory for trigger file if it doesn't exist
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let trigger_dir = format!("{}/.config/translator-app", home_dir);
    std::fs::create_dir_all(&trigger_dir).unwrap_or_else(|_| {
        println!("Could not create config directory");
    });
    
    let trigger_path = format!("{}/hotkey-trigger", trigger_dir);
    
    // Determine which environment we're running in
    let display_server = if env::var("XDG_SESSION_TYPE").map(|s| s.to_lowercase() == "wayland").unwrap_or(false) 
        || env::var("WAYLAND_DISPLAY").is_ok() {
        "wayland"
    } else {
        "x11"
    };
    
    // Create and register the hotkey based on the environment
    if display_server == "x11" {
        thread::spawn(move || {
            println!("Starting X11 global hotkey monitor for Ctrl+Alt+T");
            monitor_x11_hotkey(&trigger_path);
        });
    } else {
        // For Wayland, we'll try to register using desktop environment settings
        if is_gnome() {
            // Use GNOME settings to register a shortcut
            println!("Detected GNOME - Setting up global hotkey via gsettings");
            setup_gnome_shortcut(&trigger_path);
        } else if is_kde() {
            // Use KDE settings to register a shortcut
            println!("Detected KDE - Setting up global hotkey via KDE settings");
            setup_kde_shortcut(&trigger_path);
        } else {
            println!("Using generic method for Wayland desktop environment");
            setup_generic_shortcut(&trigger_path);
        }
        
        // No matter what method we use, we'll still watch for the trigger file
        thread::spawn(move || {
            println!("Starting trigger file monitor thread");
            loop {
                thread::sleep(Duration::from_millis(100));
                if Path::new(&trigger_path).exists() {
                    // The file exists - this means our shortcut was triggered
                    println!("Hotkey trigger detected!");
                    
                    // Check if we're triggering too frequently to prevent duplicate launches
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    
                    let last = LAST_TRIGGER.load(Ordering::Relaxed);
                    if now - last < 1000 {  // Prevent triggers within 1 second
                        // Remove the trigger file to reset for next time
                        std::fs::remove_file(&trigger_path).unwrap_or_else(|_| {});
                        println!("Ignoring rapid repeated hotkey trigger");
                        continue;
                    }
                    LAST_TRIGGER.store(now, Ordering::Relaxed);
                    
                    // Remove the trigger file to reset for next time
                    std::fs::remove_file(&trigger_path).unwrap_or_else(|_| {});

                    // Focus the window ONLY ONCE
                    focus_translator_window();
                    
                    // Get the selected text
                    let selection = get_current_selection();
                    if !selection.is_empty() {
                        // Write the selection to a file
                        let selection_path = format!("{}/selection.txt", trigger_dir);
                        if let Ok(mut file) = File::create(&selection_path) {
                            let _ = file.write_all(selection.as_bytes());
                            println!("Selection saved to: {}", selection_path);
                        }
                    }
                }
            }
        });
    }
}

/// Try multiple methods to focus the translator window
fn focus_translator_window() {
    // First check if window exists to avoid launching new instances
    let window_exists = Command::new("xdotool")
        .args(["search", "--name", "^SwiftLingo$"])
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);

    // If window doesn't exist, just create the focus trigger file and return
    if !window_exists {
        // Create a trigger file that the main app will monitor
        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let trigger_dir = format!("{}/.config/translator-app", home_dir);
        std::fs::create_dir_all(&trigger_dir).unwrap_or_else(|_| {});
        
        let focus_path = format!("{}/focus-window", trigger_dir);
        if let Ok(file) = std::fs::File::create(&focus_path) {
            drop(file); // Just create the file as a trigger
            println!("Created focus trigger file - no existing window found");
        }
        return;
    }

    // Check if we're running on Wayland
    let is_wayland = env::var("XDG_SESSION_TYPE")
        .map(|s| s.to_lowercase() == "wayland")
        .unwrap_or(false) 
        || env::var("WAYLAND_DISPLAY").is_ok();
    
    if is_wayland {
        // On Wayland, we can't directly focus windows from outside the app
        // Create a trigger file that the main app will monitor
        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let trigger_dir = format!("{}/.config/translator-app", home_dir);
        std::fs::create_dir_all(&trigger_dir).unwrap_or_else(|_| {});
        
        let focus_path = format!("{}/focus-window", trigger_dir);
        if let Ok(file) = std::fs::File::create(&focus_path) {
            drop(file); // Just create the file as a trigger
            println!("Created focus trigger file for Wayland");
        }
        return;
    }
    
    // X11-specific methods - only run these on X11
    // First try to get window ID
    if let Ok(output) = Command::new("xdotool")
        .args(["search", "--name", "^SwiftLingo$"])
        .output()
    {
        if let Ok(window_id) = String::from_utf8(output.stdout) {
            if !window_id.is_empty() {
                // Unmap and map the window to force it to top
                let _ = Command::new("xdotool")
                    .args(["windowunmap", &window_id])
                    .status();
                
                let _ = Command::new("xdotool")
                    .args(["windowmap", &window_id])
                    .status();
                
                // Now focus it
                let _ = Command::new("xdotool")
                    .args([
                        "windowactivate",
                        "--sync",
                        &window_id,
                        "windowraise",
                        "windowfocus",
                        "mousemove", "--window", &window_id, "0", "0"
                    ])
                    .status();
                
                return; // Window focused successfully
            }
        }
    }
    
    // Fallback methods if window ID approach failed - only try these if we confirmed window exists earlier
    let methods = vec![
        // Method 1: Use wmctrl to force window above others
        Command::new("wmctrl")
            .args(["-F", "-a", WINDOW_TITLE])
            .spawn(),
            
        // Method 2: Use wmctrl to force window state
        Command::new("wmctrl")
            .args(["-F", "-a", WINDOW_TITLE, "-b", "remove,hidden,shaded", "-b", "add,above,sticky"])
            .spawn(),
    ];
    
    // Wait for all methods to complete
    for mut child in methods.into_iter().filter_map(Result::ok) {
        let _ = child.wait();
    }
}

/// Check if we're running under GNOME
fn is_gnome() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|desktop| desktop.to_lowercase().contains("gnome"))
        .unwrap_or(false)
}

/// Check if we're running under KDE
fn is_kde() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|desktop| desktop.to_lowercase().contains("kde"))
        .unwrap_or(false)
}

/// Set up a GNOME shortcut for Ctrl+Alt+T
fn setup_gnome_shortcut(trigger_path: &str) {
    // Create a small script that will create the trigger file
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let script_path = format!("{}/.config/translator-app/trigger.sh", home_dir);
    
    if let Ok(mut file) = File::create(&script_path) {
        writeln!(file, "#!/bin/sh").unwrap();
        writeln!(file, "touch {}", trigger_path).unwrap();
    }
    
    let _ = Command::new("chmod")
        .args(["+x", &script_path])
        .status();
    
    // Register the shortcut with GNOME - ignoring errors
    let _ = Command::new("gsettings")
        .args([
            "set", "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings", "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/translator/']"
        ])
        .status();
    
    let _ = Command::new("gsettings")
        .args([
            "set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/translator/",
            "name", "'Translator Hotkey'"
        ])
        .status();
    
    let _ = Command::new("gsettings")
        .args([
            "set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/translator/",
            "command", &format!("'{}'", script_path)
        ])
        .status();
    
    let _ = Command::new("gsettings")
        .args([
            "set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/translator/",
            "binding", "'<Control><Alt>t'"
        ])
        .status();
}

/// Set up a KDE shortcut for Ctrl+Alt+T
fn setup_kde_shortcut(trigger_path: &str) {
    // Create a small script that will create the trigger file
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let script_path = format!("{}/.config/translator-app/trigger.sh", home_dir);
    
    if let Ok(mut file) = File::create(&script_path) {
        writeln!(file, "#!/bin/sh").unwrap();
        writeln!(file, "touch {}", trigger_path).unwrap();
    }
    
    let _ = Command::new("chmod")
        .args(["+x", &script_path])
        .status();
    
    // For KDE, we can use kwriteconfig5 to set the shortcut - ignoring errors
    let _ = Command::new("kwriteconfig5")
        .args([
            "--file", "kglobalshortcutsrc",
            "--group", "translator",
            "--key", "TranslatorHotkey", 
            &format!("{},,Translator Hotkey", script_path)
        ])
        .status();
    
    // Reload KDE shortcut config - ignoring errors
    let _ = Command::new("kquitapp5")
        .arg("kglobalaccel")
        .status();
}

/// Generic shortcut setup for other desktop environments
fn setup_generic_shortcut(trigger_path: &str) {
    // Create a small script that will create the trigger file
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let script_path = format!("{}/.config/translator-app/trigger.sh", home_dir);
    
    if let Ok(mut file) = File::create(&script_path) {
        writeln!(file, "#!/bin/sh").unwrap();
        writeln!(file, "touch {}", trigger_path).unwrap();
    }
    
    let _ = Command::new("chmod")
        .args(["+x", &script_path])
        .status();
    
    println!("Shortcut registration for your desktop environment is not directly supported.");
    println!("Please manually add a global shortcut for Ctrl+Alt+T that runs:");
    println!("  {}", script_path);
}

/// Monitor for X11 global hotkey (Ctrl+Alt+T)
fn monitor_x11_hotkey(trigger_path: &str) {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            println!("Failed to open X display");
            return;
        }
        
        let root = xlib::XDefaultRootWindow(display);
        let ctrl_mask = xlib::ControlMask;
        let alt_mask = xlib::Mod1Mask;
        
        // Get the keycode for 't'
        let t_keysym = xlib::XStringToKeysym(b"t\0".as_ptr() as *const _);
        let t_keycode = xlib::XKeysymToKeycode(display, t_keysym);
        
        // Ungrab any existing grabs on the root window
        xlib::XUngrabKey(display, xlib::AnyKey, xlib::AnyModifier, root);
        
        // Grab the key combination (Ctrl+Alt+T) globally on the root window
        let grab_result = xlib::XGrabKey(
            display,
            t_keycode as i32,
            ctrl_mask | alt_mask,
            root,
            1,
            xlib::GrabModeAsync,
            xlib::GrabModeAsync,
        );
        
        if grab_result == 0 {
            println!("Failed to grab key combination");
            return;
        }
        
        // Handle different modifier combinations (Caps Lock, Num Lock, etc.)
        let modifiers = [
            ctrl_mask | alt_mask,
            ctrl_mask | alt_mask | xlib::LockMask,
            ctrl_mask | alt_mask | xlib::Mod2Mask,
            ctrl_mask | alt_mask | xlib::LockMask | xlib::Mod2Mask,
        ];
        
        for &modifier in modifiers.iter() {
            xlib::XGrabKey(
                display,
                t_keycode as i32,
                modifier,
                root,
                1,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
            );
        }
        
        xlib::XSync(display, 0);
        println!("X11 key grabs established for Ctrl+Alt+T");
        
        let mut event: xlib::XEvent = mem::zeroed();
        
        loop {
            xlib::XNextEvent(display, &mut event);
            
            if event.get_type() == xlib::KeyPress {
                let key_event = xlib::XKeyEvent::from(event);
                let state = key_event.state & !(xlib::LockMask | xlib::Mod2Mask);
                
                if key_event.keycode == t_keycode as u32 && 
                   (state == (ctrl_mask | alt_mask)) {
                    println!("X11 Hotkey Ctrl+Alt+T detected!");
                    
                    // Check for rapid repeated triggers
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    
                    let last = LAST_TRIGGER.load(Ordering::Relaxed);
                    if now - last < 1000 {  // Prevent triggers within 1 second
                        println!("Ignoring rapid repeated hotkey trigger");
                        continue;
                    }
                    LAST_TRIGGER.store(now, Ordering::Relaxed);
                    
                    // Create trigger file
                    if let Ok(file) = File::create(trigger_path) {
                        drop(file);
                    }
                    
                    // Get the selection
                    let selection = get_current_selection();
                    if !selection.is_empty() {
                        // Write the selection to a file
                        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
                        let selection_path = format!("{}/.config/translator-app/selection.txt", home_dir);
                        if let Ok(mut file) = File::create(&selection_path) {
                            let _ = file.write_all(selection.as_bytes());
                        }
                        
                        // Focus the window - only call ONCE
                        focus_translator_window();
                    }
                }
            }
        }
    }
}

/// Get the current selection using the appropriate method
fn get_current_selection() -> String {
    crate::selection::get_selected_text()
}