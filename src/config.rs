use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::workspace::expand_home_path;

/// Configuration key that stores the cc-switch SQLite database path.
pub const CC_SWITCH_DB: &str = "cc-switch-db";

const CONFIG_FILE_NAME: &str = "config.json";

/// A persisted user configuration entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    name: String,
    value: PathBuf,
}

impl ConfigEntry {
    /// Create a configuration entry.
    ///
    /// # Arguments
    ///
    /// * `name` - Supported configuration key.
    /// * `value` - Persisted configuration value.
    ///
    /// # Returns
    ///
    /// A configuration entry with owned fields.
    #[must_use]
    pub fn new(name: String, value: PathBuf) -> Self {
        Self { name, value }
    }

    /// Return the configuration key.
    ///
    /// # Returns
    ///
    /// The configuration key as a borrowed string slice.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the configuration value.
    ///
    /// # Returns
    ///
    /// The configuration value as a borrowed path.
    #[must_use]
    pub fn value(&self) -> &Path {
        &self.value
    }
}

/// Errors returned while reading or writing user configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The operating system did not expose a usable home directory.
    #[error("failed to resolve user home directory")]
    MissingHomeDirectory,

    /// The requested configuration key is not supported.
    #[error("unsupported config name '{name}'; supported config names: {supported}")]
    UnsupportedConfigName {
        /// User-provided configuration key.
        name: String,
        /// Comma-separated supported configuration keys.
        supported: &'static str,
    },

    /// The configuration file could not be read or written.
    #[error("configuration file error: {0}")]
    Io(#[from] std::io::Error),

    /// The configuration file contained invalid JSON.
    #[error("configuration JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The system clock cannot be used to create a unique temporary file name.
    #[error("system clock is before the Unix epoch")]
    InvalidSystemClock,
}

/// User-level codex-ws configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct UserConfig {
    #[serde(rename = "cc-switch-db", skip_serializing_if = "Option::is_none")]
    cc_switch_db: Option<PathBuf>,
}

impl UserConfig {
    /// Return the configured cc-switch database path.
    ///
    /// # Returns
    ///
    /// The configured path, if the user set `cc-switch-db`.
    #[must_use]
    pub fn cc_switch_db(&self) -> Option<&Path> {
        self.cc_switch_db.as_deref()
    }

    /// Set a supported configuration value.
    ///
    /// # Arguments
    ///
    /// * `name` - Supported configuration key.
    /// * `value` - Value to store for the key.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::UnsupportedConfigName`] when `name` is not supported.
    pub fn set_value(&mut self, name: &str, value: PathBuf) -> Result<(), ConfigError> {
        match parse_config_name(name)? {
            ConfigName::CcSwitchDbRoute => {
                self.cc_switch_db = Some(value);
                Ok(())
            }
        }
    }

    /// Return one supported configuration value.
    ///
    /// # Arguments
    ///
    /// * `name` - Supported configuration key.
    ///
    /// # Returns
    ///
    /// The configuration entry when the key is set.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::UnsupportedConfigName`] when `name` is not supported.
    pub fn get_value(&self, name: &str) -> Result<Option<ConfigEntry>, ConfigError> {
        match parse_config_name(name)? {
            ConfigName::CcSwitchDbRoute => Ok(self
                .cc_switch_db
                .as_ref()
                .map(|value| ConfigEntry::new(CC_SWITCH_DB.to_owned(), value.clone()))),
        }
    }

    /// Return all configured values.
    ///
    /// # Returns
    ///
    /// Configured entries in stable key order.
    #[must_use]
    pub fn entries(&self) -> Vec<ConfigEntry> {
        self.cc_switch_db
            .as_ref()
            .map(|value| ConfigEntry::new(CC_SWITCH_DB.to_owned(), value.clone()))
            .into_iter()
            .collect()
    }
}

/// Return the default codex-ws state root.
///
/// # Returns
///
/// The `.codex-ws` directory under the real user home directory.
///
/// # Errors
///
/// Returns [`ConfigError::MissingHomeDirectory`] when the OS does not expose a home directory.
pub fn default_state_root() -> Result<PathBuf, ConfigError> {
    Ok(home_dir()?.join(".codex-ws"))
}

/// Return the default user configuration directory.
///
/// # Returns
///
/// The `config` directory under the codex-ws state root.
///
/// # Errors
///
/// Returns [`ConfigError::MissingHomeDirectory`] when the OS does not expose a home directory.
pub fn default_config_dir() -> Result<PathBuf, ConfigError> {
    Ok(default_state_root()?.join("config"))
}

/// Return the default user configuration file path.
///
/// # Returns
///
/// The `config.json` path under the codex-ws config directory.
///
/// # Errors
///
/// Returns [`ConfigError::MissingHomeDirectory`] when the OS does not expose a home directory.
pub fn default_config_file_path() -> Result<PathBuf, ConfigError> {
    Ok(default_config_dir()?.join(CONFIG_FILE_NAME))
}

