//! Win32 implementations of the platform traits.
//!
//! Keeping the machine awake uses the power request API
//! (`PowerCreateRequest` / `PowerSetRequest`). Unlike faking input, this is
//! the mechanism Windows designed for the purpose: it is reference counted,
//! carries a human readable reason that shows up in `powercfg /requests`, and
//! is released automatically if the process dies.
//!
//! This is the only module in the crate that contains `unsafe`.

use std::iter::once;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, POWER_REQUEST_TYPE, PowerClearRequest, PowerCreateRequest,
    PowerRequestDisplayRequired, PowerRequestSystemRequired, PowerSetRequest, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::POWER_REQUEST_CONTEXT_VERSION;
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::core::PWSTR;

use super::{BatteryProbe, PlatformError, PowerGuard};
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

    #[test]
    fn real_battery_probe_returns_something_sane() {
        let state = WindowsBattery.read();
        if let Some(p) = state.percent {
            assert!(p <= 100);
        }
    }
}
