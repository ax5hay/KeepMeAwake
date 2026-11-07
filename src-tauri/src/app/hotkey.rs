//! The global hotkey: one combination that does what the sun does, from any
//! app, with the window closed.
//!
//! Registration happens here in Rust (through `RegisterHotKey`, which only
//! listens; nothing is ever sent to other applications), so the webview needs
//! no permission for it.

use std::str::FromStr;

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

use super::AppState;

/// The command error when Windows refuses the combination.
pub const TAKEN: &str = "That combination is already taken.";
/// The command error for a combination without Ctrl, Alt or Win.
pub const NEEDS_MODIFIER: &str = "A hotkey needs at least one of Ctrl, Alt, Win.";

/// The plugin, with a handler that toggles on key down.
pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed
                && let Some(state) = app.try_state::<AppState>()
            {
                state.engine.toggle();
            }
        })
        .build()
}

/// Checks an accelerator such as `Ctrl+Alt+K` and turns it into a shortcut.
/// Shift alone is not enough: it would steal ordinary typing.
pub fn parse(accelerator: &str) -> Result<Shortcut, String> {
    let shortcut = Shortcut::from_str(accelerator).map_err(|err| {
        log::warn!("not a hotkey: {accelerator}: {err}");
        NEEDS_MODIFIER.to_owned()
    })?;
    if shortcut
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        Ok(shortcut)
    } else {
        Err(NEEDS_MODIFIER.to_owned())
    }
}

/// Unregisters everything, then registers `hotkey` (if any).
pub fn apply<R: Runtime>(app: &AppHandle<R>, hotkey: Option<&str>) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    if let Err(err) = shortcuts.unregister_all() {
        log::warn!("could not clear hotkeys: {err}");
    }
    let Some(accelerator) = hotkey else {
        return Ok(());
    };
    let shortcut = parse(accelerator)?;
    shortcuts.register(shortcut).map_err(|err| {
        log::warn!("could not register {accelerator}: {err}");
        TAKEN.to_owned()
    })?;
    log::info!("hotkey {accelerator} registered");
    Ok(())
}

#[cfg(test)]
mod tests {
    use tauri_plugin_global_shortcut::Code;

    use super::*;

    #[test]
    fn parses_the_default() {
        let s = parse("Ctrl+Alt+K").unwrap();
        assert_eq!(s.key, Code::KeyK);
        assert_eq!(s.mods, Modifiers::CONTROL | Modifiers::ALT);
    }

    #[test]
    fn parses_what_the_recorder_produces() {
        assert_eq!(parse("Super+Shift+F9").unwrap().key, Code::F9);
        assert_eq!(parse("Ctrl+1").unwrap().key, Code::Digit1);
        assert_eq!(parse("Alt+Space").unwrap().key, Code::Space);
        assert_eq!(parse("Ctrl+Shift+Up").unwrap().key, Code::ArrowUp);
    }

    #[test]
    fn rejects_garbage_and_bare_keys() {
        assert!(parse("").is_err());
        assert!(parse("Ctrl+").is_err());
        assert!(parse("Ctrl+Nope").is_err());
        assert_eq!(parse("K"), Err(NEEDS_MODIFIER.to_owned()));
        assert_eq!(parse("Shift+K"), Err(NEEDS_MODIFIER.to_owned()));
    }
}
