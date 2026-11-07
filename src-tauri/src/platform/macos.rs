//! IOKit implementations of the platform traits.
//!
//! Keeping the Mac awake uses IOPMAssertions
//! (`IOPMAssertionCreateWithName` / `IOPMAssertionRelease`), the mechanism
//! macOS designed for the purpose: each assertion carries a human readable
//! name that shows up in `pmset -g assertions`, and the kernel drops it if
//! the process dies.
//!
//! There is no taskbar theme to follow: the menu bar icon is a template image
//! and macOS tints it for a light or dark menu bar.
//!
//! This and `windows.rs` are the only modules in the crate that contain
//! `unsafe`. The invariants: an assertion id is only ever one IOKit handed
//! back, and each is released exactly once (on `apply` or on drop).

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};

use super::{BatteryProbe, PlatformError, PowerGuard};
use crate::core::{BatteryState, GuardState};

/// Name macOS shows next to our process in `pmset -g assertions`.
const REASON: &str = "KeepMeAwake is keeping this Mac awake";

/// `kIOPMAssertionTypePreventUserIdleSystemSleep`.
const PREVENT_SYSTEM_SLEEP: &str = "PreventUserIdleSystemSleep";
/// `kIOPMAssertionTypePreventUserIdleDisplaySleep`.
const PREVENT_DISPLAY_SLEEP: &str = "PreventUserIdleDisplaySleep";

/// `IOPMAssertionID`, a `uint32_t`.
type AssertionId = u32;
/// `IOReturn`, a `kern_return_t`.
type IoReturn = i32;
/// `kIOPMAssertionLevelOn`.
const LEVEL_ON: u32 = 255;
/// `kIOReturnSuccess`.
const SUCCESS: IoReturn = 0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        level: u32,
        name: CFStringRef,
        id: *mut AssertionId,
    ) -> IoReturn;
    fn IOPMAssertionRelease(id: AssertionId) -> IoReturn;
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, source: CFTypeRef) -> CFDictionaryRef;
}

/// `kIOPSPowerSourceStateKey`: `"AC Power"`, `"Battery Power"` or `"Off Line"`.
const STATE_KEY: &str = "Power Source State";
/// `kIOPSCurrentCapacityKey`.
const CURRENT_KEY: &str = "Current Capacity";
/// `kIOPSMaxCapacityKey`.
const MAX_KEY: &str = "Max Capacity";

/// Two IOPMAssertions, created and released to match a [`GuardState`].
#[derive(Debug, Default)]
pub struct MacPowerGuard {
    system: Option<AssertionId>,
    display: Option<AssertionId>,
}

impl MacPowerGuard {
    /// Nothing is asserted until [`PowerGuard::apply`]. Never fails today;
    /// the `Result` keeps the constructor in step with the Windows guard.
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self::default())
    }
}

impl PowerGuard for MacPowerGuard {
    fn apply(&mut self, want: GuardState) -> Result<(), PlatformError> {
        hold(&mut self.system, want.system, PREVENT_SYSTEM_SLEEP)?;
        hold(&mut self.display, want.display, PREVENT_DISPLAY_SLEEP)
    }

    fn current(&self) -> GuardState {
        GuardState {
            system: self.system.is_some(),
            display: self.display.is_some(),
        }
    }
}

impl Drop for MacPowerGuard {
    fn drop(&mut self) {
        // Best effort: the kernel would drop both when the process exits, but
        // releasing them here keeps `pmset -g assertions` clean on a quit.
        let _ = self.apply(GuardState::default());
    }
}

/// Makes `slot` hold an assertion of `kind` exactly when `on`. A slot is
/// only cleared once its assertion is released, so on error it still says
/// what is held.
fn hold(slot: &mut Option<AssertionId>, on: bool, kind: &'static str) -> Result<(), PlatformError> {
    match (*slot, on) {
        (None, true) => {
            let kind_cf = CFString::from_static_string(kind);
            let name = CFString::from_static_string(REASON);
            let mut id: AssertionId = 0;
            // SAFETY: both CFStrings are valid for the duration of the call
            // (IOKit retains what it keeps) and `id` is a writable u32.
            let code = unsafe {
                IOPMAssertionCreateWithName(
                    kind_cf.as_concrete_TypeRef(),
                    LEVEL_ON,
                    name.as_concrete_TypeRef(),
                    &raw mut id,
                )
            };
            check("IOPMAssertionCreateWithName", code)?;
            *slot = Some(id);
        }
        (Some(id), false) => {
            // SAFETY: `id` came from IOPMAssertionCreateWithName and the slot
            // is cleared below, so it is released exactly once.
            let code = unsafe { IOPMAssertionRelease(id) };
            check("IOPMAssertionRelease", code)?;
            *slot = None;
        }
        _ => {}
    }
    Ok(())
}

fn check(call: &'static str, code: IoReturn) -> Result<(), PlatformError> {
    if code == SUCCESS {
        Ok(())
    } else {
        Err(PlatformError::Os {
            call,
            detail: io_return_text(code),
        })
    }
}

/// An `IOReturn` as macOS headers and logs spell it: `IOReturn 0xe00002c2`.
fn io_return_text(code: IoReturn) -> String {
    format!("IOReturn {code:#010x}")
}

/// Reads the first power source from `IOPSCopyPowerSourcesInfo`. A Mac with
/// none (a Mac mini, a Mac Studio) is always on mains.
#[derive(Debug, Default)]
pub struct MacBattery;

