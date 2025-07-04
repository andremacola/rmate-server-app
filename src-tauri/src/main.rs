use std::fs;
use std::io::Result as IoResult;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{
    async_runtime, AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, SystemTraySubmenu,
};

#[derive(Debug, Clone, PartialEq)]
enum Editor {
    Zed,
    Vscode,
    Sublime,
}

impl Editor {
    fn to_bin_path(&self) -> &str {
        match self {
            Editor::Zed => "/usr/local/bin/zed",
            Editor::Vscode => "/usr/local/bin/code",
            Editor::Sublime => "/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl",
        }
    }

    fn to_menu_id(&self) -> &str {
        match self {
            Editor::Zed => "select_zed",
            Editor::Vscode => "select_vscode",
            Editor::Sublime => "select_sublime",
        }
    }
}

// App state
struct AppState {
    server: Mutex<Option<std::process::Child>>,
    editor: Arc<Mutex<Editor>>,
}

fn ensure_server_executable(path: &std::path::Path) -> IoResult<()> {
    fs::set_permissions(path, PermissionsExt::from_mode(0o755))
}

fn stop_server(app: &AppHandle) {
    let state = app.state::<AppState>();
    let server_to_kill = state.server.lock().unwrap().take();

    if let Some(mut server) = server_to_kill {
        let _ = server.kill();
        let _ = app
            .tray_handle()
            .get_item("server_toggle")
            .set_title("Start Server");
        let _ = app.tray_handle().set_icon(tauri::Icon::Raw(
            include_bytes!("../icons/icon-off.png").to_vec(),
        ));
    }
}

fn start_server(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut server_guard = state.server.lock().unwrap();
    let editor = state.editor.lock().unwrap().clone();

    // Ensure any previous server process is killed
    let _ = Command::new("pkill")
        .args(["-f", "zed-rmate-server"])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(500));

    let server_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bin")
        .join("zed-rmate-server");

    if !server_path.exists() {
        return Err(format!(
            "Server binary not found at: {}",
            server_path.display()
        ));
    }

    if let Err(e) = ensure_server_executable(&server_path) {
        return Err(format!("Failed to set executable permissions: {}", e));
    }

    match Command::new(&server_path)
        .arg("--zed-bin")
        .arg(editor.to_bin_path())
        .spawn()
    {
        Ok(child) => {
            *server_guard = Some(child);
            let _ = app
                .tray_handle()
                .get_item("server_toggle")
                .set_title("Stop Server");
            let _ = app.tray_handle().set_icon(tauri::Icon::Raw(
                include_bytes!("../icons/icon.png").to_vec(),
            ));
            Ok(())
        }
        Err(e) => Err(format!(
            "Failed to start server for {}: {}",
            editor.to_bin_path(),
            e
        )),
    }
}

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let server_toggle = CustomMenuItem::new("server_toggle".to_string(), "Stop Server");

    let select_zed = CustomMenuItem::new(Editor::Zed.to_menu_id().to_string(), "Zed").selected();
    let select_vscode = CustomMenuItem::new(Editor::Vscode.to_menu_id().to_string(), "VS Code");
    let select_sublime =
        CustomMenuItem::new(Editor::Sublime.to_menu_id().to_string(), "Sublime Text");

    let editor_menu = SystemTrayMenu::new()
        .add_item(select_zed)
        .add_item(select_vscode)
        .add_item(select_sublime);

    let editor_submenu = SystemTraySubmenu::new("Select Editor", editor_menu);

    let tray_menu = SystemTrayMenu::new()
        .add_item(server_toggle)
        .add_submenu(editor_submenu)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);
    let app_state = AppState {
        server: Mutex::new(None),
        editor: Arc::new(Mutex::new(Editor::Zed)),
    };

    tauri::Builder::default()
        .system_tray(system_tray)
        .manage(app_state)
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
        .on_system_tray_event(|app, event| {
            let state = app.state::<AppState>();
            match event {
                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "quit" => {
                        if let Some(mut server) = state.server.lock().unwrap().take() {
                            let _ = server.kill();
                        }
                        app.exit(0);
                    }
                    "server_toggle" => {
                        let server_guard = state.server.lock().unwrap();
                        if server_guard.is_some() {
                            stop_server(app);
                        } else {
                            drop(server_guard); // Release lock before starting
                            if let Err(e) = start_server(app) {
                                eprintln!("Error starting server: {}", e);
                            }
                        }
                    }
                    "select_zed" | "select_vscode" | "select_sublime" => {
                        let new_editor = match id.as_str() {
                            "select_zed" => Editor::Zed,
                            "select_vscode" => Editor::Vscode,
                            _ => Editor::Sublime,
                        };

                        let mut current_editor = state.editor.lock().unwrap();
                        if *current_editor != new_editor {
                            // Uncheck the old editor
                            let _ = app
                                .tray_handle()
                                .get_item(current_editor.to_menu_id())
                                .set_selected(false);

                            // Update state
                            *current_editor = new_editor.clone();

                            // Check the new editor
                            let _ = app
                                .tray_handle()
                                .get_item(new_editor.to_menu_id())
                                .set_selected(true);

                            // Restart server if it was running to apply the new editor
                            let server_is_running = state.server.lock().unwrap().is_some();
                            if server_is_running {
                                stop_server(app);
                                let app_handle = app.clone();
                                async_runtime::spawn(async move {
                                    // Give the OS a moment to release the port
                                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                    if let Err(e) = start_server(&app_handle) {
                                        eprintln!("Error restarting server: {}", e);
                                    }
                                });
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
