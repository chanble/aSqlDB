#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(not(debug_assertions))]
            {
                tauri::async_runtime::spawn(async move {
                    use tauri_plugin_shell::ShellExt;
                    use tauri_plugin_shell::process::CommandEvent;
                    use futures::StreamExt;

                    let shell = handle.shell();
                    let (mut rx, _child) = shell
                        .sidecar("asql-web")
                        .expect("failed to create sidecar")
                        .args(["-p", "0"])
                        .spawn()
                        .expect("failed to spawn sidecar");

                    while let Some(event) = rx.next().await {
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
