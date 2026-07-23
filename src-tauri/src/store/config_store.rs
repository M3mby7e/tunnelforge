use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::{validate_app_config, AppConfig};

const CONFIG_FILE: &str = "config.json";
const TEMP_FILE: &str = "config.json.tmp";

/// Reads and writes the application config as JSON in a directory.
///
/// Writes are atomic (temp file + rename) so a crash mid-write never leaves a
/// half-written config. The store is intentionally decoupled from Tauri so it
/// can be unit-tested against a temp directory.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    dir: PathBuf,
}

impl ConfigStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn config_path(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE)
    }

    /// Load the config, or return defaults if none exists yet.
    pub fn load(&self) -> Result<AppConfig> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(AppConfig::default());
        }
        let data = fs::read_to_string(&path)?;
        let config = serde_json::from_str(&data)?;
        Ok(config)
    }

    /// Validate and atomically persist the config.
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        validate_app_config(config).map_err(Error::Validation)?;
        fs::create_dir_all(&self.dir)?;

        let tmp = self.dir.join(TEMP_FILE);
        let data = serde_json::to_string_pretty(config)?;
        write_then_rename(&tmp, &self.config_path(), &data)?;
        Ok(())
    }
}

fn write_then_rename(tmp: &Path, final_path: &Path, data: &str) -> Result<()> {
    fs::write(tmp, data)?;
    fs::rename(tmp, final_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ForwardKind, ThemePref};
    use tempfile::tempdir;

    #[test]
    fn load_returns_defaults_when_missing() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());
        let config = store.load().unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());

        let config = AppConfig {
            theme: ThemePref::Dark,
            start_on_boot: true,
            ..AppConfig::default()
        };

        store.save(&config).unwrap();
        assert!(store.config_path().exists());

        let loaded = store.load().unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn save_creates_missing_directories() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        let store = ConfigStore::new(&nested);
        store.save(&AppConfig::default()).unwrap();
        assert!(nested.join("config.json").exists());
    }

    #[test]
    fn save_rejects_invalid_config() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());

        let now = chrono::Utc::now();
        let bad = TunnelConfigFixture::invalid(now);
        let mut config = AppConfig::default();
        config.tunnels.push(bad);

        let err = store.save(&config).unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
        // Nothing should have been written.
        assert!(!store.config_path().exists());
    }

    // Small fixture builder kept local to the test module.
    struct TunnelConfigFixture;
    impl TunnelConfigFixture {
        fn invalid(now: chrono::DateTime<chrono::Utc>) -> crate::model::TunnelConfig {
            use crate::model::{
                AuthConfig, ListenSpec, ReconnectPolicy, SshEndpoint, TunnelConfig,
            };
            TunnelConfig {
                id: uuid::Uuid::new_v4(),
                name: String::new(), // invalid: empty name
                description: None,
                kind: ForwardKind::Dynamic,
                enabled: true,
                auto_start: false,
                reconnect: ReconnectPolicy::default(),
                ssh: SshEndpoint {
                    host: "example.com".into(),
                    port: 22,
                    username: "sam".into(),
                },
                auth: AuthConfig::Agent,
                listen: ListenSpec {
                    bind_address: "127.0.0.1".into(),
                    port: 1080,
                },
                target: None,
                proxy: None,
                jump_hosts: Vec::new(),
                keep_alive_seconds: None,
                connect_timeout_ms: None,
                compression: None,
                group: None,
                tags: Vec::new(),
                created_at: now,
                updated_at: now,
            }
        }
    }
}
