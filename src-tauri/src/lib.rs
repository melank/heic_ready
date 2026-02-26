mod commands;
mod config;

use std::sync::Mutex;

use commands::{get_config, set_paused, update_config};
use config::ConfigStore;
use tauri::{
  menu::{Menu, MenuItem, PredefinedMenuItem},
  tray::TrayIconBuilder,
  AppHandle, Emitter, Manager, State, Wry,
};

const TRAY_ID: &str = "main-tray";
const MENU_STATUS_ID: &str = "status";
const MENU_TOGGLE_ID: &str = "pause_resume";
const MENU_SETTINGS_ID: &str = "settings";
const MENU_QUIT_ID: &str = "quit";
const EVENT_PAUSED_CHANGED: &str = "paused-changed";

pub(crate) struct AppState {
  pub(crate) config_store: Mutex<ConfigStore>,
}

fn build_tray_menu(app: &AppHandle, paused: bool) -> tauri::Result<Menu<Wry>> {
  let status_text = if paused { "Paused" } else { "Running" };
  let toggle_text = if paused { "Resume" } else { "Pause" };

  let status = MenuItem::with_id(app, MENU_STATUS_ID, status_text, false, None::<&str>)?;
  let toggle = MenuItem::with_id(app, MENU_TOGGLE_ID, toggle_text, true, None::<&str>)?;
  let settings = MenuItem::with_id(app, MENU_SETTINGS_ID, "Settings", true, None::<&str>)?;
  let quit = MenuItem::with_id(app, MENU_QUIT_ID, "Quit", true, None::<&str>)?;
  let separator = PredefinedMenuItem::separator(app)?;

  Menu::with_items(app, &[&status, &separator, &toggle, &settings, &quit])
}

fn show_settings_window(app: &AppHandle) {
  if let Some(window) = app.get_webview_window("main") {
    if let Err(err) = window.show() {
      log::error!("failed to show settings window: {err}");
      return;
    }
    if let Err(err) = window.set_focus() {
      log::error!("failed to focus settings window: {err}");
    }
  }
}

fn set_paused_and_refresh_ui(app: &AppHandle, paused: bool) {
  let state: State<'_, AppState> = app.state();
  let mut config_store = match state.config_store.lock() {
    Ok(guard) => guard,
    Err(err) => {
      log::error!("failed to lock app state: {err}");
      return;
    }
  };

  config_store.set_paused(paused);
  if let Err(err) = config_store.save() {
    log::error!("failed to save config: {err}");
    return;
  }

  if let Some(tray) = app.tray_by_id(TRAY_ID) {
    match build_tray_menu(app, paused) {
      Ok(menu) => {
        if let Err(err) = tray.set_menu(Some(menu)) {
          log::error!("failed to update tray menu: {err}");
        }
      }
      Err(err) => {
        log::error!("failed to rebuild tray menu: {err}");
      }
    }
  }

  if let Err(err) = app.emit(EVENT_PAUSED_CHANGED, paused) {
    log::error!("failed to emit paused change event: {err}");
  }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![get_config, update_config, set_paused])
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
      log::info!("config loaded from {}", config_store.config_path().display());

      app.manage(AppState {
        config_store: Mutex::new(config_store),
      });

      if let Some(window) = app.get_webview_window("main") {
        window.hide()?;
        let settings_window = window.clone();
        window.on_window_event(move |event| {
          if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(err) = settings_window.hide() {
              log::error!("failed to hide settings window: {err}");
            }
          }
        });
      }

      let menu = build_tray_menu(&app.handle(), paused)?;
      let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("heic_ready");

      if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
      } else {
        log::warn!("default window icon is unavailable; tray icon may be hidden");
      }

      tray_builder
        .on_menu_event(|app, event| match event.id().as_ref() {
          MENU_TOGGLE_ID => {
            let state: State<'_, AppState> = app.state();
            let paused = match state.config_store.lock() {
              Ok(store) => !store.config().paused,
              Err(err) => {
                log::error!("failed to lock app state: {err}");
                return;
              }
            };
            set_paused_and_refresh_ui(app, paused);
          }
          MENU_SETTINGS_ID => show_settings_window(app),
          MENU_QUIT_ID => app.exit(0),
          _ => {}
        })
        .build(app)?;

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
