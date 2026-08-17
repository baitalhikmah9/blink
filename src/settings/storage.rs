use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::Settings;

const SCHEMA_VERSION: u32 = 1;
const FILE_NAME: &str = "settings.json";

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct StoredFile {
    #[serde(default = "schema_version_default")]
    schema_version: u32,
    #[serde(flatten)]
    settings: Settings,
}

impl Default for StoredFile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            settings: Settings::default(),
        }
    }
}

const fn schema_version_default() -> u32 {
    SCHEMA_VERSION
}

pub fn config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Blink"))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(FILE_NAME))
}

pub fn load() -> Settings {
    let Some(path) = config_path() else {
        return normalized_default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return normalized_default();
    };
    match serde_json::from_str::<StoredFile>(&text) {
        Ok(stored) if stored.schema_version == SCHEMA_VERSION => {
            let mut settings = stored.settings;
            settings.normalize();
            settings
        }
        Ok(_) => normalized_default(),
        Err(_) => {
            preserve_corrupt_file(&path);
            normalized_default()
        }
    }
}

fn normalized_default() -> Settings {
    let mut settings = Settings::default();
    settings.normalize();
    settings
}

fn preserve_corrupt_file(path: &Path) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = path.with_extension(format!("json.corrupt-{ts}"));
    let _ = std::fs::rename(path, backup);
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let dir = config_dir().ok_or_else(|| std::io::Error::other("HOME is not set"))?;
    std::fs::create_dir_all(&dir)?;
    let final_path = dir.join(FILE_NAME);
    let tmp_path = dir.join(format!("{FILE_NAME}.tmp"));
    let stored = StoredFile {
        schema_version: SCHEMA_VERSION,
        settings: *settings,
    };
    let json = serde_json::to_string_pretty(&stored)?;
    {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let mut s = Settings::default();
        s.blink.interval_secs = 42.0;
        let stored = StoredFile {
            schema_version: SCHEMA_VERSION,
            settings: s,
        };
        let json = serde_json::to_string(&stored).unwrap();
        let back: StoredFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.settings.blink.interval_secs, 42.0);
    }

    #[test]
    fn additive_deserialization_with_defaults() {
        let json = r#"{ "blink": { "enabled": false } }"#;
        let back: StoredFile = serde_json::from_str(json).unwrap();
        assert!(!back.settings.blink.enabled);
        assert_eq!(back.settings.blink.interval_secs, 30.0);
        assert!(back.settings.eye_break.enabled);
    }

    #[test]
    fn missing_schema_version_defaults_to_current() {
        let json = r#"{ "blink": { "interval_secs": 45.0 } }"#;
        let back: StoredFile = serde_json::from_str(json).unwrap();
        assert_eq!(back.schema_version, SCHEMA_VERSION);
        assert_eq!(back.settings.blink.interval_secs, 45.0);
    }

    #[test]
    fn old_unknown_fields_ignored() {
        let json = r#"{ "statistics": { "enabled": true }, "blink": { "enabled": true } }"#;
        let back: StoredFile = serde_json::from_str(json).unwrap();
        assert!(back.settings.blink.enabled);
    }

    #[test]
    fn statistics_counters_survive_round_trip() {
        let mut s = Settings::default();
        s.statistics.enabled = true;
        s.statistics.blink_cues_shown = 42;
        s.statistics.eye_break_reminders_shown = 7;
        let stored = StoredFile {
            schema_version: SCHEMA_VERSION,
            settings: s,
        };
        let json = serde_json::to_string(&stored).unwrap();
        let back: StoredFile = serde_json::from_str(&json).unwrap();
        assert!(back.settings.statistics.enabled);
        assert_eq!(back.settings.statistics.blink_cues_shown, 42);
        assert_eq!(back.settings.statistics.eye_break_reminders_shown, 7);
    }
}
