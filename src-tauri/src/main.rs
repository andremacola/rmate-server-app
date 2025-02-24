use std::process::Command;
use std::sync::Mutex;
use std::fs;
use std::io::Result as IoResult;
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, Manager};
use std::os::unix::fs::PermissionsExt;

// Structure to hold the server process
struct RmateServer(Mutex<Option<std::process::Child>>);

fn ensure_server_executable(path: &std::path::Path) -> IoResult<()> {
    fs::set_permissions(path, PermissionsExt::from_mode(0o755))
}

fn start_server(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<RmateServer>();
    let mut server_guard = state.0.lock().unwrap();
    
    // Try to kill any existing server process using the same port
    let _ = Command::new("pkill")
        .args(["-f", "zed-rmate-server"])
        .output();

    // Wait a moment for the port to be released
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    let server_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bin")
        .join("zed-rmate-server");

    if !server_path.exists() {
        return Err(format!("Server binary not found at: {}", server_path.display()));
    }

    if let Err(e) = ensure_server_executable(&server_path) {
        return Err(format!("Failed to set executable permissions: {}", e));
    }

    match Command::new(&server_path)
        .spawn() {
        Ok(child) => {
            *server_guard = Some(child);
            Ok(())
        }
        Err(e) => {
            Err(format!("Failed to start server: {}", e))
        }
    }
}

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let server_toggle = CustomMenuItem::new("server_toggle".to_string(), "Stop Server");

    let tray_menu = SystemTrayMenu::new()
        .add_item(server_toggle)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);
    let server = RmateServer(Mutex::new(None));

    tauri::Builder::default()
        .system_tray(system_tray)
        .manage(server)
        .setup(|app| {
            // Set macOS activation policy to Accessory
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Start the server automatically when the app launches
            if let Err(e) = start_server(&app.handle()) {
                eprintln!("Error starting server: {}", e);
            }
            Ok(())
        })
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    let state = app.state::<RmateServer>();
                    if let Some(mut server) = state.0.lock().unwrap().take() {
                        let _ = server.kill();
                    }
                    app.exit(0);
                }
                "server_toggle" => {
                    let state = app.state::<RmateServer>();
                    let mut server_guard = state.0.lock().unwrap();
                    
                    if server_guard.is_some() {
                        // Stop the server
                        if let Some(mut server) = server_guard.take() {
                            let _ = server.kill();
                            drop(server); // Ensure the server process is properly dropped
                            app.tray_handle().get_item("server_toggle").set_title("Start Server").unwrap();
                            app.tray_handle().set_icon(tauri::Icon::Raw(include_bytes!("../icons/icon-off.png").to_vec())).unwrap();
                            eprintln!("Server stopped successfully");
                        }
                    } else {
                        // Start the server
                        drop(server_guard); // Release the lock before starting new server
                        if let Err(e) = start_server(app) {
                            eprintln!("Error: {}", e);
                        } else {
                            app.tray_handle().get_item("server_toggle").set_title("Stop Server").unwrap();
                            app.tray_handle().set_icon(tauri::Icon::Raw(include_bytes!("../icons/icon.png").to_vec())).unwrap();
                            eprintln!("Server started successfully");
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}