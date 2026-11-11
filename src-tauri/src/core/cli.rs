//! The command line, parsed. Pure: the app decides what each command does.
//!
//! ```text
//! keepmeawake --for 2h | --until 18:30 | --always | --off | --toggle
//! keepmeawake --status | --run CMD ARGS… | --help
//! ```

/// One command from the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cli {
    /// Awake for this many seconds.
    For {
        /// Length, in seconds.
        seconds: u64,
    },
    /// Awake until this time of day: today, or tomorrow if it has passed.
    Until {
        /// Hour, 0–23.
        hour: u32,
        /// Minute, 0–59.
        minute: u32,
    },
    /// Awake until turned off.
    Always,
    /// Let the machine sleep.
    Off,
    /// What clicking the sun does.
    Toggle,
    /// Print the status line.
    Status,
    /// Run a command and stay awake while it runs.
    Run {
        /// The program.
        program: String,
        /// Its arguments.
        args: Vec<String>,
    },
    /// Stay awake while this process lives (what `--run` asks the app).
    WhilePid {
        /// Its id.
        pid: u32,
    },
    /// Print the usage.
    Help,
}

/// Printed for `--help` and after a mistake.
pub const USAGE: &str = "\
Keep me awake, from a terminal.

  keepmeawake --for 2h          awake for two hours (30m, 90m, 1h30m)
  keepmeawake --until 18:30     awake until a time today, or tomorrow
  keepmeawake --always          awake until turned off
  keepmeawake --off             let it sleep
  keepmeawake --toggle          what clicking the sun does
  keepmeawake --status          say what it is doing
  keepmeawake --run CMD ARGS…   awake while CMD runs; exits with its code
  keepmeawake --help            this
";

/// Longest `--for`: a week, as the default length allows.
pub const MAX_FOR_SECS: u64 = 7 * 24 * 60 * 60;

/// Parses the arguments after the program name. `Ok(None)` is an ordinary
/// launch: no arguments, or only the autostart's `--hidden`.
///
/// # Errors
///
/// A message saying what was wrong, to print above [`USAGE`].
pub fn parse(args: &[String]) -> Result<Option<Cli>, String> {
    let args: Vec<&str> = args
        .iter()
        .map(String::as_str)
        // macOS adds `-psn_…` to apps opened from Finder on older systems.
        .filter(|a| *a != "--hidden" && !a.starts_with("-psn_"))
        .collect();
    let Some((&first, rest)) = args.split_first() else {
        return Ok(None);
    };
    let one = |cli: Cli| {
        if rest.is_empty() {
            Ok(Some(cli))
        } else {
            Err(format!("{first} takes nothing after it"))
        }
    };
    let value = || match rest {
        [value] => Ok(*value),
        [] => Err(format!("{first} needs a value")),
        _ => Err(format!("{first} takes one value")),
    };
    match first {
        "--for" => {
            let seconds = duration(value()?)?;
            Ok(Some(Cli::For { seconds }))
        }
        "--until" => {
            let (hour, minute) = time_of_day(value()?)?;
            Ok(Some(Cli::Until { hour, minute }))
        }
        "--always" => one(Cli::Always),
        "--off" => one(Cli::Off),
        "--toggle" => one(Cli::Toggle),
        "--status" => one(Cli::Status),
        "--help" | "-h" => one(Cli::Help),
        "--run" => match rest.split_first() {
            Some((program, args)) => Ok(Some(Cli::Run {
                program: (*program).to_owned(),
                args: args.iter().map(|a| (*a).to_owned()).collect(),
            })),
            None => Err("--run needs a command".to_owned()),
        },
        "--while-pid" => {
            let pid = value()?
                .parse::<u32>()
                .ok()
                .filter(|pid| *pid > 0)
                .ok_or_else(|| "--while-pid needs a process id".to_owned())?;
            Ok(Some(Cli::WhilePid { pid }))
        }
        other => Err(format!("{other} is not something it understands")),
    }
}

/// `2h`, `90m`, `1h30m`, `45` (minutes). Between a minute and a week.
fn duration(text: &str) -> Result<u64, String> {
    let bad = || format!("{text} is not a length: try 30m, 2h or 1h30m");
    let text = text.trim().to_ascii_lowercase();
    let mut seconds: u64 = 0;
    let mut number = String::new();
    let mut any_unit = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let unit = match c {
            'h' => 3600,
            'm' => 60,
            _ => return Err(bad()),
        };
        let n: u64 = number.parse().map_err(|_| bad())?;
        seconds = seconds.saturating_add(n.saturating_mul(unit));
        number.clear();
        any_unit = true;
    }
    if !number.is_empty() {
        // A bare number, or minutes after hours (`1h30`).
        let n: u64 = number.parse().map_err(|_| bad())?;
        seconds = seconds.saturating_add(n.saturating_mul(60));
    } else if !any_unit {
        return Err(bad());
    }
    if !(60..=MAX_FOR_SECS).contains(&seconds) {
        return Err(format!("{text} is outside a minute to a week"));
    }
    Ok(seconds)
}

