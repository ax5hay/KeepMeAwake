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
//! How long nobody has touched the Mac comes from CoreGraphics
//! (`CGEventSourceSecondsSinceLastEventType`), which only reads the HID
//! system's last event time and needs no permission.
//!
//! This and `windows.rs` are the only modules in the crate that contain
//! `unsafe`. The invariants: an assertion id is only ever one IOKit handed
//! back, and each is released exactly once (on `apply` or on drop).

use std::time::Duration;

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};

use super::{ActivityProbe, BatteryProbe, IdleProbe, PlatformError, PowerGuard};
use crate::core::watch::CpuTicks;
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
    fn IOPSGetTimeRemainingEstimate() -> f64;
}

/// `CGEventSourceStateID`, an `int32_t`.
type EventSourceState = i32;
/// `CGEventType`, a `uint32_t`.
type EventType = u32;
/// `kCGEventSourceStateHIDSystemState`: input from the hardware itself.
const HID_SYSTEM_STATE: EventSourceState = 1;
/// `kCGAnyInputEventType`, `(CGEventType)(~0)`: any key, click, move or scroll.
const ANY_INPUT_EVENT: EventType = u32::MAX;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(
        state: EventSourceState,
        event_type: EventType,
    ) -> f64;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFArrayRef;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayIsBuiltin(display: u32) -> u32;
    fn CGDisplayBounds(display: u32) -> CgRect;
}

/// `CGRect`, as CoreGraphics lays it out on a 64-bit Mac.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct CgRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// `kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements`.
const ON_SCREEN_WINDOWS: u32 = (1 << 0) | (1 << 4);
/// Displays looked at; nobody has more.
const MAX_DISPLAYS: usize = 16;
/// `IFT_ETHER`: the interface type of Wi-Fi and Ethernet on a Mac. Tunnels,
/// bridges and loopback carry the same bytes again, so they are left out.
const IFT_ETHER: u8 = 0x6;
/// `HOST_CPU_LOAD_INFO`'s four counters: user, system, idle, nice.
const CPU_STATES: usize = 4;
const CPU_STATE_IDLE: usize = 2;

// Mach, declared here because the libc crate marks its own copies
// deprecated in favour of another crate; both come from libSystem.
unsafe extern "C" {
    fn mach_host_self() -> u32;
    fn host_statistics(host: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
}

/// `kIOPSPowerSourceStateKey`: `"AC Power"`, `"Battery Power"` or `"Off Line"`.
const STATE_KEY: &str = "Power Source State";
/// `kIOPSCurrentCapacityKey`.
const CURRENT_KEY: &str = "Current Capacity";
/// `kIOPSMaxCapacityKey`.
const MAX_KEY: &str = "Max Capacity";
/// `kIOPSTypeKey`, and the value it has for a laptop's own battery
/// (`kIOPSInternalBatteryType`). A UPS is another type and is not a battery
/// this machine carries.
const TYPE_KEY: &str = "Type";
const INTERNAL_BATTERY: &str = "InternalBattery";

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
            present: false,
            minutes_left: None,
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
        // The laptop's own battery, wherever it is in the list; a Mac mini
        // or an iMac has none (a UPS may be listed, and is not one).
        let description = list.iter().find_map(|source| {
            // SAFETY: `blob` and `source` are both live; the description is
            // owned by `blob` (get rule), which outlives every use below.
            let description = unsafe {
                IOPSGetPowerSourceDescription(blob.as_CFTypeRef(), source.as_CFTypeRef())
            };
            if description.is_null() {
                return None;
            }
            // SAFETY: non-null CFDictionary with CFString keys, retained by
            // the wrapper for as long as it is used.
            let description: CFDictionary<CFString, CFType> =
                unsafe { CFDictionary::wrap_under_get_rule(description) };
            let kind = description
                .find(CFString::from_static_string(TYPE_KEY))
                .and_then(|value| value.downcast::<CFString>())
                .map(|value| value.to_string());
            (kind.as_deref() == Some(INTERNAL_BATTERY)).then_some(description)
        });
        let Some(description) = description else {
            return mains;
        };
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
        let mut battery = decode_power_source(
            state.as_deref().unwrap_or_default(),
            number(CURRENT_KEY),
            number(MAX_KEY),
        );
        if !battery.on_ac {
            // SAFETY: no arguments; returns a plain double.
            battery.minutes_left = minutes_from_estimate(unsafe { IOPSGetTimeRemainingEstimate() });
        }
        battery
    }
}

