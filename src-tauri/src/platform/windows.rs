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

use super::{ActivityProbe, BatteryProbe, IdleProbe, PlatformError, PowerGuard, TaskbarTheme};
use crate::core::watch::CpuTicks;
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
                present: false,
                minutes_left: None,
            };
        }
        decode_power_status(
            status.ACLineStatus,
            status.BatteryFlag,
            status.BatteryLifePercent,
            status.BatteryLifeTime,
        )
    }
}

/// What the PC is doing, for stay awake while…. Every reading is a single
/// read-only call; nothing here touches another process.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsActivity;

/// `IF_TYPE_SOFTWARE_LOOPBACK`: the loopback adapter's `Type`.
const LOOPBACK: u32 = 24;

/// The names of every process running now. Also what the app picker
/// offers, so it needs no probe of its own.
pub fn list_processes() -> Vec<String> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    // SAFETY: plain flags in; the handle is closed below on every path.
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).unwrap_or(0),
        ..PROCESSENTRY32W::default()
    };
    // SAFETY: `entry` is a valid, writable struct whose `dwSize` says
    // how large it is, as the toolhelp functions require.
    let mut more = unsafe { Process32FirstW(snapshot, &raw mut entry) }.is_ok();
    while more {
        names.push(wide_to_string(&entry.szExeFile));
        // SAFETY: as above; the snapshot handle is still open.
        more = unsafe { Process32NextW(snapshot, &raw mut entry) }.is_ok();
    }
    // SAFETY: the snapshot handle came from CreateToolhelp32Snapshot and
    // is closed exactly once.
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    names
}

impl ActivityProbe for WindowsActivity {
    fn processes(&self) -> Vec<String> {
        list_processes()
    }

    fn alive(&self, pid: u32) -> bool {
        use windows::Win32::Foundation::STILL_ACTIVE;
        use windows::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        if pid == 0 {
            return false;
        }
        // SAFETY: plain values in; the handle is closed below.
        let Ok(process) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
        else {
            return false;
        };
        let mut code: u32 = 0;
        // SAFETY: `process` is an open handle with query rights and `code`
        // is a writable u32.
        let read = unsafe { GetExitCodeProcess(process, &raw mut code) }.is_ok();
        // SAFETY: opened above, closed exactly once.
        unsafe {
            let _ = CloseHandle(process);
        }
        read && i32::try_from(code).is_ok_and(|code| code == STILL_ACTIVE.0)
    }

    fn transferred(&self) -> Option<u64> {
        use windows::Win32::NetworkManagement::IpHelper::{
            FreeMibTable, GetIfTable2, MIB_IF_TABLE2,
        };
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        // SAFETY: `table` is a writable pointer the call fills with a table
        // it allocates; it is freed with FreeMibTable below.
        if unsafe { GetIfTable2(&raw mut table) }.is_err() || table.is_null() {
            return None;
        }
        // SAFETY: on success `table` points at a MIB_IF_TABLE2 whose `Table`
        // runs on for `NumEntries` rows (a C flexible array).
        let rows = unsafe {
            let count = usize::try_from((*table).NumEntries).unwrap_or(0);
            std::slice::from_raw_parts(
                (&raw const (*table).Table)
                    .cast::<windows::Win32::NetworkManagement::IpHelper::MIB_IF_ROW2>(),
                count,
            )
        };
        let total = rows
            .iter()
            // Real adapters only: loopback, VPNs and virtual switches would
            // count the same bytes twice.
            .filter(|row| row.Type != LOOPBACK && row.InterfaceAndOperStatusFlags._bitfield & 1 == 1)
            .fold(0_u64, |sum, row| {
                sum.saturating_add(row.InOctets).saturating_add(row.OutOctets)
            });
        // SAFETY: allocated by GetIfTable2, freed exactly once, not used after.
        unsafe { FreeMibTable(table.cast_const().cast()) };
        Some(total)
    }

    fn cpu(&self) -> Option<CpuTicks> {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::GetSystemTimes;
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        // SAFETY: three valid, writable FILETIMEs.
        unsafe {
            GetSystemTimes(
                Some(&raw mut idle),
                Some(&raw mut kernel),
                Some(&raw mut user),
            )
        }
        .ok()?;
        Some(cpu_ticks(ticks(idle), ticks(kernel), ticks(user)))
    }

    fn full_screen(&self) -> bool {
        use windows::Win32::UI::Shell::{
            QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
            SHQueryUserNotificationState,
        };
        // SAFETY: no arguments; returns the state by value.
        unsafe { SHQueryUserNotificationState() }.is_ok_and(|state| {
            state == QUNS_BUSY
                || state == QUNS_RUNNING_D3D_FULL_SCREEN
                || state == QUNS_PRESENTATION_MODE
        })
    }

    fn external_display(&self) -> bool {
        use windows::Win32::Devices::Display::{
            DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED,
            DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS,
            DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED, DISPLAYCONFIG_PATH_INFO,
            GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
        };
        let mut paths_len: u32 = 0;
        let mut modes_len: u32 = 0;
        // SAFETY: two writable counts.
        let sized = unsafe {
            GetDisplayConfigBufferSizes(
                QDC_ONLY_ACTIVE_PATHS,
                &raw mut paths_len,
                &raw mut modes_len,
            )
        };
        if sized.is_err() {
            return false;
        }
        let mut paths =
            vec![DISPLAYCONFIG_PATH_INFO::default(); usize::try_from(paths_len).unwrap_or(0)];
        let mut modes =
            vec![DISPLAYCONFIG_MODE_INFO::default(); usize::try_from(modes_len).unwrap_or(0)];
        // SAFETY: the buffers hold exactly the counts passed, which the call
        // may lower to what it wrote.
        let queried = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &raw mut paths_len,
                paths.as_mut_ptr(),
                &raw mut modes_len,
                modes.as_mut_ptr(),
                None,
            )
        };
        if queried.is_err() {
            return false;
        }
        paths.truncate(usize::try_from(paths_len).unwrap_or(0));
        paths.iter().any(|path| {
            let tech = path.targetInfo.outputTechnology;
            tech != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL
                && tech != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED
                && tech != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED
                && tech != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS
        })
    }
}

