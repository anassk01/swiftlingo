use std::env;
use std::process::Command;

pub enum SelectionSource {
    X11,
    Wayland,
    Unknown,
}

/// Detect which display server we're running on
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

/// Get the currently selected text using the appropriate method for the detected display server
pub fn get_selected_text() -> String {
    let display_server = detect_display_server();
    println!("Getting selected text from {}", match display_server {
        SelectionSource::X11 => "X11",
        SelectionSource::Wayland => "Wayland",
        SelectionSource::Unknown => "Unknown display server"
    });

    match display_server {
        SelectionSource::X11 => get_x11_selection(),
        SelectionSource::Wayland => get_wayland_selection(),
        SelectionSource::Unknown => String::from("Could not detect display server")
    }
}

/// Get selected text on X11 using xclip
fn get_x11_selection() -> String {
    // Try first using xclip
    match Command::new("xclip")
        .args(["-o", "-selection", "primary"])
        .output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).to_string()
        },
        _ => {
            // Fallback to xsel
            match Command::new("xsel")
                .arg("--primary")
                .output() {
                Ok(output) if output.status.success() => {
                    String::from_utf8_lossy(&output.stdout).to_string()
                },
                _ => String::from("Failed to get X11 selection")
            }
        }
    }
}

/// Get selected text on Wayland using wl-paste
fn get_wayland_selection() -> String {
    // Try wl-paste first
    match Command::new("wl-paste")
        .arg("--primary")
        .output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).to_string()
        },
        _ => {
            // Fallback to XWayland (xclip with WAYLAND_DISPLAY unset)
            match Command::new("sh")
                .arg("-c")
                .arg("WAYLAND_DISPLAY= xclip -o -selection primary")
                .output() {
                Ok(output) if output.status.success() => {
                    String::from_utf8_lossy(&output.stdout).to_string()
                },
                _ => String::from("Failed to get Wayland selection")
            }
        }
    }
}