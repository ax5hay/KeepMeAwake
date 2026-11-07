//! Win32 implementations of the platform traits.
//!
//! Keeping the machine awake uses the power request API
//! (`PowerCreateRequest` / `PowerSetRequest`). Unlike faking input, this is
//! the mechanism Windows designed for the purpose: it is reference counted,
//! carries a human readable reason that shows up in `powercfg /requests`, and
//! is released automatically if the process dies.
//!
//! The taskbar theme is read from, and watched in, the registry, so the tray
//! icons can follow a light or dark taskbar without polling.
//!
//! How long nobody has touched the machine comes from `GetLastInputInfo`,
//! which only reads the session's last input time.
//!
//! This and `macos.rs` are the only modules in the crate that contain
//! `unsafe`.

use std::iter::once;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WIN32_ERROR};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, POWER_REQUEST_TYPE, PowerClearRequest, PowerCreateRequest,
    PowerRequestDisplayRequired, PowerRequestSystemRequired, PowerSetRequest, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, RRF_RT_REG_DWORD, RegCloseKey,
    RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW,
};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::SystemServices::POWER_REQUEST_CONTEXT_VERSION;
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::core::{PCWSTR, PWSTR, w};

use super::{BatteryProbe, IdleProbe, PlatformError, PowerGuard, TaskbarTheme};
use crate::core::{BatteryState, GuardState};

/// Text Windows shows next to our process in `powercfg /requests`.
const REASON: &str = "KeepMeAwake is keeping this PC awake";

/// A power request object plus the two flags we may hold on it.
pub struct WindowsPowerGuard {
    handle: HANDLE,
    // The reason string must outlive the request object.
    _reason: Box<[u16]>,
    current: GuardState,
}

// SAFETY: the handle is only used from the engine thread; HANDLE is a plain
// integer wrapper and the request object is process-wide, not thread-affine.
unsafe impl Send for WindowsPowerGuard {}

impl WindowsPowerGuard {
    /// Creates the request object. Nothing is requested until [`PowerGuard::apply`].
    pub fn new() -> Result<Self, PlatformError> {
        let mut reason: Box<[u16]> = REASON.encode_utf16().chain(once(0)).collect();
        let context = REASON_CONTEXT {
            Version: POWER_REQUEST_CONTEXT_VERSION,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: PWSTR(reason.as_mut_ptr()),
            },
        };
        // SAFETY: `context` is fully initialised and `reason` is a valid,
        // NUL-terminated UTF-16 buffer that lives as long as `self`.
        let handle = unsafe { PowerCreateRequest(&raw const context) }
            .map_err(|e| os_error("PowerCreateRequest", &e))?;
        Ok(Self {
            handle,
            _reason: reason,
            current: GuardState::default(),
        })
    }

    fn set(
        &self,
        kind: POWER_REQUEST_TYPE,
        on: bool,
        name: &'static str,
    ) -> Result<(), PlatformError> {
        // SAFETY: `self.handle` came from PowerCreateRequest and is still open.
        let result = unsafe {
            if on {
                PowerSetRequest(self.handle, kind)
            } else {
                PowerClearRequest(self.handle, kind)
            }
        };
        result.map_err(|e| os_error(name, &e))
    }
}

impl PowerGuard for WindowsPowerGuard {
    fn apply(&mut self, want: GuardState) -> Result<(), PlatformError> {
        if want.system != self.current.system {
            self.set(
                PowerRequestSystemRequired,
                want.system,
                "PowerSetRequest(System)",
            )?;
            self.current.system = want.system;
        }
        if want.display != self.current.display {
            self.set(
                PowerRequestDisplayRequired,
                want.display,
                "PowerSetRequest(Display)",
            )?;
            self.current.display = want.display;
        }
        Ok(())
    }

    fn current(&self) -> GuardState {
        self.current
    }
}

