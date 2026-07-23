pub mod commands;
pub mod error;
pub mod model;
pub mod store;
pub mod tray;
pub mod tunnel;

use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::store::{paths, ConfigStore};
use crate::tunnel::{EngineEvent, TunnelManager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(|window, event| {
            // Close-to-tray when the user has kept "minimize to tray" on.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let minimize = window
                    .app_handle()
                    .state::<ConfigStore>()
                    .load()
                    .map(|config| config.minimize_to_tray)
                    .unwrap_or(true);
                if minimize {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(|app| {
            let dir = paths::app_config_dir()?;
            app.manage(ConfigStore::new(dir));
            tray::build_tray(app.handle())?;

            // Engine → UI event bridge. The engine pushes events into `tx`; this
            // task forwards them to the webview as Tauri events.
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<EngineEvent>();
            app.manage(TunnelManager::new(tx));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = match event {
                        EngineEvent::Status(status) => handle.emit("tunnel://status", status),
                        EngineEvent::Log(line) => handle.emit("tunnel://log", line),
                    };
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config_cmd::load_config,
            commands::config_cmd::save_config,
            commands::secret_cmd::set_secret,
            commands::secret_cmd::clear_secret,
            commands::tunnel_cmd::start_tunnel,
            commands::tunnel_cmd::stop_tunnel,
            commands::tunnel_cmd::start_all_tunnels,
            commands::tunnel_cmd::stop_all_tunnels,
            commands::tunnel_cmd::running_tunnels,
            commands::tunnel_cmd::forget_host_key,
            commands::system_cmd::list_network_interfaces,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
