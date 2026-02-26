use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
  config::{AppConfig, OutputPolicy},
  AppState,
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
  pub paused: bool,
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

    let watch_folders = value
      .watch_folders
      .into_iter()
      .filter(|path| !path.trim().is_empty())
      .map(PathBuf::from)
      .collect();

    Ok(Self {
      watch_folders,
      recursive_watch: value.recursive_watch,
      output_policy: value.output_policy.into(),
      jpeg_quality: value.jpeg_quality,
      paused: value.paused,
    })
  }
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
pub fn update_config(config: AppConfigDto, state: State<'_, AppState>) -> Result<(), String> {
  let mut store = state
    .config_store
    .lock()
    .map_err(|err| format!("failed to lock config store: {err}"))?;

  let new_config = AppConfig::try_from(config)?;
  store.replace_config(new_config);
  store
    .save()
    .map_err(|err| format!("failed to save config: {err}"))
}

#[tauri::command]
pub fn set_paused(paused: bool, state: State<'_, AppState>) -> Result<(), String> {
  let mut store = state
    .config_store
    .lock()
    .map_err(|err| format!("failed to lock config store: {err}"))?;

  store.set_paused(paused);
  store
    .save()
    .map_err(|err| format!("failed to save config: {err}"))
}
