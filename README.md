# SwiftLingo

A lightning-fast desktop translator built in Rust using GTK4. SwiftLingo runs silently in the background and springs to action with customizable hotkeys, instantly translating your selected text.

## Features

- Quick text translation using global hotkeys
- Multiple translation services support:
  - Google Translate (Beta/Free)
  - Google Translate (Official API)
  - LibreTranslate
  - Bing Translator
  - DeepL
- Translation history with SQLite storage
- Customizable settings with dark mode support
- Modern GTK4 UI with CSS styling
- Background operation with minimal resource usage
- Cross-platform support (X11 and Wayland)
- Fallback mechanism when primary translation service fails

## Project Structure

```
.
├── src/
│   ├── main.rs           # Application entry point and UI setup
│   ├── database.rs       # SQLite database operations
│   ├── history_ui.rs     # Translation history interface
│   ├── hotkey.rs         # Global hotkey management
│   ├── languages.rs      # Supported languages configuration
│   ├── selection.rs      # Text selection handling
│   ├── settings.rs       # Application settings management
│   ├── settings_ui.rs    # Settings interface
│   ├── translation.rs    # Translation services implementation
│   ├── ui_helpers.rs     # UI utility functions
│   ├── window_manager.rs # Window management
│   └── style.css        # Application styling
├── build.rs             # Build configuration
└── Cargo.toml           # Project dependencies and configuration
```

## Dependencies

- GTK4 for the user interface
- reqwest for HTTP requests
- tokio for async runtime
- rusqlite for SQLite database
- X11 for Linux window management
- Optional Wayland support via zbus

## Building from Source

1. Ensure you have Rust and Cargo installed
2. Install GTK4 development libraries:
   - Fedora: `sudo dnf install gtk4-devel`
   - Ubuntu: `sudo apt install libgtk4-dev`
   - Arch: `sudo pacman -S gtk4`
3. Clone the repository
4. Run `cargo build --release`
5. The binary will be available in `target/release/swiftlingo`

## Configuration

The app supports multiple translation services. Some services require API keys which can be configured in the settings dialog. The default service is Google Translate (Beta) which doesn't require an API key.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 