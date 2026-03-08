use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, Wry,
};

use crate::{
    commands::{open_recent_logs_window, LocaleDto},
    config::AppLocale,
    restart_watch_service,
    window::show_settings_window,
    AppState, EVENT_LOCALE_CHANGED, EVENT_PAUSED_CHANGED,
};

const TRAY_ID: &str = "main-tray";
const MENU_STATUS_ID: &str = "status";
const MENU_TOGGLE_ID: &str = "pause_resume";
const MENU_SETTINGS_GROUP_ID: &str = "settings_group";
const MENU_OPEN_SETTINGS_ID: &str = "open_settings";
const MENU_LANGUAGE_GROUP_ID: &str = "language_group";
const MENU_RECENT_LOGS_ID: &str = "recent_logs";
const MENU_LANG_EN_ID: &str = "lang_en";
const MENU_LANG_JA_ID: &str = "lang_ja";
const MENU_QUIT_ID: &str = "quit";

fn build_tray_menu(app: &AppHandle, paused: bool, locale: AppLocale) -> tauri::Result<Menu<Wry>> {
    let status_text = match (paused, locale) {
        (true, AppLocale::En) => "🔴 Paused",
        (true, AppLocale::Ja) => "🔴 Paused",
        (false, AppLocale::En) => "🟢 Ready",
        (false, AppLocale::Ja) => "🟢 Ready",
    };
    let toggle_text = match (paused, locale) {
        (true, AppLocale::En) => "Resume",
        (true, AppLocale::Ja) => "再開",
        (false, AppLocale::En) => "Pause",
        (false, AppLocale::Ja) => "一時停止",
    };
    let settings_text = match locale {
        AppLocale::En => "Settings",
        AppLocale::Ja => "設定",
    };
    let open_settings_text = match locale {
        AppLocale::En => "Open Settings",
        AppLocale::Ja => "設定を開く",
    };
    let recent_logs_text = match locale {
        AppLocale::En => "Recent Logs",
        AppLocale::Ja => "最近のログ",
    };
    let language_text = match locale {
        AppLocale::En => "Language",
        AppLocale::Ja => "言語",
    };
    let lang_en_text = match locale {
        AppLocale::En => "✓ Language: English",
        AppLocale::Ja => "✓ 言語: English",
    };
    let lang_ja_text = match locale {
        AppLocale::En => "Language: Japanese",
        AppLocale::Ja => "言語: 日本語",
    };
    let lang_en_alt_text = match locale {
        AppLocale::En => "Language: English",
        AppLocale::Ja => "言語: English",
    };
    let lang_ja_alt_text = match locale {
        AppLocale::En => "✓ Language: Japanese",
        AppLocale::Ja => "✓ 言語: 日本語",
    };
    let quit_text = match locale {
        AppLocale::En => "Quit",
        AppLocale::Ja => "終了",
    };

    let status = MenuItem::with_id(app, MENU_STATUS_ID, status_text, false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, MENU_TOGGLE_ID, toggle_text, true, None::<&str>)?;
    let open_settings = MenuItem::with_id(
        app,
        MENU_OPEN_SETTINGS_ID,
        open_settings_text,
        true,
        None::<&str>,
    )?;
    let recent_logs =
        MenuItem::with_id(app, MENU_RECENT_LOGS_ID, recent_logs_text, true, None::<&str>)?;
    let lang_en = MenuItem::with_id(
        app,
        MENU_LANG_EN_ID,
        if matches!(locale, AppLocale::En) {
            lang_en_text
        } else {
            lang_en_alt_text
        },
        true,
        None::<&str>,
    )?;
    let lang_ja = MenuItem::with_id(
        app,
        MENU_LANG_JA_ID,
        if matches!(locale, AppLocale::Ja) {
            lang_ja_alt_text
        } else {
            lang_ja_text
        },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT_ID, quit_text, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let language_menu = Submenu::with_id_and_items(
        app,
        MENU_LANGUAGE_GROUP_ID,
        language_text,
        true,
        &[&lang_en, &lang_ja],
    )?;
    let settings_menu = Submenu::with_id_and_items(
        app,
        MENU_SETTINGS_GROUP_ID,
        settings_text,
        true,
        &[&open_settings, &language_menu],
    )?;

    Menu::with_items(
        app,
        &[&status, &separator, &toggle, &settings_menu, &recent_logs, &quit],
    )
}

pub(crate) fn setup_tray(app: &AppHandle, paused: bool, locale: AppLocale) -> tauri::Result<()> {
    let menu = build_tray_menu(app, paused, locale)?;
    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .icon_as_template(true)
        .tooltip("HEIC Ready");

    {
        let tray_icon_bytes = include_bytes!("../icons/tray-icon.png");
        match tauri::image::Image::from_bytes(tray_icon_bytes) {
            Ok(icon) => {
                tray_builder = tray_builder.icon(icon);
            }
            Err(err) => {
                log::warn!("failed to load tray icon: {err}");
                if let Some(icon) = app.default_window_icon().cloned() {
                    tray_builder = tray_builder.icon(icon);
                }
            }
        }
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
            MENU_OPEN_SETTINGS_ID => show_settings_window(app),
            MENU_RECENT_LOGS_ID => {
                if let Err(err) = open_recent_logs_window(app.clone()) {
                    log::error!("failed to open recent logs window: {err}");
                }
            }
            MENU_LANG_EN_ID => set_locale_and_refresh_ui(app, AppLocale::En),
            MENU_LANG_JA_ID => set_locale_and_refresh_ui(app, AppLocale::Ja),
            MENU_QUIT_ID => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
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
    let locale = config_store.config().locale;
    if let Err(err) = config_store.save() {
        log::error!("failed to save config: {err}");
        return;
    }
    drop(config_store);

    if let Err(err) = restart_watch_service(app) {
        log::error!("failed to restart watch service: {err}");
    }

    refresh_tray_menu(app, paused, locale);

    if let Err(err) = app.emit(EVENT_PAUSED_CHANGED, paused) {
        log::error!("failed to emit paused change event: {err}");
    }
}

fn set_locale_and_refresh_ui(app: &AppHandle, locale: AppLocale) {
    let state: State<'_, AppState> = app.state();
    let mut config_store = match state.config_store.lock() {
        Ok(guard) => guard,
        Err(err) => {
            log::error!("failed to lock app state: {err}");
            return;
        }
    };

    if config_store.config().locale == locale {
        return;
    }

    config_store.set_locale(locale);
    let paused = config_store.config().paused;
    if let Err(err) = config_store.save() {
        log::error!("failed to save config: {err}");
        return;
    }
    drop(config_store);

    refresh_tray_menu(app, paused, locale);

    if let Err(err) = app.emit(EVENT_LOCALE_CHANGED, LocaleDto::from(locale)) {
        log::error!("failed to emit locale change event: {err}");
    }
}

fn refresh_tray_menu(app: &AppHandle, paused: bool, locale: AppLocale) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        match build_tray_menu(app, paused, locale) {
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
}