/// `18:30`, `6:30pm`, `6pm`, `0930`. A 24-hour time or a 12-hour one.
fn time_of_day(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("{text} is not a time: try 18:30 or 6:30pm");
    let lower = text.trim().to_ascii_lowercase();
    let (clock, meridiem) = if let Some(clock) = lower.strip_suffix("pm") {
        (clock.trim(), Some(true))
    } else if let Some(clock) = lower.strip_suffix("am") {
        (clock.trim(), Some(false))
    } else {
        (lower.as_str(), None)
    };
    let (hour, minute) = match clock.split_once(':') {
        Some((h, m)) => (h.parse::<u32>(), m.parse::<u32>()),
        None if clock.len() == 4 && meridiem.is_none() => {
            (clock[..2].parse::<u32>(), clock[2..].parse::<u32>())
        }
        None => (clock.parse::<u32>(), Ok(0)),
    };
    let (Ok(mut hour), Ok(minute)) = (hour, minute) else {
        return Err(bad());
    };
    if let Some(pm) = meridiem {
        if !(1..=12).contains(&hour) {
            return Err(bad());
        }
        hour %= 12;
        if pm {
            hour += 12;
        }
    }
    if hour > 23 || minute > 59 {
        return Err(bad());
    }
    Ok((hour, minute))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Option<Cli>, String> {
        parse(&args.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn an_ordinary_launch_is_not_a_command() {
        assert_eq!(p(&[]), Ok(None));
        assert_eq!(p(&["--hidden"]), Ok(None));
    }

    #[test]
    fn lengths_in_hours_and_minutes() {
        let secs = |s: &str| match p(&["--for", s]) {
            Ok(Some(Cli::For { seconds })) => seconds,
            other => panic!("{s}: {other:?}"),
        };
        assert_eq!(secs("2h"), 7200);
        assert_eq!(secs("90m"), 5400);
        assert_eq!(secs("1h30m"), 5400);
        assert_eq!(secs("1h30"), 5400);
        assert_eq!(secs("45"), 2700);
        assert_eq!(secs("2H"), 7200);
        for bad in ["", "2x", "h", "30s", "0m", "8d", "200h"] {
            assert!(p(&["--for", bad]).is_err(), "{bad}");
        }
        assert!(p(&["--for"]).is_err());
        assert!(p(&["--for", "1h", "2h"]).is_err());
    }

    #[test]
    fn times_in_either_clock() {
        let at = |s: &str| match p(&["--until", s]) {
            Ok(Some(Cli::Until { hour, minute })) => (hour, minute),
            other => panic!("{s}: {other:?}"),
        };
        assert_eq!(at("18:30"), (18, 30));
        assert_eq!(at("6:30pm"), (18, 30));
        assert_eq!(at("6pm"), (18, 0));
        assert_eq!(at("12am"), (0, 0));
        assert_eq!(at("12:15pm"), (12, 15));
        assert_eq!(at("0930"), (9, 30));
        assert_eq!(at("7"), (7, 0));
        for bad in ["24:00", "13pm", "6:61", "noon", "0pm"] {
            assert!(p(&["--until", bad]).is_err(), "{bad}");
        }
    }

    #[test]
    fn switches_take_nothing_after_them() {
        assert_eq!(p(&["--always"]), Ok(Some(Cli::Always)));
        assert_eq!(p(&["--off"]), Ok(Some(Cli::Off)));
        assert_eq!(p(&["--toggle"]), Ok(Some(Cli::Toggle)));
        assert_eq!(p(&["--status"]), Ok(Some(Cli::Status)));
        assert_eq!(p(&["-h"]), Ok(Some(Cli::Help)));
        assert!(p(&["--off", "now"]).is_err());
        assert!(p(&["--sleep"]).is_err());
    }

    #[test]
    fn run_keeps_the_rest_as_the_command() {
        assert_eq!(
            p(&["--run", "cargo", "build", "--release"]),
            Ok(Some(Cli::Run {
                program: "cargo".into(),
                args: vec!["build".into(), "--release".into()],
            }))
        );
        assert!(p(&["--run"]).is_err());
    }

    #[test]
    fn a_pid_must_be_a_real_one() {
        assert_eq!(
            p(&["--while-pid", "4120"]),
            Ok(Some(Cli::WhilePid { pid: 4120 }))
        );
        assert!(p(&["--while-pid", "0"]).is_err());
        assert!(p(&["--while-pid", "x"]).is_err());
    }
}
