use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const APP_CONFIG_SUBDIR: &str = "heic-ready";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputPolicy {
    Coexist,
    Replace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppLocale {
    En,
    Ja,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub watch_folders: Vec<PathBuf>,
    pub recursive_watch: bool,
    pub output_policy: OutputPolicy,
    pub jpeg_quality: u8,
    #[serde(default = "default_rescan_interval_secs")]
    pub rescan_interval_secs: u64,
    pub paused: bool,
    #[serde(default = "default_locale")]
    pub locale: AppLocale,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watch_folders: Vec::new(),
            recursive_watch: false,
            output_policy: OutputPolicy::Coexist,
            jpeg_quality: 92,
            rescan_interval_secs: default_rescan_interval_secs(),
            paused: false,
            locale: default_locale(),
        }
    }
}

const fn default_rescan_interval_secs() -> u64 {
    60
}

const fn default_locale() -> AppLocale {
    AppLocale::En
}

pub struct ConfigStore {
    path: PathBuf,
    config: AppConfig,
}

impl ConfigStore {
    pub fn load_or_init(app_config_dir: &Path) -> io::Result<Self> {
        let path = config_file_path(app_config_dir);
        if !path.exists() {
            let mut store = Self {
                path,
                config: AppConfig::default(),
            };
            store.save()?;
            return Ok(store);
        }

        let contents = fs::read_to_string(&path)?;
        let config = match serde_json::from_str::<AppConfig>(&contents) {
            Ok(config) => config,
            Err(err) => {
                log::warn!("failed to parse config at {}: {err}", path.display());
                let mut store = Self {
                    path,
                    config: AppConfig::default(),
                };
                store.save()?;
                return Ok(store);
            }
        };

        Ok(Self { path, config })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn replace_config(&mut self, config: AppConfig) {
        self.config = config;
    }

    pub fn config_path(&self) -> &Path {
        &self.path
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.config.paused = paused;
    }

    pub fn set_locale(&mut self, locale: AppLocale) {
        self.config.locale = locale;
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized = serde_json::to_vec_pretty(&self.config)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        atomic_write(&self.path, &serialized)
    }
}

fn config_file_path(app_config_dir: &Path) -> PathBuf {
    app_config_dir
        .join(APP_CONFIG_SUBDIR)
        .join(CONFIG_FILE_NAME)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp_path = tmp_path_for(path);
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

fn tmp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.json".to_string());
    path.with_file_name(format!("{file_name}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn test_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let seq = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "heic-ready-config-test-{}_{}_{}",
            std::process::id(),
            nanos,
            seq
        ));
        fs::create_dir_all(&path).expect("create temp root");
        path
    }

    #[test]
    fn creates_default_config_on_first_load() {
        let root = test_root();
        let store = ConfigStore::load_or_init(&root).expect("load config");

        assert_eq!(store.config(), &AppConfig::default());
        assert!(store.config_path().exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reads_existing_config() {
        let root = test_root();
        let config_dir = root.join(APP_CONFIG_SUBDIR);
        fs::create_dir_all(&config_dir).expect("create dir");
        let path = config_dir.join(CONFIG_FILE_NAME);
        let expected = AppConfig {
            watch_folders: vec![PathBuf::from("/tmp/drop")],
            recursive_watch: true,
            output_policy: OutputPolicy::Replace,
            jpeg_quality: 88,
            rescan_interval_secs: 120,
            paused: true,
            locale: AppLocale::Ja,
        };
        fs::write(
            &path,
            serde_json::to_vec_pretty(&expected).expect("serialize"),
        )
        .expect("write config");

        let store = ConfigStore::load_or_init(&root).expect("load config");
        assert_eq!(store.config(), &expected);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_default_when_config_is_invalid_json() {
        let root = test_root();
        let config_dir = root.join(APP_CONFIG_SUBDIR);
        fs::create_dir_all(&config_dir).expect("create dir");
        let path = config_dir.join(CONFIG_FILE_NAME);
        fs::write(&path, b"{ this is invalid json ").expect("write bad config");

        let store = ConfigStore::load_or_init(&root).expect("load config");

        assert_eq!(store.config(), &AppConfig::default());
        let content = fs::read_to_string(store.config_path()).expect("read rewritten config");
        assert!(content.contains("\"output_policy\": \"coexist\""));
        assert!(content.contains("\"rescan_interval_secs\": 60"));
        assert!(content.contains("\"locale\": \"en\""));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn save_uses_tmp_then_rename() {
        let root = test_root();
        let mut store = ConfigStore::load_or_init(&root).expect("load config");
        store.set_paused(true);
        store.save().expect("save config");

        let tmp_path = tmp_path_for(store.config_path());
        assert!(!tmp_path.exists());

        let content = fs::read_to_string(store.config_path()).expect("read config");
        assert!(content.contains("\"paused\": true"));
        let _ = fs::remove_dir_all(root);
    }

}