/// Turns the screen off or sleeps, through `pmset`, which any user may run
/// for these two. macOS has no public way to lock: the screen going off
/// locks it when the Mac asks for a password on wake.
pub fn after_end(action: crate::core::AfterEnd) -> Result<(), PlatformError> {
    use crate::core::AfterEnd;
    let verb = match action {
        AfterEnd::Nothing => return Ok(()),
        AfterEnd::Lock | AfterEnd::ScreenOff => "displaysleepnow",
        AfterEnd::Sleep => "sleepnow",
    };
    let status = std::process::Command::new("/usr/bin/pmset")
        .arg(verb)
        .status()
        .map_err(|err| PlatformError::Os {
            call: "pmset",
            detail: err.to_string(),
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(PlatformError::Os {
            call: "pmset",
            detail: status.to_string(),
        })
    }
}

/// Pure, tested: `IOPSGetTimeRemainingEstimate`'s seconds as whole minutes.
/// `-1` (still working it out) and `-2` (on mains) are no estimate.
pub fn minutes_from_estimate(secs: f64) -> Option<u32> {
    if !secs.is_finite() || secs < 0.0 {
        return None;
    }
    // Saturating: an estimate past u32 minutes is not one anybody reads.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((secs / 60.0).round().min(f64::from(u32::MAX)) as u32)
}

/// What the Mac is doing, for stay awake while…. Each reading is one
/// read-only call; nothing here touches another process.
#[derive(Debug)]
pub struct MacActivity {
    /// This task's send right to the host port, taken once: each
    /// `mach_host_self` adds a reference that would otherwise pile up.
    host: u32,
}

impl MacActivity {
    /// Takes the host port once, for the processor readings.
    pub fn new() -> Self {
        // SAFETY: no arguments; returns a port name for this task.
        let host = unsafe { mach_host_self() };
        Self { host }
    }
}

impl Default for MacActivity {
    fn default() -> Self {
        Self::new()
    }
}

/// The names of every process running now. Also what the app picker
/// offers, so it needs no probe of its own.
pub fn list_processes() -> Vec<String> {
    // SAFETY: a null buffer of size 0 asks only for the count.
    let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    let Ok(count) = usize::try_from(count) else {
        return Vec::new();
    };
    // Room for processes started between the two calls.
    let mut pids: Vec<libc::pid_t> = vec![0; count + 64];
    let bytes = i32::try_from(pids.len() * size_of::<libc::pid_t>()).unwrap_or(i32::MAX);
    // SAFETY: the buffer holds `bytes` bytes of pid_t, as passed.
    let listed = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
    pids.truncate(usize::try_from(listed).unwrap_or(0).min(pids.len()));
    let mut name = [0_u8; 256];
    pids.iter()
        .filter(|pid| **pid > 0)
        .filter_map(|pid| {
            // SAFETY: `name` is writable for the length passed.
            let len = unsafe {
                libc::proc_name(
                    *pid,
                    name.as_mut_ptr().cast(),
                    u32::try_from(name.len()).ok()?,
                )
            };
            let len = usize::try_from(len).ok().filter(|len| *len > 0)?;
            Some(String::from_utf8_lossy(&name[..len.min(name.len())]).into_owned())
        })
        .collect()
}

impl ActivityProbe for MacActivity {
    fn processes(&self) -> Vec<String> {
        list_processes()
    }

    fn alive(&self, pid: u32) -> bool {
        pid_alive(pid)
    }

    fn transferred(&self) -> Option<u64> {
        let mut mib = [libc::CTL_NET, libc::PF_ROUTE, 0, 0, libc::NET_RT_IFLIST2, 0];
        let mut len: libc::size_t = 0;
        // SAFETY: a null buffer asks only for the size needed.
        let sized = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                6,
                std::ptr::null_mut(),
                &raw mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if sized != 0 || len == 0 {
            return None;
        }
        // Room for an interface that appears between the two calls.
        len += 1024;
        let mut buf = vec![0_u8; len];
        // SAFETY: `buf` is writable for `len` bytes, as passed.
        let read = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                6,
                buf.as_mut_ptr().cast(),
                &raw mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if read != 0 {
            return None;
        }
        buf.truncate(len);
        Some(sum_interface_bytes(&buf))
    }

    fn cpu(&self) -> Option<CpuTicks> {
        let mut ticks = [0_i32; CPU_STATES];
        let mut count = u32::try_from(CPU_STATES).ok()?;
        // SAFETY: `ticks` holds HOST_CPU_LOAD_INFO_COUNT integers, as
        // `count` says; `host` is this task's host port.
        let code = unsafe {
            host_statistics(
                self.host,
                libc::HOST_CPU_LOAD_INFO,
                ticks.as_mut_ptr(),
                &raw mut count,
            )
        };
        if code != 0 {
            return None;
        }
        // The counters are unsigned in the kernel.
        let ticks = ticks.map(|t| u64::from(t.cast_unsigned()));
        Some(CpuTicks {
            busy: ticks.iter().sum::<u64>() - ticks[CPU_STATE_IDLE],
            total: ticks.iter().sum(),
        })
    }

    fn full_screen(&self) -> bool {
        let displays = displays();
        if displays.is_empty() {
            return false;
        }
        let bounds: Vec<CgRect> = displays
            .iter()
            // SAFETY: a display id from CGGetActiveDisplayList.
            .map(|id| unsafe { CGDisplayBounds(*id) })
            .collect();
        // SAFETY: plain flags in; the array is owned (create rule) below.
        let list = unsafe { CGWindowListCopyWindowInfo(ON_SCREEN_WINDOWS, 0) };
        if list.is_null() {
            return false;
        }
        // SAFETY: non-null CFArray of CFDictionary, released by the wrapper.
        let list: CFArray<CFDictionary<CFString, CFType>> =
            unsafe { CFArray::wrap_under_create_rule(list) };
        let ours = i64::from(std::process::id());
        let number = |window: &CFDictionary<CFString, CFType>, key: &'static str| {
            window
                .find(CFString::from_static_string(key))
                .and_then(|value| value.downcast::<CFNumber>())
        };
        list.iter().any(|window| {
            let layer = number(&window, "kCGWindowLayer").and_then(|n| n.to_i64());
            let owner = number(&window, "kCGWindowOwnerPID").and_then(|n| n.to_i64());
            if layer != Some(0) || owner == Some(ours) {
                return false;
            }
            let Some(rect) = window
                .find(CFString::from_static_string("kCGWindowBounds"))
                .and_then(|value| value.downcast::<CFDictionary>())
            else {
                return false;
            };
            // SAFETY: the bounds dictionary's keys are CFStrings and its
            // values CFNumbers; re-typed without changing ownership.
            let rect: CFDictionary<CFString, CFType> =
                unsafe { CFDictionary::wrap_under_get_rule(rect.as_concrete_TypeRef()) };
            let side = |key: &'static str| {
                rect.find(CFString::from_static_string(key))
                    .and_then(|value| value.downcast::<CFNumber>())
                    .and_then(|n| n.to_f64())
            };
            let (Some(x), Some(y), Some(width), Some(height)) =
                (side("X"), side("Y"), side("Width"), side("Height"))
            else {
                return false;
            };
            covers_a_display(
                CgRect {
                    x,
                    y,
                    width,
                    height,
                },
                &bounds,
            )
        })
    }

    fn external_display(&self) -> bool {
        displays()
            .iter()
            // SAFETY: a display id from CGGetActiveDisplayList.
            .any(|id| unsafe { CGDisplayIsBuiltin(*id) } == 0)
    }
}

