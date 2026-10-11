# Roblox Account Manager 4 (Rust)

The Rust + Tauri version of the app. It reads and writes the same files as the C# version
(`AccountData.json`, `RAMSettings.ini`, `RAMTheme.ini`, the language tables), so either can open the other's accounts.
The two never run at the same time: they share the same single-instance lock.

## Build

Requirements: [Rust](https://rustup.rs), Node.js 20+, WebView2 (included with Windows 10/11).

```
npm --prefix ui install
npm --prefix ui run build
cargo build --release
```

`target/release/` then has the two files to ship side by side:

| File | What it is |
| --- | --- |
| `RobloxAccountManager.exe` | the app |
| `ram-cli.exe` | the command line, see below |

During development `npm --prefix ui run dev` serves the interface at localhost:1420 with hot reload, and a debug build (`cargo build`) uses it.

## Layout

```
src/
  app.rs        state shared by everything
  core/         file formats shared with C#: settings, accounts, encryption, languages
  roblox/       the Roblox web APIs
  platform/     Windows: processes, windows, mutexes, ping, log file
  services/     launching Roblox, browser, web API, watcher, Anti-AFK
  commands/     what the interface can ask for, one file per part of the interface
  cli/          ram-cli: options, commands, the pipe to the open app
ui/src/
  components/   one folder per part of the interface (shadcn/ui underneath)
  hooks/ lib/   actions, store, translations, theme
tests/          ui.mjs (interface), cli.mjs (command line), fixtures
tools/          strings.py (translation check)
```

## Command line

```
ram-cli list [--group <name>] [--json]
ram-cli join <accounts> [--place <id or game link>] [--job <job id or VIP link>]
ram-cli follow <accounts> --user <username>
ram-cli close <accounts> | --all
ram-cli import [file]
ram-cli antiafk [start|stop|now|status]
ram-cli status
ram-cli open
```

Accounts are named by username, user id or alias. `--all` picks every account and `--group <name>` picks a group.
`ram-cli help` explains each command.

- When the app is open, the command runs inside it through a named pipe that only the same Windows user can use. The window updates live, and Anti-AFK can be started and stopped.
- When the app isn't open, ram-cli opens the accounts file itself, without a window. A password-protected file asks for the password, or reads it from `RAM_PASSWORD`.
- Exit codes: `0` done, `1` failed, `2` wrong command line. `list --json` never includes cookies.

## Tests

```
cargo test --lib                 # file formats, encryption (including a file encrypted by the C# version), ini, dates
node tests/cli.mjs [--release]   # command line, alone and through an open app
node tests/ui.mjs  [--release]   # the interface, through the WebView2 DevTools protocol
```

The interface and command line tests use demo accounts in `Tests/.run/`. They never launch Roblox, close it or press keys.
The interface tests save screenshots in `Tests/.run/rust/screenshots`.

## Translations

The texts are the English keys of `RBX Alt Manager/Resources/Lang/*.json`, the same tables the C# version uses.
`python tools/strings.py` lists every text shown by the interface or sent by Rust that has no translation yet.
