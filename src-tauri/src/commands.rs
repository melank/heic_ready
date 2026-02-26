use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::{
    config::{AppConfig, OutputPolicy},
    watcher,
    restart_watch_service, AppState, EVENT_PAUSED_CHANGED,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputPolicyDto {
    Coexist,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigDto {
    pub watch_folders: Vec<String>,
    pub recursive_watch: bool,
    pub output_policy: OutputPolicyDto,
    pub jpeg_quality: u8,
    pub rescan_interval_secs: u64,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigResult {
    pub config: AppConfigDto,
    pub warning: Option<String>,
}

impl From<OutputPolicy> for OutputPolicyDto {
    fn from(value: OutputPolicy) -> Self {
        match value {
            OutputPolicy::Coexist => Self::Coexist,
            OutputPolicy::Replace => Self::Replace,
        }
    }
}

impl From<OutputPolicyDto> for OutputPolicy {
    fn from(value: OutputPolicyDto) -> Self {
        match value {
            OutputPolicyDto::Coexist => Self::Coexist,
            OutputPolicyDto::Replace => Self::Replace,
        }
    }
}

impl From<AppConfig> for AppConfigDto {
    fn from(value: AppConfig) -> Self {
        Self {
            watch_folders: value
                .watch_folders
                .into_iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            recursive_watch: value.recursive_watch,
            output_policy: value.output_policy.into(),
            jpeg_quality: value.jpeg_quality,
            rescan_interval_secs: value.rescan_interval_secs,
            paused: value.paused,
        }
    }
}

impl TryFrom<AppConfigDto> for AppConfig {
    type Error = String;

    fn try_from(value: AppConfigDto) -> Result<Self, Self::Error> {
        if value.jpeg_quality > 100 {
            return Err("jpeg_quality must be in range 0..=100".to_string());
        }
        if value.rescan_interval_secs < 15 || value.rescan_interval_secs > 3600 {
            return Err("rescan_interval_secs must be in range 15..=3600".to_string());
        }

        let mut watch_folders = Vec::new();
        let mut seen = HashSet::new();
        for raw in value.watch_folders {
            let Some(path) = normalize_watch_folder_path(&raw)? else {
                continue;
            };
            if seen.insert(path.clone()) {
                watch_folders.push(path);
            }
        }

        Ok(Self {
            watch_folders,
            recursive_watch: value.recursive_watch,
            output_policy: value.output_policy.into(),
            jpeg_quality: value.jpeg_quality,
            rescan_interval_secs: value.rescan_interval_secs,
            paused: value.paused,
        })
    }
}

fn normalize_watch_folder_path(raw: &str) -> Result<Option<PathBuf>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let normalized: PathBuf = Path::new(trimmed).components().collect();
    if !normalized.is_absolute() {
        return Err(format!("watch folder must be absolute: {trimmed}"));
    }
    Ok(Some(normalized))
}

#[tauri::command]
pub fn get_recent_logs() -> Vec<watcher::RecentLog> {
    watcher::get_recent_logs()
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<AppConfigDto, String> {
    let store = state
        .config_store
        .lock()
        .map_err(|err| format!("failed to lock config store: {err}"))?;

    Ok(store.config().clone().into())
}

#[tauri::command]
pub fn update_config(
    config: AppConfigDto,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<UpdateConfigResult, String> {
    let mut store = state
        .config_store
        .lock()
        .map_err(|err| format!("failed to lock config store: {err}"))?;

    let (new_config, warning) = apply_replace_permission_policy(AppConfig::try_from(config)?);

    store.replace_config(new_config);
    store
        .save()
        .map_err(|err| format!("failed to save config: {err}"))?;
    let paused = store.config().paused;
    drop(store);

    restart_watch_service(&app)?;
    app.emit(EVENT_PAUSED_CHANGED, paused)
        .map_err(|err| format!("failed to emit pause event: {err}"))?;

    if warning.is_some() {
        watcher::push_recent_info("replace unavailable; fallback to coexist");
    }

    let result_config = store_config_to_dto(state)?;
    Ok(UpdateConfigResult {
        config: result_config,
        warning,
    })
}

#[tauri::command]
pub fn set_paused(paused: bool, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let mut store = state
        .config_store
        .lock()
        .map_err(|err| format!("failed to lock config store: {err}"))?;

    store.set_paused(paused);
    store
        .save()
        .map_err(|err| format!("failed to save config: {err}"))?;
    drop(store);

    restart_watch_service(&app)?;
    app.emit(EVENT_PAUSED_CHANGED, paused)
        .map_err(|err| format!("failed to emit pause event: {err}"))?;

    Ok(())
}

#[tauri::command]
pub fn pick_watch_folder() -> Result<Option<String>, String> {
    let script = r#"try
POSIX path of (choose folder with prompt "Select watch folder for heic_ready")
on error number -128
return ""
end try"#;

    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|err| format!("failed to open folder picker: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("folder picker failed: {stderr}"));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Ok(None);
    }

    let Some(normalized) = normalize_watch_folder_path(&path)? else {
        return Ok(None);
    };
    Ok(Some(normalized.to_string_lossy().into_owned()))
}

fn store_config_to_dto(state: State<'_, AppState>) -> Result<AppConfigDto, String> {
    let store = state
        .config_store
        .lock()
        .map_err(|err| format!("failed to lock config store: {err}"))?;
    Ok(store.config().clone().into())
}

fn verify_replace_permissions(watch_folders: &[PathBuf]) -> Result<(), String> {
    let mut issues = Vec::new();

    let trash = user_trash_dir()?;
    if let Err(err) = verify_writable_dir(&trash, "trash") {
        issues.push(err);
    }
    for folder in watch_folders {
        if let Err(err) = verify_writable_dir(folder, "watch folder") {
            issues.push(err);
        }
    }

    if !issues.is_empty() {
        return Err(issues.join("\n"));
    }
    Ok(())
}

fn apply_replace_permission_policy(mut config: AppConfig) -> (AppConfig, Option<String>) {
    if matches!(config.output_policy, OutputPolicy::Replace) {
        if let Err(err) = verify_replace_permissions(&config.watch_folders) {
            config.output_policy = OutputPolicy::Coexist;
            return (config, Some(format!("Replace unavailable\n{err}\nFallback: coexist")));
        }
    }
    (config, None)
}

fn user_trash_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".Trash"))
}

fn verify_writable_dir(dir: &Path, label: &str) -> Result<(), String> {
    if !dir.exists() {
        return Err(format!("{label}: missing ({})", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("{label}: not directory ({})", dir.display()));
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(".heic_ready_perm_{}_{}", std::process::id(), stamp));
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp)
        .map_err(|err| format!("{label}: not writable ({}) - {err}", dir.display()))?;
    fs::remove_file(&tmp).map_err(|err| {
        format!("{label}: probe cleanup failed ({}) - {err}", tmp.display())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_policy_falls_back_to_coexist_on_permission_probe_failure() {
        let config = AppConfig {
            watch_folders: vec![PathBuf::from("/path/does/not/exist")],
            recursive_watch: false,
            output_policy: OutputPolicy::Replace,
            jpeg_quality: 92,
            rescan_interval_secs: 60,
            paused: false,
        };

        let (updated, warning) = apply_replace_permission_policy(config);
        assert!(matches!(updated.output_policy, OutputPolicy::Coexist));
        assert!(warning.is_some());
    }

    #[test]
    fn coexist_policy_is_unchanged() {
        let config = AppConfig {
            watch_folders: vec![PathBuf::from("/path/does/not/exist")],
            recursive_watch: false,
            output_policy: OutputPolicy::Coexist,
            jpeg_quality: 92,
            rescan_interval_secs: 60,
            paused: false,
        };

        let (updated, warning) = apply_replace_permission_policy(config.clone());
        assert!(matches!(updated.output_policy, OutputPolicy::Coexist));
        assert!(warning.is_none());
    }

    #[test]
    fn normalize_watch_folder_path_trims_and_removes_trailing_separator() {
        let path = normalize_watch_folder_path(" /tmp/heic_ready_perm_test/ ")
            .expect("normalize")
            .expect("path");
        assert_eq!(path, PathBuf::from("/tmp/heic_ready_perm_test"));
    }

    #[test]
    fn normalize_watch_folder_path_rejects_relative_path() {
        let err = normalize_watch_folder_path("tmp/heic_ready").expect_err("must fail");
        assert!(err.contains("must be absolute"));
    }
}