/// The process with this id is running. Also how `--status` tells a
/// running app from a status file left behind by one that was killed.
pub fn pid_alive(pid: u32) -> bool {
    // pid 0 would ask about the whole process group: never.
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // SAFETY: signal 0 sends nothing; it only checks the pid exists.
    let sent = unsafe { libc::kill(pid, 0) };
    sent == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// The displays in use now.
fn displays() -> Vec<u32> {
    let mut ids = [0_u32; MAX_DISPLAYS];
    let mut count: u32 = 0;
    let max = u32::try_from(MAX_DISPLAYS).unwrap_or(0);
    // SAFETY: `ids` holds `max` display ids, as passed; `count` is writable.
    let code = unsafe { CGGetActiveDisplayList(max, ids.as_mut_ptr(), &raw mut count) };
    if code != 0 {
        return Vec::new();
    }
    ids[..usize::try_from(count).unwrap_or(0).min(MAX_DISPLAYS)].to_vec()
}

/// Pure, tested: a window covers a whole display, give or take a point of
/// rounding: full screen, as a video, a game or slides are.
fn covers_a_display(window: CgRect, displays: &[CgRect]) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() < 1.0;
    displays.iter().any(|d| {
        d.width > 0.0
            && near(window.x, d.x)
            && near(window.y, d.y)
            && near(window.width, d.width)
            && near(window.height, d.height)
    })
}