impl BatteryProbe for MacBattery {
    fn read(&self) -> BatteryState {
        let mains = BatteryState {
            on_ac: true,
            percent: None,
        };
        // SAFETY: a Copy function; null is checked before the result is
        // wrapped, and the wrapper releases it once.
        let blob = unsafe { IOPSCopyPowerSourcesInfo() };
        if blob.is_null() {
            return mains;
        }
        // SAFETY: `blob` is a non-null CFTypeRef we own (create rule).
        let blob = unsafe { CFType::wrap_under_create_rule(blob) };
        // SAFETY: `blob` is the live snapshot the list must come from.
        let list = unsafe { IOPSCopyPowerSourcesList(blob.as_CFTypeRef()) };
        if list.is_null() {
            return mains;
        }
        // SAFETY: `list` is a non-null CFArray of CFTypeRefs we own.
        let list: CFArray<CFType> = unsafe { CFArray::wrap_under_create_rule(list) };
        let Some(source) = list.get(0) else {
            return mains;
        };
        // SAFETY: `blob` and `source` are both live; the description is
        // owned by `blob` (get rule), which outlives every use below.
        let description =
            unsafe { IOPSGetPowerSourceDescription(blob.as_CFTypeRef(), source.as_CFTypeRef()) };
        if description.is_null() {
            return mains;
        }
        // SAFETY: non-null CFDictionary with CFString keys, retained by the
        // wrapper for as long as it is used.
        let description: CFDictionary<CFString, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(description) };
        let state = description
            .find(CFString::from_static_string(STATE_KEY))
            .and_then(|value| value.downcast::<CFString>())
            .map(|value| value.to_string());
        let number = |key: &'static str| {
            description
                .find(CFString::from_static_string(key))
                .and_then(|value| value.downcast::<CFNumber>())
                .and_then(|value| value.to_i64())
        };
        decode_power_source(
            state.as_deref().unwrap_or_default(),
            number(CURRENT_KEY),
            number(MAX_KEY),
        )
    }
}

/// Pure, tested: a power source's state string and capacities (as IOKit
/// reports them) to a [`BatteryState`]. An unknown state reads as mains so
/// a bad reading never pauses the sun.
pub fn decode_power_source(state: &str, current: Option<i64>, max: Option<i64>) -> BatteryState {
    let percent = match (current, max) {
        (Some(current), Some(max)) if current >= 0 && max > 0 => {
            // Rounded to the nearest whole percent, as the menu bar shows it.
            let share = (current.saturating_mul(100) + max / 2) / max;
            u8::try_from(share.min(100)).ok()
        }
        _ => None,
    };
    BatteryState {
        on_ac: state != "Battery Power",
        percent,
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// Whether `pmset -g assertions` lists our assertion under `kind`.
    fn pmset_lists(kind: &str) -> bool {
        let out = Command::new("pmset")
            .args(["-g", "assertions"])
            .output()
            .expect("run pmset");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|line| line.contains(kind) && line.contains(REASON))
    }

    #[test]
    fn io_return_codes_read_as_in_the_headers() {
        // kIOReturnNotPermitted, a negative kern_return_t.
        assert_eq!(io_return_text(-0x1fff_fd3e), "IOReturn 0xe00002c2");
        assert_eq!(io_return_text(1), "IOReturn 0x00000001");
    }

    #[test]
    fn decodes_plugged_in_and_unplugged() {
        assert_eq!(
            decode_power_source("AC Power", Some(77), Some(100)),
            BatteryState {
                on_ac: true,
                percent: Some(77)
            }
        );
        assert_eq!(
            decode_power_source("Battery Power", Some(19), Some(100)),
            BatteryState {
                on_ac: false,
                percent: Some(19)
            }
        );
    }

    #[test]
    fn capacity_is_a_share_of_max_and_capped() {
        // Intel Macs report mAh, not a percentage.
        assert_eq!(
            decode_power_source("Battery Power", Some(2_500), Some(5_000)).percent,
            Some(50)
        );
        assert_eq!(
            decode_power_source("AC Power", Some(4_997), Some(5_000)).percent,
            Some(100)
        );
        assert_eq!(
            decode_power_source("AC Power", Some(120), Some(100)).percent,
            Some(100)
        );
    }

    #[test]
    fn unknown_state_or_capacity_never_pauses() {
        assert!(decode_power_source("Off Line", Some(50), Some(100)).on_ac);
        assert!(decode_power_source("", None, None).on_ac);
        assert_eq!(
            decode_power_source("Battery Power", None, Some(100)).percent,
            None
        );
        assert_eq!(
            decode_power_source("Battery Power", Some(5), Some(0)).percent,
            None
        );
        assert_eq!(
            decode_power_source("Battery Power", Some(-1), Some(100)).percent,
            None
        );
    }

    /// A Mac without a battery (this Mac mini, for one) reads as mains.
    #[test]
    fn real_battery_probe_returns_something_sane() {
        let state = MacBattery.read();
        if let Some(p) = state.percent {
            assert!(p <= 100);
        } else {
            assert!(state.on_ac);
        }
    }

    /// Exercises the real API and checks it with `pmset`, the same proof the
    /// spec asks for by hand. Harmless: it holds the assertions for a moment.
    #[test]
    fn real_power_assertion_round_trip() {
        let both = GuardState {
            system: true,
            display: true,
        };
        let system_only = GuardState {
            system: true,
            display: false,
        };
        let mut guard = MacPowerGuard::new().expect("create guard");
        guard.apply(both).expect("assert both");
        assert_eq!(guard.current(), both);
        assert!(pmset_lists(PREVENT_SYSTEM_SLEEP));
        assert!(pmset_lists(PREVENT_DISPLAY_SLEEP));

        guard.apply(system_only).expect("release display");
        guard.apply(system_only).expect("idempotent");
        assert_eq!(guard.current(), system_only);
        assert!(!pmset_lists(PREVENT_DISPLAY_SLEEP));

        drop(guard);
        assert!(!pmset_lists(PREVENT_SYSTEM_SLEEP), "drop releases");
    }
}