impl Drop for WindowsPowerGuard {
    fn drop(&mut self) {
        // Best effort: release whatever we hold, then the handle. The OS would
        // do both when the process exits, but being explicit keeps
        // `powercfg /requests` clean during a graceful shutdown.
        let _ = self.apply(GuardState::default());
        // SAFETY: handle is valid and not used after this point.
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

/// Reads `GetSystemPowerStatus`.
#[derive(Debug, Default)]
pub struct WindowsBattery;

impl BatteryProbe for WindowsBattery {
    fn read(&self) -> BatteryState {
        let mut status = SYSTEM_POWER_STATUS::default();
        // SAFETY: `status` is a valid, writable struct of the expected size.
        if unsafe { GetSystemPowerStatus(&raw mut status) }.is_err() {
            // Unknown: assume mains so we never pause for a bad reading.
            return BatteryState {
                on_ac: true,
                percent: None,
            };
        }
        decode_power_status(
            status.ACLineStatus,
            status.BatteryFlag,
            status.BatteryLifePercent,
        )
    }
}

/// Where Windows keeps the light/dark choice for the shell and for apps.
const PERSONALIZE: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

/// 1 when the taskbar, Start and the tray use the light theme.
const SYSTEM_USES_LIGHT_THEME: PCWSTR = w!("SystemUsesLightTheme");

/// `true` when the taskbar uses the light theme (`SystemUsesLightTheme` = 1).
/// A missing value (older Windows) reads as dark, the look the white icons
/// were drawn for.
pub fn taskbar_is_light() -> bool {
    let mut value: u32 = 0;
    let mut size: u32 = 4;
    // SAFETY: both strings are static NUL-terminated UTF-16; `value` is a
    // writable u32 and `size` says exactly that, and RRF_RT_REG_DWORD makes
    // the call fail rather than write anything that is not a DWORD.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PERSONALIZE,
            SYSTEM_USES_LIGHT_THEME,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&raw mut size),
        )
    };
    status.is_ok() && value == 1
}

/// Blocks until something under the Personalize key changes, then returns.
/// Used by a dedicated thread; the wait happens in the kernel, so it costs
/// nothing while nobody touches the theme.
pub fn wait_for_theme_change() -> Result<(), PlatformError> {
    let mut key = HKEY::default();
    // SAFETY: the subkey is a static NUL-terminated UTF-16 string and `key`
    // is a valid place for the handle; it is closed below on every path.
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PERSONALIZE,
            None,
            KEY_NOTIFY,
            &raw mut key,
        )
    };
    if opened.is_err() {
        return Err(win32_error("RegOpenKeyExW", opened));
    }
    // SAFETY: `key` is open with KEY_NOTIFY; with no event and
    // `fasynchronous` false the call blocks until a value changes.
    let waited =
        unsafe { RegNotifyChangeKeyValue(key, false, REG_NOTIFY_CHANGE_LAST_SET, None, false) };
    // SAFETY: `key` came from RegOpenKeyExW and is not used after this.
    let _ = unsafe { RegCloseKey(key) };
    if waited.is_err() {
        return Err(win32_error("RegNotifyChangeKeyValue", waited));
    }
    Ok(())
}

/// The taskbar theme, read from the registry.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsTaskbarTheme;

impl TaskbarTheme for WindowsTaskbarTheme {
    fn is_light(&self) -> bool {
        taskbar_is_light()
    }
}

/// Time since the last keyboard or mouse input in this session, from
/// `GetLastInputInfo`. Never sends input; needs no permission.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsIdle;

impl IdleProbe for WindowsIdle {
    fn idle(&self) -> Option<Duration> {
        let mut info = LASTINPUTINFO {
            cbSize: u32::try_from(size_of::<LASTINPUTINFO>()).ok()?,
            dwTime: 0,
        };
        // SAFETY: `info` is a writable LASTINPUTINFO whose `cbSize` is set
        // to its own size, as the API requires.
        if !unsafe { GetLastInputInfo(&raw mut info) }.as_bool() {
            return None;
        }
        // SAFETY: takes no arguments and cannot fail.
        let now = unsafe { GetTickCount() };
        Some(idle_between(now, info.dwTime))
    }
}

/// How long ago `last_input` was, both read from the 32-bit tick count.
/// The count wraps every 49.7 days, so the subtraction wraps with it.
pub fn idle_between(now_ticks: u32, last_input_ticks: u32) -> Duration {
    Duration::from_millis(u64::from(now_ticks.wrapping_sub(last_input_ticks)))
}

