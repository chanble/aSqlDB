#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::Manager;

struct SidecarChild(Mutex<Option<tauri_plugin_shell::process::CommandChild>>);

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(SidecarChild(Mutex::new(None)))
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(not(debug_assertions))]
            {
                tauri::async_runtime::spawn(async move {
                    use tauri_plugin_shell::ShellExt;
                    use tauri_plugin_shell::process::CommandEvent;

                    let shell = handle.shell();
                    let (mut rx, child) = shell
                        .sidecar("asql-web")
                        .expect("failed to create sidecar")
                        .args(["-p", "0"])
                        .spawn()
                        .expect("failed to spawn sidecar");

                    // Store child so it can be killed on app exit
                    if let Some(state) = handle.try_state::<SidecarChild>() {
                        *state.0.lock().unwrap() = Some(child);
                    }

                    while let Some(event) = rx.recv().await {
                        if let CommandEvent::Stdout(line) = event {
                            let s = String::from_utf8_lossy(&line).trim().to_string();
                            if let Ok(port) = s.parse::<u16>() {
                                let url = format!("http://localhost:{}", port);
                                if let Some(window) = handle.get_webview_window("main") {
                                    let _ = window.navigate(url.parse().unwrap());
                                    let _ = window.show();
                                }
                                break;
                            }
                        }
                    }
                });
            }

            #[cfg(debug_assertions)]
            {
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.app_handle().try_state::<SidecarChild>() {
                    if let Some(child) = state.0.lock().unwrap().take() {
                        let _ = child.kill();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
