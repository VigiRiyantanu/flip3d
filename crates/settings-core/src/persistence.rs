// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::model::{Config, CURRENT_CONFIG_VERSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    LoadedNormal,
    MigratedFromToml,
    CreatedDefault,
    RecoveredFromCorruption,
}

pub fn app_data_dir() -> PathBuf {
    crate::paths::AppPaths::get().config_dir().to_path_buf()
}

pub fn config_json_path() -> PathBuf {
    crate::paths::AppPaths::get().config_file().to_path_buf()
}

pub fn config_toml_path() -> PathBuf {
    app_data_dir().join("config.toml")
}

pub fn config_toml_migrated_path() -> PathBuf {
    app_data_dir().join("config.toml.migrated")
}

pub fn config_bak_path() -> PathBuf {
    app_data_dir().join("config.json.bak")
}

/// Load configuration with automatic TOML migration, corruption recovery, and validation.
pub fn load_config() -> (Config, LoadStatus) {
    let json_path = config_json_path();
    let toml_path = config_toml_path();

    // 1. Check if config.json exists
    if json_path.exists() {
        match fs::read_to_string(&json_path) {
            Ok(content) => match serde_json::from_str::<Config>(&content) {
                Ok(mut config) => {
                    config.validate_and_clamp();
                    return (config, LoadStatus::LoadedNormal);
                }
                Err(err) => {
                    log::warn!(
                        "Corrupted config.json encountered ({}). Backing up to config.json.bak and loading defaults.",
                        err
                    );
                    let _ = fs::copy(&json_path, config_bak_path());
                    let default_config = Config::default();
                    return (default_config, LoadStatus::RecoveredFromCorruption);
                }
            },
            Err(err) => {
                log::error!("Failed to read config.json: {}", err);
                return (Config::default(), LoadStatus::CreatedDefault);
            }
        }
    }

    // 2. config.json does not exist. Check if legacy config.toml exists to migrate.
    if toml_path.exists() {
        log::info!(
            "Found legacy config.toml at {:?}. Migrating to config.json...",
            toml_path
        );
        if let Ok(content) = fs::read_to_string(&toml_path) {
            match toml::from_str::<Config>(&content) {
                Ok(mut config) => {
                    config.version = CURRENT_CONFIG_VERSION;
                    config.validate_and_clamp();
                    if let Err(e) = save_config_atomic(&config) {
                        log::error!("Failed to write migrated config.json: {}", e);
                    } else {
                        let _ = fs::rename(&toml_path, config_toml_migrated_path());
                        log::info!(
                            "Migration complete: saved config.json and renamed legacy config to config.toml.migrated"
                        );
                        return (config, LoadStatus::MigratedFromToml);
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to parse legacy config.toml ({}). Renaming to config.toml.migrated and using defaults.",
                        e
                    );
                    let _ = fs::rename(&toml_path, config_toml_migrated_path());
                }
            }
        }
    }

    // 3. Brand new installation: save default config.json
    let default_config = Config::default();
    let _ = save_config_atomic(&default_config);
    (default_config, LoadStatus::CreatedDefault)
}

/// Atomically save configuration by writing to a temporary file in the same directory and renaming.
pub fn save_config_atomic(config: &Config) -> std::io::Result<()> {
    let dest_path = config_json_path();
    let dir = app_data_dir();
    fs::create_dir_all(&dir)?;

    let mut validated = config.clone();
    validated.validate_and_clamp();

    let json_str = serde_json::to_string_pretty(&validated)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let tmp_path = dir.join(format!(
        "config.json.tmp.{}.{}",
        std::process::id(),
        now_millis
    ));

    {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)?;
        file.write_all(json_str.as_bytes())?;
        file.sync_all()?;
    }

    // On Windows, rename replaces an existing file. If destination is locked momentarily, retry briefly.
    let mut replaced = false;
    for attempt in 0..5 {
        match fs::rename(&tmp_path, &dest_path) {
            Ok(_) => {
                replaced = true;
                break;
            }
            Err(err) => {
                if attempt == 4 {
                    let _ = fs::remove_file(&tmp_path);
                    return Err(err);
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
        }
    }

    if !replaced {
        let _ = fs::remove_file(&tmp_path);
        return Err(std::io::Error::other(
            "Failed to replace config.json after retries",
        ));
    }

    Ok(())
}

impl crate::model::Settings {
    pub fn config_path() -> PathBuf {
        config_json_path()
    }

    pub fn load() -> Self {
        let (cfg, _) = load_config();
        Self::from_config(&cfg)
    }

    pub fn save_raw(raw: &crate::model::RawConfigFile) -> std::io::Result<()> {
        save_config_atomic(raw)
    }
}