fn win32_error(call: &'static str, code: WIN32_ERROR) -> PlatformError {
    PlatformError::Os {
        call,
        detail: windows::core::Error::from(code.to_hresult()).to_string(),
    }
}

/// Pure decoding of the three bytes `GetSystemPowerStatus` returns, so the
/// mapping can be tested without a battery.
///
/// `ac_line`: 0 offline, 1 online, 255 unknown. `flag` bit 128 means no
/// battery. `percent` 255 means unknown.
pub fn decode_power_status(ac_line: u8, flag: u8, percent: u8) -> BatteryState {
    let no_battery = flag & 128 != 0;
    let on_ac = ac_line == 1 || ac_line == 255 || no_battery;
    let percent = if no_battery || percent == 255 {
        None
    } else {
        Some(percent.min(100))
    };
    BatteryState { on_ac, percent }
}

fn os_error(call: &'static str, err: &windows::core::Error) -> PlatformError {
    PlatformError::Os {
        call,
        detail: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plugged_in_with_battery() {
        assert_eq!(
            decode_power_status(1, 0, 77),
            BatteryState {
                on_ac: true,
                percent: Some(77)
            }
        );
    }

    #[test]
    fn decodes_unplugged() {
        assert_eq!(
            decode_power_status(0, 0, 19),
            BatteryState {
                on_ac: false,
                percent: Some(19)
            }
        );
    }

    #[test]
    fn unknown_line_status_is_treated_as_mains() {
        assert!(decode_power_status(255, 0, 50).on_ac);
    }

    #[test]
    fn desktops_without_battery_are_always_on_mains() {
        assert_eq!(
            decode_power_status(0, 128, 255),
            BatteryState {
                on_ac: true,
                percent: None
            }
        );
    }

    #[test]
    fn unknown_percent_is_none_and_values_are_capped() {
        assert_eq!(decode_power_status(1, 0, 255).percent, None);
        assert_eq!(decode_power_status(1, 0, 120).percent, Some(100));
    }

    /// Exercises the real API. Runs only on Windows and is harmless: it holds a
    /// request for a moment and releases it.
    #[test]
    fn real_power_request_round_trip() {
        let mut guard = WindowsPowerGuard::new().expect("create request");
        guard
            .apply(GuardState {
                system: true,
                display: true,
            })
            .expect("set");
        assert_eq!(
            guard.current(),
            GuardState {
                system: true,
                display: true
            }
        );
        guard
            .apply(GuardState {
                system: true,
                display: false,
            })
            .expect("partial clear");
        assert_eq!(
            guard.current(),
            GuardState {
                system: true,
                display: false
            }
        );
        guard.apply(GuardState::default()).expect("clear");
        assert_eq!(guard.current(), GuardState::default());
    }

    /// Reads the real registry value. Only asserts that it answers; the
    /// value depends on the machine's theme.
    #[test]
    fn taskbar_theme_reads_without_error() {
        let light = taskbar_is_light();
        assert_eq!(WindowsTaskbarTheme.is_light(), light);
    }

    #[test]
    fn idle_time_survives_the_tick_count_wrapping() {
        assert_eq!(idle_between(5_000, 2_000), Duration::from_secs(3));
        // Input just before the wrap, read just after it.
        assert_eq!(idle_between(1_000, u32::MAX - 999), Duration::from_secs(2));
        assert_eq!(idle_between(7, 7), Duration::ZERO);
    }

    /// Reads the real last-input time. Only asserts that it answers and is
    /// no longer than the tick count can measure.
    #[test]
    fn real_idle_reading_answers_and_is_sane() {
        let idle = WindowsIdle.idle().expect("GetLastInputInfo answers");
        assert!(idle <= Duration::from_millis(u64::from(u32::MAX)));
    }

    #[test]
    fn real_battery_probe_returns_something_sane() {
        let state = WindowsBattery.read();
        if let Some(p) = state.percent {
            assert!(p <= 100);
        }
    }
}
