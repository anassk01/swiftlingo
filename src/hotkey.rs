use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use x11::xlib;
use std::ptr;
use std::mem;

// The exact window title to search for when focusing
const WINDOW_TITLE: &str = "Instant Translator";

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
                    // Remove the trigger file to reset for next time
                    std::fs::remove_file(&trigger_path).unwrap_or_else(|_| {});
                    // Get the selected text
                    let selection = get_current_selection();
                    if !selection.is_empty() {
                        // Write the selection to a file
                        let selection_path = format!("{}/selection.txt", trigger_dir);
                        if let Ok(mut file) = File::create(&selection_path) {
                            let _ = file.write_all(selection.as_bytes());
                            println!("Selection saved to: {}", selection_path);
                            
                            // Create a file to signal the app should come to the foreground
                            let focus_path = format!("{}/focus-window", trigger_dir);
                            if let Ok(file) = File::create(&focus_path) {
                                drop(file);
                            }
                            
                            // Try to directly activate the application window
                            focus_translator_window();
                        }
                    }
                }
            }
        });
    }
}

/// Try multiple methods to focus the translator window
fn focus_translator_window() {
    // Method 1: Use wmctrl with exact title match
    let _ = Command::new("wmctrl")
        .args(["-a", WINDOW_TITLE])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    
    // Method 2: Use xdotool with exact title match
    let _ = Command::new("xdotool")
        .args(["search", "--name", "^Instant Translator$", "windowactivate"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    
    // Method 3: Create a temporary focusing script
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let script_path = format!("{}/.config/translator-app/focus.sh", home_dir);
    
    if let Ok(mut file) = File::create(&script_path) {
        writeln!(file, "#!/bin/sh").unwrap();
        writeln!(file, "wmctrl -a \"{}\" || true", WINDOW_TITLE).unwrap();
        writeln!(file, "xdotool search --name \"{}\" windowactivate || true", WINDOW_TITLE).unwrap();
        writeln!(file, "# Try to focus using window class").unwrap();
        writeln!(file, "xdotool search --class \"translator-app\" windowactivate || true").unwrap();
    }
    
    let _ = Command::new("chmod")
        .args(["+x", &script_path])
        .status();
    
    let _ = Command::new("sh")
        .arg(&script_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
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
        writeln!(file, "# Focus the window after a small delay").unwrap();
        writeln!(file, "(sleep 0.5 && wmctrl -a \"{}\" || xdotool search --name \"{}\" windowactivate) &", 
                WINDOW_TITLE, WINDOW_TITLE).unwrap();
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
        writeln!(file, "# Focus the window after a small delay").unwrap();
        writeln!(file, "(sleep 0.5 && wmctrl -a \"{}\" || xdotool search --name \"{}\" windowactivate) &", 
                WINDOW_TITLE, WINDOW_TITLE).unwrap();
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
        writeln!(file, "# Focus the window after a small delay").unwrap();
        writeln!(file, "(sleep 0.5 && wmctrl -a \"{}\" || xdotool search --name \"{}\" windowactivate) &", 
                WINDOW_TITLE, WINDOW_TITLE).unwrap();
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
                        
                        // Create a file to signal the app should come to the foreground
                        let focus_path = format!("{}/.config/translator-app/focus-window", home_dir);
                        if let Ok(file) = File::create(&focus_path) {
                            drop(file);
                        }
                        
                        // Try to focus the translator window
                        focus_translator_window();
                    }
                }
            }
        }
    }
}

/// Get the current selection using the appropriate method
fn get_current_selection() -> String {
    // Try X11 selection method first
    if let Ok(output) = Command::new("xclip").args(["-o", "-selection", "primary"]).output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).to_string();
        }
    }
    
    // Try Wayland selection method
    if let Ok(output) = Command::new("wl-paste").arg("--primary").output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).to_string();
        }
    }
    
    // Fallback - try again with X environment variable unset (for XWayland)
    if let Ok(output) = Command::new("sh")
        .arg("-c")
        .arg("WAYLAND_DISPLAY= xclip -o -selection primary")
        .output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).to_string();
        }
    }
    
    String::new()
}