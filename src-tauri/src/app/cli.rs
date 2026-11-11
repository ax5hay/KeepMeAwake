//! The command line: what is answered here before the app starts, what is
//! handed to the running app, and the status line `--status` reads.
//!
//! `--status`, `--help` and `--run` never start the app's window: they are
//! answered in this process. Every other command starts Tauri as usual; the
//! single-instance plugin passes it to the running app, or this process
//! becomes the app (launched hidden) and applies it after setup.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use chrono::{Duration as Days, Local, NaiveTime, TimeZone};
use parking_lot::Mutex;

use crate::core::cli::{self, Cli, USAGE};
use crate::core::{Mode, PlanRequest, Snapshot};
use crate::engine::Engine;

use super::tray;

/// The app's settings folder, as Tauri's `app_config_dir` names it, worked
/// out without Tauri so `--status` can answer before anything starts.
fn config_dir() -> Option<PathBuf> {
    const ID: &str = "dev.bajpai.keepmeawake";
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
    });
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(any(windows, target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|base| base.join(ID))
}

/// Where the running app keeps its one status line.
fn status_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("status.txt"))
}

/// The command in `argv` (program name first), if there is one. A command
/// that does not parse was already answered by [`early`], so it is `None`.
pub fn command(argv: &[String]) -> Option<Cli> {
    cli::parse(argv.get(1..).unwrap_or_default()).ok().flatten()
}

/// Answers what needs no app, before Tauri starts. `Some(code)` means exit
/// with it now; `None`, start the app (which applies any other command).
pub fn early() -> Option<i32> {
    let argv: Vec<String> = std::env::args().collect();
    let parsed = cli::parse(argv.get(1..).unwrap_or_default());
    if !matches!(parsed, Ok(None)) {
        attach_console();
    }
    match parsed {
        // An ordinary launch, or a command the running app carries out.
        Ok(
            None
            | Some(
                Cli::For { .. }
                | Cli::Until { .. }
                | Cli::Always
                | Cli::Off
                | Cli::Toggle
                | Cli::WhilePid { .. },
            ),
        ) => None,
        Err(why) => {
            eprintln!("{why}\n\n{USAGE}");
            Some(2)
        }
        Ok(Some(Cli::Help)) => {
            print!("{USAGE}");
            Some(0)
        }
        Ok(Some(Cli::Status)) => {
            let saved = status_path().and_then(|path| fs::read_to_string(path).ok());
            let line = saved
                .as_deref()
                .and_then(read_status)
                .filter(|(pid, _)| pid_alive(*pid))
                .map_or("Keep me awake is not running.", |(_, line)| line);
            println!("{line}");
            Some(0)
        }
        Ok(Some(Cli::Run { program, args })) => Some(run(&program, &args)),
    }
}

/// `--run`: start the command, ask the app to stay awake while this
/// process lives, wait, and hand back the command's exit code.
fn run(program: &str, args: &[String]) -> i32 {
    let mut child = match Command::new(program).args(args).spawn() {
        Ok(child) => child,
        Err(err) => {
            eprintln!("could not start {program}: {err}");
            return 127;
        }
    };
    // Another launch of this program carries the pid to the app (starting
    // it, hidden, if it is not running). Not waited for: it either hands
    // over and exits, or it is the app.
    match std::env::current_exe() {
        Ok(exe) => {
            let handed = Command::new(exe)
                .args(["--while-pid", &std::process::id().to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            if let Err(err) = handed {
                eprintln!("could not reach Keep me awake: {err}");
            }
        }
        Err(err) => eprintln!("could not reach Keep me awake: {err}"),
    }
    match child.wait() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!("lost {program}: {err}");
            1
        }
    }
}

/// Does what the command says, through the same engine calls as the tray.
pub fn apply(engine: &Engine, cli: &Cli) {
    log::info!("command line: {cli:?}");
    match cli {
        Cli::For { seconds } => {
            engine.set_plan(PlanRequest::For { seconds: *seconds });
        }
        Cli::Until { hour, minute } => {
            if let Some(unix_ms) = next_at(*hour, *minute) {
                engine.set_plan(PlanRequest::Until { unix_ms });
            }
        }
        Cli::Always => {
            engine.set_plan(PlanRequest::Indefinite);
        }
        Cli::Off => {
            engine.set_plan(PlanRequest::Off);
        }
        Cli::Toggle => {
            engine.toggle();
        }
        Cli::WhilePid { pid } => {
            engine.watch_pid(*pid);
        }
        // Answered before the app starts.
        Cli::Status | Cli::Help | Cli::Run { .. } => {}
    }
}

