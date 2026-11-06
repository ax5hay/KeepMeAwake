//! Loads and saves [`Settings`] as pretty JSON, atomically.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::Settings;

/// File name inside the app's config directory.
pub const SETTINGS_FILE: &str = "settings.json";

/// Full path of the settings file inside `config_dir`.
pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SETTINGS_FILE)
}

/// Reads settings. A missing or unreadable file yields defaults rather than an
/// error: the app must always start, and a corrupt file is overwritten on the
/// next save.
pub fn load(path: &Path) -> Settings {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<Settings>(&text) {
            Ok(settings) => settings.sanitized(),
            Err(err) => {
                log::warn!("settings file is not valid JSON, using defaults: {err}");
                Settings::default()
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => Settings::default(),
        Err(err) => {
            log::warn!("could not read settings, using defaults: {err}");
            Settings::default()
        }
    }
}

/// Writes settings via a temporary file and rename so a crash mid-write can
/// never leave a truncated file behind.
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    // On Windows `rename` fails if the target exists, so remove it first; a
    // crash between these two calls loses only this save, never the schema.
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::Theme;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&settings_path(dir.path())), Settings::default());
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(dir.path());
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn save_then_load_round_trips_and_sanitises() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir.path().join("nested/dir"));
        let settings = Settings {
            theme: Theme::Light,
            pause_below_battery_percent: Some(1),
            ..Settings::default()
        };
        save(&path, &settings).unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.theme, Theme::Light);
        assert_eq!(loaded.pause_below_battery_percent, Some(5));
        assert!(
            !path.with_extension("json.tmp").exists(),
            "temp file must be gone"
        );
    }

    #[test]
    fn save_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(dir.path());
        save(&path, &Settings::default()).unwrap();
        let second = Settings {
            keep_display_on: false,
            ..Settings::default()
        };
        save(&path, &second).unwrap();
        assert!(!load(&path).keep_display_on);
    }
}