/// Pure, tested: the bytes in and out of every Wi-Fi and Ethernet
/// interface in a `NET_RT_IFLIST2` reply. Each message starts with its
/// length; only `RTM_IFINFO2` ones carry counters.
pub fn sum_interface_bytes(buf: &[u8]) -> u64 {
    let mut total: u64 = 0;
    let mut offset = 0;
    while offset + size_of::<libc::if_msghdr>() <= buf.len() {
        // SAFETY: at least a header's worth of bytes remains at `offset`;
        // read unaligned, as the kernel packs messages back to back.
        let header: libc::if_msghdr =
            unsafe { std::ptr::read_unaligned(buf.as_ptr().add(offset).cast()) };
        let step = usize::from(header.ifm_msglen);
        if step == 0 {
            break;
        }
        if i32::from(header.ifm_type) == libc::RTM_IFINFO2
            && offset + size_of::<libc::if_msghdr2>() <= buf.len()
        {
            // SAFETY: a whole if_msghdr2 remains at `offset`, read unaligned.
            let info: libc::if_msghdr2 =
                unsafe { std::ptr::read_unaligned(buf.as_ptr().add(offset).cast()) };
            if info.ifm_flags & libc::IFF_LOOPBACK == 0 && info.ifm_data.ifi_type == IFT_ETHER {
                total = total
                    .saturating_add(info.ifm_data.ifi_ibytes)
                    .saturating_add(info.ifm_data.ifi_obytes);
            }
        }
        offset += step;
    }
    total
}

/// Time since the last keyboard, mouse or trackpad input, from the HID
/// system's event source. Never sends input; needs no permission.
#[derive(Debug, Default, Clone, Copy)]
pub struct MacIdle;

impl IdleProbe for MacIdle {
    fn idle(&self) -> Option<Duration> {
        // SAFETY: takes two plain integers and returns a double; no
        // pointers, nothing to free.
        let secs =
            unsafe { CGEventSourceSecondsSinceLastEventType(HID_SYSTEM_STATE, ANY_INPUT_EVENT) };
        idle_from_seconds(secs)
    }
}

