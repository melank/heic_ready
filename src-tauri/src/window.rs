use tauri::{AppHandle, Manager};

pub(crate) fn show_settings_window(app: &AppHandle) {
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

pub(crate) fn setup_main_window(app: &AppHandle) -> tauri::Result<()> {
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
    Ok(())
}
