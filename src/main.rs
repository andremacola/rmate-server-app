use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;

use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::TrayIconBuilder;
use winit::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

struct AppState {
    server: Option<Child>,
    editor: Editor,
}

fn load_icon(base_path: &PathBuf, icon_name: &str) -> tray_icon::Icon {
    let path = base_path.join("icons").join(icon_name);
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(&path)
            .unwrap_or_else(|e| panic!("Failed to open icon at {:?}: {}", path, e))
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

fn start_server(state: &mut AppState, resources_path: &PathBuf) {
    if state.server.is_some() {
        return;
    }

    let editor_path = state.editor.to_bin_path();
    let server_path = resources_path.join("bin").join("zed-rmate-server");

    match Command::new(&server_path)
        .arg("--zed-bin")
        .arg(editor_path)
        .spawn()
    {
        Ok(child) => {
            state.server = Some(child);
            println!("Server started for {:?}", state.editor);
        }
        Err(e) => {
            eprintln!("Failed to start server for {}: {}", editor_path, e);
        }
    }
}

fn stop_server(state: &mut AppState) {
    if let Some(mut child) = state.server.take() {
        if let Err(e) = child.kill() {
            eprintln!("Failed to kill server process: {}", e);
        } else {
            println!("Server stopped.");
        }
    }
}

fn main() {
    env_logger::init();
    let mut event_loop_builder = EventLoopBuilder::new();

    #[cfg(target_os = "macos")]
    event_loop_builder.with_activation_policy(winit::platform::macos::ActivationPolicy::Accessory);

    let event_loop = event_loop_builder.build().unwrap();

    let resources_path = {
        if let Ok(exe_path) = std::env::current_exe() {
            if cfg!(target_os = "macos") && exe_path.to_string_lossy().contains(".app/") {
                exe_path
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("Resources"))
                    .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            }
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        }
    };

    let icon_on = load_icon(&resources_path, "icon.png");
    let icon_off = load_icon(&resources_path, "icon-off.png");

    // App state shared between threads
    let app_state = Arc::new(Mutex::new(AppState {
        server: None,
        editor: Editor::Zed,
    }));

    // --- Menu setup ---
    let menu = Menu::new();

    let toggle_server_mi = MenuItem::new("Start Server", true, None);
    menu.append_items(&[&toggle_server_mi, &PredefinedMenuItem::separator()])
        .unwrap();

    let zed_mi = CheckMenuItem::new("Zed", true, true, None);
    let vscode_mi = CheckMenuItem::new("VS Code", true, false, None);
    let sublime_mi = CheckMenuItem::new("Sublime Text", true, false, None);
    menu.append_items(&[
        &zed_mi,
        &vscode_mi,
        &sublime_mi,
        &PredefinedMenuItem::separator(),
    ])
    .unwrap();

    let quit_mi = MenuItem::new("Quit", true, None);
    menu.append(&quit_mi).unwrap();
    // --- End of menu setup ---

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon_off.clone())
        .with_icon_as_template(true)
        .with_tooltip("RMate Server")
        .build()
        .unwrap();

    // Start server on launch
    {
        let mut state = app_state.lock().unwrap();
        start_server(&mut state, &resources_path);
        if state.server.is_some() {
            toggle_server_mi.set_text("Stop Server");
            tray_icon.set_icon(Some(icon_on.clone())).unwrap();
        }
    }

    let menu_channel = tray_icon::menu::MenuEvent::receiver();

    let _ = event_loop.run(move |_event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Wait);

        if let Ok(event) = menu_channel.try_recv() {
            let mut state = app_state.lock().unwrap();

            if event.id == toggle_server_mi.id() {
                if state.server.is_some() {
                    stop_server(&mut state);
                    toggle_server_mi.set_text("Start Server");
                    tray_icon.set_icon(Some(icon_off.clone())).unwrap();
                } else {
                    start_server(&mut state, &resources_path);
                    toggle_server_mi.set_text("Stop Server");
                    tray_icon.set_icon(Some(icon_on.clone())).unwrap();
                }
            } else if event.id == zed_mi.id()
                || event.id == vscode_mi.id()
                || event.id == sublime_mi.id()
            {
                let new_editor = if event.id == zed_mi.id() {
                    Editor::Zed
                } else if event.id == vscode_mi.id() {
                    Editor::Vscode
                } else {
                    Editor::Sublime
                };

                if state.editor != new_editor {
                    let was_running = state.server.is_some();
                    if was_running {
                        stop_server(&mut state);
                    }

                    // Uncheck old editor
                    match state.editor {
                        Editor::Zed => zed_mi.set_checked(false),
                        Editor::Vscode => vscode_mi.set_checked(false),
                        Editor::Sublime => sublime_mi.set_checked(false),
                    }

                    // Update state and check new editor
                    state.editor = new_editor;
                    match new_editor {
                        Editor::Zed => zed_mi.set_checked(true),
                        Editor::Vscode => vscode_mi.set_checked(true),
                        Editor::Sublime => sublime_mi.set_checked(true),
                    }
                    println!("Switched editor to {:?}", new_editor);

                    if was_running {
                        // Give the port a moment to be released
                        thread::sleep(std::time::Duration::from_millis(500));
                        start_server(&mut state, &resources_path);
                    }
                } else {
                    // If the user clicks the already selected editor, re-check it
                    match state.editor {
                        Editor::Zed => zed_mi.set_checked(true),
                        Editor::Vscode => vscode_mi.set_checked(true),
                        Editor::Sublime => sublime_mi.set_checked(true),
                    }
                }
            } else if event.id == quit_mi.id() {
                stop_server(&mut state);
                event_loop.exit();
            }
        }
    });
}
