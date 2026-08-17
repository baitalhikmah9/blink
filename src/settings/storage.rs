use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::Settings;

const SCHEMA_VERSION: u32 = 1;
const FILE_NAME: &str = "settings.json";

#[derive(Serialize, Deserialize)]
struct StoredFile {
    schema_version: u32,
    #[serde(flatten)]
    settings: Settings,
}

pub fn config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/BlinkCue"))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(FILE_NAME))
}

pub fn load() -> Settings {
    let Some(path) = config_path() else {
        return Settings::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Settings::default();
    };
    match serde_json::from_str::<StoredFile>(&text) {
        Ok(stored) if stored.schema_version == SCHEMA_VERSION => stored.settings,
        Ok(_) => Settings::default(),
        Err(_) => {
            preserve_corrupt_file(&path);
            Settings::default()
        }
    }
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
}
