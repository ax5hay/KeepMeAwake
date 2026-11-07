//! Loads and saves [`Settings`] and the session [`History`] as pretty JSON,
//! atomically.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::Settings;
use crate::core::history::History;

/// File name inside the app's config directory.
pub const SETTINGS_FILE: &str = "settings.json";

/// The session log's file name inside the app's config directory.
pub const HISTORY_FILE: &str = "history.json";

/// Full path of the history file inside `config_dir`.
pub fn history_path(config_dir: &Path) -> PathBuf {
    config_dir.join(HISTORY_FILE)
}

/// Reads the session log. Missing or unreadable reads as empty: it is a log,
/// and losing it must never stop the app.
pub fn load_history(path: &Path) -> History {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
            log::warn!("history file is not valid JSON, starting a new one: {err}");
            History::default()
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => History::default(),
        Err(err) => {
            log::warn!("could not read history, starting a new one: {err}");
            History::default()
        }
    }
}

/// Writes the session log, atomically like settings.
pub fn save_history(path: &Path, history: &History) -> io::Result<()> {
    write_json(path, history)
}

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
    write_json(path, settings)
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
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
    fn history_round_trips_and_survives_a_bad_file() {
        use crate::core::Mode;
        use crate::core::history::Ended;

        let dir = tempfile::tempdir().unwrap();
        let path = history_path(dir.path());
        assert_eq!(load_history(&path), History::default(), "missing");
        let mut history = History::default();
        history.open(1_000, Mode::Timed);
        history.close(61_000, Ended::Timer);
        save_history(&path, &history).unwrap();
        assert_eq!(load_history(&path), history);
        fs::write(&path, "[ not a log").unwrap();
        assert_eq!(load_history(&path), History::default(), "corrupt");
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