/// Pure, tested: the seconds CoreGraphics reports as a [`Duration`], or
/// `None` for a reading that is negative, not a number or out of range.
pub fn idle_from_seconds(secs: f64) -> Option<Duration> {
    Duration::try_from_secs_f64(secs).ok()
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
        present: true,
        minutes_left: None,
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// Whether `pmset -g assertions` lists an assertion of `kind` held by
    /// this test process. Matching the pid, not just the name, keeps the
    /// test honest while the app itself is running and holding its own.
    fn pmset_lists(kind: &str) -> bool {
        let out = Command::new("pmset")
            .args(["-g", "assertions"])
            .output()
            .expect("run pmset");
        let ours = format!("pid {}(", std::process::id());
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|line| line.contains(&ours) && line.contains(kind) && line.contains(REASON))
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
                percent: Some(77),
                present: true,
                minutes_left: None,
            }
        );
        assert_eq!(
            decode_power_source("Battery Power", Some(19), Some(100)),
            BatteryState {
                on_ac: false,
                percent: Some(19),
                present: true,
                minutes_left: None,
            }
        );
    }

    #[test]
    fn a_window_the_size_of_a_display_is_full_screen() {
        let screen = CgRect {
            x: 0.0,
            y: 0.0,
            width: 1512.0,
            height: 982.0,
        };
        let second = CgRect {
            x: 1512.0,
            y: 0.0,
            width: 2560.0,
            height: 1440.0,
        };
        let displays = [screen, second];
        assert!(covers_a_display(screen, &displays));
        assert!(covers_a_display(
            CgRect {
                x: 1512.4,
                ..second
            },
            &displays
        ));
        // A zoomed window stops at the menu bar: not full screen.
        let zoomed = CgRect {
            y: 33.0,
            height: 949.0,
            ..screen
        };
        assert!(!covers_a_display(zoomed, &displays));
        assert!(!covers_a_display(screen, &[]));
    }

    #[test]
    fn interface_bytes_skip_loopback_and_tunnels() {
        fn message(flags: i32, kind: u8, ibytes: u64, obytes: u64) -> Vec<u8> {
            // SAFETY: if_msghdr2 is plain old data; all zeroes is valid.
            let mut info: libc::if_msghdr2 = unsafe { std::mem::zeroed() };
            info.ifm_msglen = u16::try_from(size_of::<libc::if_msghdr2>()).unwrap();
            info.ifm_type = u8::try_from(libc::RTM_IFINFO2).unwrap();
            info.ifm_flags = flags;
            info.ifm_data.ifi_type = kind;
            info.ifm_data.ifi_ibytes = ibytes;
            info.ifm_data.ifi_obytes = obytes;
            // SAFETY: viewing a plain struct as its bytes.
            unsafe {
                std::slice::from_raw_parts(
                    (&raw const info).cast::<u8>(),
                    size_of::<libc::if_msghdr2>(),
                )
            }
            .to_vec()
        }
        let mut buf = message(0, IFT_ETHER, 1_000, 500);
        buf.extend(message(libc::IFF_LOOPBACK, 0x18, 9_000, 9_000));
        buf.extend(message(0, 0xff, 7_000, 7_000)); // a tunnel
        buf.extend(message(0, IFT_ETHER, 20, 30));
        assert_eq!(sum_interface_bytes(&buf), 1_550);
        assert_eq!(sum_interface_bytes(&[]), 0);
        assert_eq!(sum_interface_bytes(&buf[..10]), 0);
    }

    /// The real readings answer, and this test process is alive.
    #[test]
    fn real_activity_readings_answer() {
        let activity = MacActivity::new();
        let processes = activity.processes();
        assert_ne!(processes.len(), 0);
        assert!(activity.alive(std::process::id()));
        assert!(!activity.alive(0));
        assert!(!activity.alive(u32::MAX));
        let cpu = activity.cpu().expect("host_statistics answers");
        assert!(cpu.total >= cpu.busy && cpu.total > 0);
        assert!(activity.transferred().is_some());
        let _ = activity.full_screen();
        let _ = activity.external_display();
    }

    #[test]
    fn the_estimate_is_whole_minutes_or_nothing() {
        assert_eq!(minutes_from_estimate(11_400.0), Some(190));
        assert_eq!(minutes_from_estimate(89.0), Some(1));
        assert_eq!(minutes_from_estimate(-1.0), None);
        assert_eq!(minutes_from_estimate(-2.0), None);
        assert_eq!(minutes_from_estimate(f64::NAN), None);
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

    #[test]
    fn idle_seconds_become_a_duration_or_nothing() {
        assert_eq!(idle_from_seconds(90.5), Some(Duration::from_millis(90_500)));
        assert_eq!(idle_from_seconds(0.0), Some(Duration::ZERO));
        assert_eq!(idle_from_seconds(-1.0), None);
        assert_eq!(idle_from_seconds(f64::NAN), None);
        assert_eq!(idle_from_seconds(f64::INFINITY), None);
    }

    /// Reads the real HID idle time. Only asserts that it answers with
    /// something a running Mac could report (under a year).
    #[test]
    fn real_idle_reading_answers_and_is_sane() {
        let idle = MacIdle.idle().expect("CoreGraphics answers");
        assert!(idle < Duration::from_secs(365 * 24 * 3600));
    }

    /// A Mac without a battery (a Mac mini, say) reads as mains.
    #[test]
    fn real_battery_probe_returns_something_sane() {
        let state = MacBattery.read();
        if let Some(p) = state.percent {
            assert!(p <= 100);
        } else {
            assert!(state.on_ac);
        }
        // No battery fitted is always mains, with nothing to estimate.
        if !state.present {
            assert!(state.on_ac);
            assert_eq!(state.minutes_left, None);
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
