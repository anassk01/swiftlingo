use std::process::Command;
use std::env;
use std::io::Write;
use std::fs;
use gtk::prelude::*;
use gtk::Window;
use gtk::gdk;
#[cfg(feature = "x11")]
use gdk4_x11::X11Surface;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use std::thread;
use std::time::Duration;
use std::sync::Arc;

// The window title to match
const WINDOW_TITLE: &str = "SwiftLingo";

type Window32 = u32;

#[derive(Debug)]
struct X11Connection {
    conn: Arc<x11rb::rust_connection::RustConnection>,
    atoms: X11Atoms,
}

#[derive(Debug)]
struct X11Atoms {
    net_active_window: u32,
    net_wm_state: u32,
    net_wm_state_above: u32,
    net_wm_state_sticky: u32,
}

/// Centralizes window management functionality for consistent behavior
pub struct WindowManager {
    is_wayland: bool,
    x11_conn: Option<X11Connection>,
}

impl WindowManager {
    /// Create a new WindowManager with auto-detected display server type
    pub fn new() -> Self {
        let is_wayland = WindowManager::detect_wayland();
        let x11_conn = if !is_wayland {
            WindowManager::setup_x11().ok()
        } else {
            None
        };
        
        let wm = WindowManager {
            is_wayland,
            x11_conn,
        };
        
        // If running under Wayland, start the focus trigger file monitor
        if wm.is_wayland {
            wm.start_focus_trigger_monitor();
        }
        
        wm
    }

    /// Setup X11 connection and get required atoms
    fn setup_x11() -> Result<X11Connection, Box<dyn std::error::Error>> {
        let (conn, screen_num) = x11rb::connect(None).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let conn = Arc::new(conn);
        let screen = &conn.setup().roots[screen_num];
        let _root = screen.root; // Keep for future use if needed

        // Get required atoms
        let net_active_window = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .map_err(|e| Box::new(e))?.reply()
            .map_err(|e| Box::new(e))?.atom;
            
        let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")
            .map_err(|e| Box::new(e))?.reply()
            .map_err(|e| Box::new(e))?.atom;
            
        let net_wm_state_above = conn.intern_atom(false, b"_NET_WM_STATE_ABOVE")
            .map_err(|e| Box::new(e))?.reply()
            .map_err(|e| Box::new(e))?.atom;
            
        let net_wm_state_sticky = conn.intern_atom(false, b"_NET_WM_STATE_STICKY")
            .map_err(|e| Box::new(e))?.reply()
            .map_err(|e| Box::new(e))?.atom;

        let atoms = X11Atoms {
            net_active_window,
            net_wm_state,
            net_wm_state_above,
            net_wm_state_sticky,
        };

        Ok(X11Connection {
            conn,
            atoms,
        })
    }
    
    /// Start monitoring for the Wayland focus trigger file
    fn start_focus_trigger_monitor(&self) {
        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let focus_path = format!("{}/.config/translator-app/focus-window", home_dir);
        
        thread::spawn(move || {
            loop {
                if fs::metadata(&focus_path).is_ok() {
                    // Remove the trigger file first to prevent race conditions
                    let _ = fs::remove_file(&focus_path);
                    
                    // Small delay to allow the file to be fully processed
                    thread::sleep(Duration::from_millis(50));
                }
                
                thread::sleep(Duration::from_millis(100));
            }
        });
    }
    
    /// Detect if we're running under Wayland
    fn detect_wayland() -> bool {
        // Check multiple indicators for Wayland
        let session_type = env::var("XDG_SESSION_TYPE")
            .map(|s| s.to_lowercase() == "wayland")
            .unwrap_or(false);
        let wayland_display = env::var("WAYLAND_DISPLAY").is_ok();
        let gdk_backend = env::var("GDK_BACKEND")
            .map(|s| s.to_lowercase() == "wayland")
            .unwrap_or(false);
            
        session_type || wayland_display || gdk_backend
    }
    
    /// No need to install tools anymore as we use native implementation
    pub fn install_tools_if_needed(&self) {
        // Native implementation doesn't require external tools
    }
    
    /// Setup initial window properties
    pub fn setup_window(&self, window: &impl IsA<Window>) {
        if let Some(win) = window.dynamic_cast_ref::<Window>() {
            // Set window properties for better desktop integration
            win.set_focusable(true);
            win.set_can_focus(true);
            
            // Set window title
            win.set_title(Some(WINDOW_TITLE));
            
            // Ensure window appears in taskbar and can be focused
            win.set_hide_on_close(true);
            
            // Make window appear on all workspaces (sticky) - X11 only
            if !self.is_wayland {
                if let Some(_x11_conn) = &self.x11_conn {
                    let surface = win.surface();
                    #[cfg(feature = "x11")]
                    if let Some(x11_surface) = surface.downcast_ref::<X11Surface>() {
                        let window_id = x11_surface.xid() as Window32;
                        self.set_window_state(window_id, true, true);
                    }
                }
            }
        }
    }

    /// Set window state (sticky and above)
    fn set_window_state(&self, window_id: Window32, sticky: bool, above: bool) {
        if let Some(x11_conn) = &self.x11_conn {
            let mut data = vec![];
            
            if sticky {
                data.push(x11_conn.atoms.net_wm_state_sticky);
            }
            if above {
                data.push(x11_conn.atoms.net_wm_state_above);
            }

            if !data.is_empty() {
                let data_bytes: Vec<u8> = data.iter()
                    .flat_map(|&x| x.to_ne_bytes())
                    .collect();

                let _ = x11_conn.conn.change_property(
                    PropMode::REPLACE,
                    window_id,
                    x11_conn.atoms.net_wm_state,
                    AtomEnum::ATOM,
                    32,
                    data.len() as u32,
                    &data_bytes,
                );
                let _ = x11_conn.conn.flush();
            }
        }
    }
    
    /// Focus this window using the most appropriate method for the environment
    pub fn focus_window(&self, window: &impl IsA<Window>) {
        if let Some(win) = window.dynamic_cast_ref::<Window>() {
            // First ensure window is mapped and visible
            win.show();
            win.unminimize();
            
            if self.is_wayland {
                // Wayland-specific window management
                win.present_with_time(gdk::CURRENT_TIME);
            } else {
                // X11-specific window management
                win.present();
                let surface = win.surface();
                #[cfg(feature = "x11")]
                if let Some(x11_surface) = surface.downcast_ref::<X11Surface>() {
                    let window_id = x11_surface.xid() as Window32;
                    self.focus_x11_window(window_id);
                }
            }
        }
    }
    
    /// Focus X11 window using native X11 calls
    fn focus_x11_window(&self, window_id: Window32) {
        if let Some(x11_conn) = &self.x11_conn {
            // Send _NET_ACTIVE_WINDOW message
            let data = [1, gdk::CURRENT_TIME, 0, 0, 0];
            let _ = x11_conn.conn.send_event(
                false,
                window_id,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                ClientMessageEvent::new(
                    32,
                    window_id,
                    x11_conn.atoms.net_active_window,
                    data,
                ),
            );
            let _ = x11_conn.conn.flush();
        }
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
            // Create a simple script that uses our native implementation
            let script_content = format!(
                "#!/bin/sh\n{} --focus\n",
                env::current_exe().unwrap_or_default().to_string_lossy()
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
    pub fn minimize_window(&self, window: &impl IsA<Window>) {
        window.minimize();
    }
    
    /// Check if required functionality is available
    pub fn check_clipboard_tools(&self) -> bool {
        true // Native implementation always available
    }
}