/// A FILETIME's two halves as one count of 100 ns ticks.
fn ticks(time: windows::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

/// Pure, tested: `GetSystemTimes`' three counts as busy and total. Kernel
/// time includes idle time, so busy is kernel plus user, less idle.
pub fn cpu_ticks(idle: u64, kernel: u64, user: u64) -> CpuTicks {
    let total = kernel.saturating_add(user);
    CpuTicks {
        busy: total.saturating_sub(idle),
        total,
    }
}

/// Pure, tested: a NUL-terminated UTF-16 buffer as a string.
pub fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
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
/// The count wraps every 49.7 days, so the subtraction wraps with it. An
/// input stamped a moment after the tick count was read (the two reads are
/// not atomic) would wrap to about 49 days; anything past half the range
/// is that, not idleness, and reads as just now.
pub fn idle_between(now_ticks: u32, last_input_ticks: u32) -> Duration {
    let elapsed = now_ticks.wrapping_sub(last_input_ticks);
    if elapsed > u32::MAX / 2 {
        return Duration::ZERO;
    }
    Duration::from_millis(u64::from(elapsed))
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
pub fn decode_power_status(ac_line: u8, flag: u8, percent: u8, life_secs: u32) -> BatteryState {
    // 128 is "no system battery"; 255 ("unknown") has that bit set too.
    let no_battery = flag & 128 != 0;
    let on_ac = ac_line == 1 || ac_line == 255 || no_battery;
    let percent = if no_battery || percent == 255 {
        None
    } else {
        Some(percent.min(100))
    };
    // `u32::MAX` is "unknown", which is also what Windows says on mains.
    let minutes_left = (!on_ac && life_secs != u32::MAX).then_some(life_secs / 60);
    BatteryState {
        on_ac,
        percent,
        present: !no_battery,
        minutes_left,
    }
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
            decode_power_status(1, 0, 77, u32::MAX),
            BatteryState {
                on_ac: true,
                percent: Some(77),
                present: true,
                minutes_left: None,
            }
        );
    }

    #[test]
    fn decodes_unplugged() {
        assert_eq!(
            decode_power_status(0, 0, 19, 11_400),
            BatteryState {
                on_ac: false,
                percent: Some(19),
                present: true,
                minutes_left: Some(190),
            }
        );
    }

    #[test]
    fn unknown_line_status_is_treated_as_mains() {
        assert!(decode_power_status(255, 0, 50, u32::MAX).on_ac);
    }

    #[test]
    fn desktops_without_battery_are_always_on_mains() {
        assert_eq!(
            decode_power_status(0, 128, 255, u32::MAX),
            BatteryState {
                on_ac: true,
                percent: None,
                present: false,
                minutes_left: None,
            }
        );
    }

    #[test]
    fn unknown_percent_is_none_and_values_are_capped() {
        assert_eq!(decode_power_status(1, 0, 255, u32::MAX).percent, None);
        assert_eq!(decode_power_status(1, 0, 120, u32::MAX).percent, Some(100));
    }

    #[test]
    fn processor_time_counts_kernel_without_idle() {
        assert_eq!(
            cpu_ticks(600, 800, 200),
            CpuTicks {
                busy: 400,
                total: 1000
            }
        );
    }

    #[test]
    fn wide_names_stop_at_the_terminator() {
        let mut name = [0_u16; 8];
        for (slot, c) in name.iter_mut().zip("zsh.exe".encode_utf16()) {
            *slot = c;
        }
        assert_eq!(wide_to_string(&name), "zsh.exe");
        assert_eq!(wide_to_string(&[]), "");
    }

    /// The real readings answer, and this test process is alive.
    #[test]
    fn real_activity_readings_answer() {
        let activity = WindowsActivity;
        assert_ne!(activity.processes().len(), 0);
        assert!(activity.alive(std::process::id()));
        assert!(!activity.alive(0));
        assert!(activity.cpu().is_some_and(|t| t.total >= t.busy));
        let _ = activity.transferred();
        let _ = activity.full_screen();
        let _ = activity.external_display();
    }

    #[test]
    fn time_left_only_on_battery_and_only_when_known() {
        assert_eq!(decode_power_status(0, 0, 50, u32::MAX).minutes_left, None);
        assert_eq!(decode_power_status(1, 0, 50, 3600).minutes_left, None);
        assert_eq!(decode_power_status(0, 0, 50, 3659).minutes_left, Some(60));
        assert!(!decode_power_status(0, 255, 50, 3600).present);
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
        // Input stamped just after the tick count was read: just now, not
        // 49.7 days.
        assert_eq!(idle_between(1_000, 1_016), Duration::ZERO);
    }

    /// Reads the real last-input time. Asserts that it answers and is no
    /// longer than half the tick count's range, the most it reports.
    #[test]
    fn real_idle_reading_answers_and_is_sane() {
        let idle = WindowsIdle.idle().expect("GetLastInputInfo answers");
        assert!(idle <= Duration::from_millis(u64::from(u32::MAX / 2)));
    }

    #[test]
    fn real_battery_probe_returns_something_sane() {
        let state = WindowsBattery.read();
        if let Some(p) = state.percent {
            assert!(p <= 100);
        }
    }
}
