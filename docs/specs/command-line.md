# Command line

Drive the running app from a terminal or a script, and keep the laptop
awake for exactly as long as a command runs.

## Why

`caffeinate -i make` is why developers keep a terminal habit for this. The
app already has the engine; the command line should reach it.

## Experience

```text
keepmeawake --for 2h          awake for two hours (30m, 90m, 1h30m)
keepmeawake --until 18:30     awake until a time today, or tomorrow
keepmeawake --always          awake until turned off
keepmeawake --off             let it sleep
keepmeawake --toggle          what clicking the sun does
keepmeawake --status          one line: Awake until 6:30 pm · 1h 12m left
keepmeawake --run CMD ARGS…   awake while CMD runs; exits with its code
keepmeawake --help
```

Each command reaches the running app (starting it hidden if it is not
running) and returns at once. `--status` prints the tray's status line,
read from a small file the app writes on every change of state. `--run`
starts the command, asks the app to stay awake while this terminal
process lives (`--while-pid`, internal), waits, and exits with the
command's code; quitting it with Ctrl+C releases the laptop as well.

A bad argument prints the usage and exits with 2. Nothing here opens the
window.

| Platform | The command                                                                                                 |
| -------- | ----------------------------------------------------------------------------------------------------------- |
| Windows  | the installed `keepmeawake.exe`; the guide shows how to add its folder to PATH                              |
| macOS    | `/Applications/KeepMeAwake.app/Contents/MacOS/keepmeawake`; the guide shows a `ln -s` into `/usr/local/bin` |

## Rust

- `core::cli::parse(&[String]) -> Result<Cli, String>`, pure: durations
  (`2h`, `90m`, `1h30m`, plain minutes), times (`18:30`, `6:30pm`).
- `app/cli.rs`: `--status` and `--help` are answered before Tauri starts.
  Everything else starts Tauri as usual: the single-instance plugin hands
  the arguments to the running app, whose callback applies them through
  the same engine calls as the tray, without showing the window. With no
  app running, this process becomes the app, launched hidden, and applies
  them after setup.
- `--run` spawns the command, then launches this executable again with
  `--while-pid <own pid>` and waits; `Engine::watch_pid` (from
  [stay-awake-while](stay-awake-while.md)) holds while the pid lives.
- The app writes `status.txt` (the status line, one line) next to the
  settings on each state change, not on countdown ticks.
- Windows: the release binary is a GUI program; `platform::attach_console`
  calls `AttachConsole(ATTACH_PARENT_PROCESS)` so output reaches the shell.

## Tests

`parse` for every form and error; status text; the single-instance
callback's dispatch.

## Resource impact

At rest: one small write per change of state, which is a handful a day.

## Acceptance

From a terminal on each OS: `--for 1m` raises the sun and it sets a minute
later; `--status` prints the line; `--run sleep 30` holds for 30 s and
returns 0.

## Commits

1. `feat(core): parse the command line`
2. `feat(platform): attach to the parent console on Windows`
3. `feat(app): answer the command line, write the status line`
4. `docs: the command line in the guide`

## Out of scope

JSON output, a shell completion script, controlling settings.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