/// Return the fallback cc-switch database path.
///
/// # Returns
///
/// The legacy cc-switch database path under the user's home directory.
///
/// # Errors
///
/// Returns [`ConfigError::MissingHomeDirectory`] when the home directory cannot be resolved.
pub fn default_cc_switch_database_path() -> Result<PathBuf, ConfigError> {
    let default_path = home_dir()?.join(".cc-switch").join("cc-switch.db");
    #[cfg(windows)]
    {
        if !default_path.exists() {
            if let Ok(home_env) = std::env::var("HOME") {
                let trimmed = home_env.trim();
                if !trimmed.is_empty() {
                    let legacy_path = PathBuf::from(trimmed)
                        .join(".cc-switch")
                        .join("cc-switch.db");
                    if legacy_path.exists() {
                        return Ok(legacy_path);
                    }
                }
            }
        }
    }
    Ok(default_path)
}

/// Load user configuration from the default codex-ws config file.
///
/// # Returns
///
/// Parsed user configuration, or an empty configuration when the file does not exist.
///
/// # Errors
///
/// Returns an error when the config path cannot be resolved, the file cannot be read, or its JSON
/// is invalid.
pub fn load_default_user_config() -> Result<UserConfig, ConfigError> {
    load_user_config(&default_config_file_path()?)
}

/// Load user configuration from a file path.
///
/// # Arguments
///
/// * `path` - Configuration JSON path.
///
/// # Returns
///
/// Parsed user configuration, or an empty configuration when the file does not exist.
///
/// # Errors
///
/// Returns an error when the file cannot be read or its JSON is invalid.
pub fn load_user_config(path: &Path) -> Result<UserConfig, ConfigError> {
    if !path.exists() {
        return Ok(UserConfig::default());
    }

    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Save user configuration to a file path.
///
/// # Arguments
///
/// * `path` - Configuration JSON path.
/// * `config` - User configuration to persist.
///
/// # Errors
///
/// Returns an error when the parent directory cannot be created or the file cannot be written.
pub fn save_user_config(path: &Path, config: &UserConfig) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(config)?;
    atomic_write(path, content.as_bytes())?;
    Ok(())
}

/// Set a configuration value in the default codex-ws config file.
///
/// # Arguments
///
/// * `name` - Supported configuration key.
/// * `value` - Value to persist.
///
/// # Errors
///
/// Returns an error when the key is unsupported or the config file cannot be updated.
pub fn set_default_config_value(name: &str, value: PathBuf) -> Result<PathBuf, ConfigError> {
    let path = default_config_file_path()?;
    let mut config = load_user_config(&path)?;
    config.set_value(name, expand_home_path(value))?;
    save_user_config(&path, &config)?;
    Ok(path)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigName {
    CcSwitchDbRoute,
}

fn parse_config_name(name: &str) -> Result<ConfigName, ConfigError> {
    match name {
        CC_SWITCH_DB => Ok(ConfigName::CcSwitchDbRoute),
        _ => Err(ConfigError::UnsupportedConfigName {
            name: name.to_owned(),
            supported: CC_SWITCH_DB,
        }),
    }
}

fn home_dir() -> Result<PathBuf, ConfigError> {
    BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or(ConfigError::MissingHomeDirectory)
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), ConfigError> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid configuration path '{}'", path.display()),
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid configuration file name '{}'", path.display()),
        )
    })?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ConfigError::InvalidSystemClock)?
        .as_nanos();
    let temporary_path = parent.join(format!(
        "{}.tmp.{}-{timestamp}",
        file_name.to_string_lossy(),
        std::process::id()
    ));

    fs::write(&temporary_path, content)?;
    #[cfg(windows)]
    {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    fs::rename(temporary_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn user_config_sets_and_gets_cc_switch_database_path() {
        let mut config = UserConfig::default();

        config
            .set_value(CC_SWITCH_DB, PathBuf::from("/tmp/cc-switch.db"))
            .expect("supported config should set");

        let entry = config
            .get_value(CC_SWITCH_DB)
            .expect("supported config should get")
            .expect("entry should be present");
        assert_eq!(entry.name(), CC_SWITCH_DB);
        assert_eq!(entry.value(), Path::new("/tmp/cc-switch.db"));
    }

    #[test]
    fn user_config_rejects_unsupported_config_names() {
        let mut config = UserConfig::default();
        let error = config
            .set_value("unknown", PathBuf::from("value"))
            .expect_err("unsupported config should fail")
            .to_string();

        assert_eq!(
            error,
            "unsupported config name 'unknown'; supported config names: cc-switch-db"
        );
    }

    #[test]
    fn load_user_config_returns_default_when_file_is_missing() {
        let temp_dir = TestTempDir::create();
        let config = load_user_config(&temp_dir.path().join("missing.json"))
            .expect("missing config should load as default");

        assert_eq!(config, UserConfig::default());
    }

    #[test]
    fn save_user_config_creates_parent_directories() {
        let temp_dir = TestTempDir::create();
        let config_path = temp_dir.path().join("nested").join("config.json");
        let mut config = UserConfig::default();
        config
            .set_value(CC_SWITCH_DB, PathBuf::from("/tmp/cc-switch.db"))
            .expect("supported config should set");

        save_user_config(&config_path, &config).expect("config should save");
        let loaded = load_user_config(&config_path).expect("config should load");

        assert_eq!(loaded, config);
    }

    #[derive(Debug)]
    struct TestTempDir {
        path: PathBuf,
    }

    impl TestTempDir {
        fn create() -> Self {
            let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "codex-ws-config-test-{}-{timestamp}-{counter}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temporary test directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