/// The next `hour:minute` in local time: today, or tomorrow if it passed.
fn next_at(hour: u32, minute: u32) -> Option<u64> {
    let now = Local::now();
    let time = NaiveTime::from_hms_opt(hour, minute, 0)?;
    let today = Local
        .from_local_datetime(&now.date_naive().and_time(time))
        .earliest()?;
    let at = if today > now {
        today
    } else {
        today.checked_add_signed(Days::days(1))?
    };
    u64::try_from(at.timestamp_millis()).ok()
}

/// The line `--status` prints. A timer says when it ends rather than how
/// long is left, since the file is written on changes, not every second.
pub fn status_line(snapshot: &Snapshot) -> String {
    match (snapshot.mode, snapshot.paused, snapshot.ends_at_unix_ms) {
        (Mode::Timed, None, Some(end)) => {
            format!("Awake until {}", tray::when(end, tray::now_unix_ms()))
        }
        _ => tray::status_line(snapshot),
    }
}

/// The last line written, so an unchanged state is not written again.
static WRITTEN: Mutex<Option<String>> = Mutex::new(None);

/// Keeps `status.txt` in step with the app. Called for every snapshot;
/// writes only when the line changed: a handful of times a day.
pub fn note(snapshot: &Snapshot) {
    let line = status_line(snapshot);
    let mut written = WRITTEN.lock();
    if written.as_deref() == Some(line.as_str()) {
        return;
    }
    let Some(path) = status_path() else {
        return;
    };
    let saved = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::File::create(&path))
        .and_then(|mut file| writeln!(file, "{}\n{line}", std::process::id()));
    match saved {
        Ok(()) => *written = Some(line),
        Err(err) => log::debug!("could not write the status line: {err}"),
    }
}

/// Pure, tested: the app's pid and its status line from `status.txt`.
fn read_status(text: &str) -> Option<(u32, &str)> {
    let (pid, line) = text.split_once('\n')?;
    Some((pid.trim().parse().ok()?, line.trim()))
}

/// On quit: no file, so `--status` says the app is not running.
/// (A killed app leaves its file; its pid then says it is gone.)
pub fn forget() {
    if let Some(path) = status_path() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(target_os = "macos")]
fn pid_alive(pid: u32) -> bool {
    crate::platform::macos::pid_alive(pid)
}

#[cfg(windows)]
fn pid_alive(pid: u32) -> bool {
    crate::platform::windows::pid_alive(pid)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn pid_alive(_pid: u32) -> bool {
    false
}

#[cfg(windows)]
fn attach_console() {
    crate::platform::windows::attach_console();
}

#[cfg(not(windows))]
fn attach_console() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_is_read_after_the_program_name() {
        let argv = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(command(&argv(&["keepmeawake"])), None);
        assert_eq!(command(&argv(&["keepmeawake", "--hidden"])), None);
        assert_eq!(command(&argv(&["keepmeawake", "--off"])), Some(Cli::Off));
        assert_eq!(command(&argv(&["keepmeawake", "--nonsense"])), None);
        assert_eq!(command(&[]), None);
    }

    #[test]
    fn the_next_time_of_day_is_ahead_and_within_a_day() {
        let now = u64::try_from(Local::now().timestamp_millis()).unwrap();
        for (hour, minute) in [(0, 0), (12, 30), (23, 59)] {
            let at = next_at(hour, minute).unwrap();
            assert!(at > now, "{hour}:{minute} is ahead");
            assert!(at - now <= 25 * 3600 * 1000, "{hour}:{minute} within a day");
        }
    }

    #[test]
    fn the_status_file_carries_the_pid_then_the_line() {
        assert_eq!(
            read_status("4120\nSleeping normally\n"),
            Some((4120, "Sleeping normally"))
        );
        assert_eq!(read_status("Sleeping normally"), None);
        assert_eq!(read_status("x\nAwake"), None);
        assert!(pid_alive(std::process::id()));
    }

    #[test]
    fn the_status_file_lives_with_the_settings() {
        let path = status_path().unwrap();
        assert!(
            path.ends_with("dev.bajpai.keepmeawake/status.txt")
                || path.ends_with("dev.bajpai.keepmeawake\\status.txt")
        );
    }
}
