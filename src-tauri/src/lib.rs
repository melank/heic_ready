mod commands;
mod config;
mod tray;
mod watcher;
mod window;

use std::sync::Mutex;

use commands::{
    get_config, get_recent_logs, open_recent_logs_window, pick_watch_folder, set_paused,
    update_config,
};
use config::{AppConfig, ConfigStore};
use tauri::{AppHandle, Manager, State};
use watcher::WatchService;

pub(crate) const EVENT_PAUSED_CHANGED: &str = "paused-changed";
pub(crate) const EVENT_LOCALE_CHANGED: &str = "locale-changed";

pub(crate) struct AppState {
    pub(crate) config_store: Mutex<ConfigStore>,
    pub(crate) watch_service: Mutex<Option<WatchService>>,
}

pub(crate) fn restart_watch_service(app: &AppHandle) -> Result<(), String> {
    let state: State<'_, AppState> = app.state();

    let config = {
        let config_store = state
            .config_store
            .lock()
            .map_err(|err| format!("failed to lock config store: {err}"))?;
        config_store.config().clone()
    };

    let mut watch_slot = state
        .watch_service
        .lock()
        .map_err(|err| format!("failed to lock watch service: {err}"))?;

    if let Some(existing) = watch_slot.take() {
        existing.stop();
    }

    if should_start_watcher(&config) {
        let service = WatchService::start(config)?;
        *watch_slot = Some(service);
    }

    Ok(())
}

fn should_start_watcher(config: &AppConfig) -> bool {
    !config.paused && !config.watch_folders.is_empty()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_recent_logs,
            open_recent_logs_window,
            pick_watch_folder,
            update_config,
            set_paused
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let config_dir = app.path().app_config_dir()?;
            let config_store = ConfigStore::load_or_init(&config_dir)?;
            let paused = config_store.config().paused;
            let locale = config_store.config().locale;
            log::info!(
                "config loaded from {}",
                config_store.config_path().display()
            );

            app.manage(AppState {
                config_store: Mutex::new(config_store),
                watch_service: Mutex::new(None),
            });

            if let Err(err) = restart_watch_service(&app.handle()) {
                log::error!("failed to start watch service: {err}");
            }

            window::setup_main_window(&app.handle())?;
            tray::setup_tray(&app.handle(), paused, locale)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
