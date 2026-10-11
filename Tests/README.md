# Tests

```powershell
pwsh -File Tests\Run-AllTests.ps1             # build + tests that don't open the app
pwsh -File Tests\Run-AllTests.ps1 -WithUI     # also the UI tests, they open the app and click through it
pwsh -File Tests\UI\Run-UITests.ps1 -SkipNetwork -SkipBrowser   # UI tests only, offline
```

Requirements: Visual Studio 2022 or its Build Tools (MSBuild) and PowerShell 7 (`pwsh`).

## Unit tests (`Tests\Unit`)

A small console runner (no test framework needed) that loads the built `RobloxAccountManager.exe` and checks,
without opening any window:

- translations: every language has the same keys, keeps `{0}` placeholders, has no empty texts, templates work;
- every UI text written in the source code has a translation;
- theme upgrade, settings file, password encryption/decryption;
- every icon used by a button or menu is embedded;
- the app's own logic: language choice (a saved pt-BR falls back to pt-PT, Portuguese systems start in pt-PT),
  saving accounts (never skipped, never overwritten by an empty list, a backup is kept), alias/description limits.

Everything they write goes to a temporary folder that is deleted afterwards.

## UI tests (`Tests\UI`, only with `-WithUI`)

`Run-UITests.ps1` starts a copy of the build in `Tests\.run\app` with 6 fake accounts and goes through every
button, menu, submenu, setting, the theme editor, the utilities and account utilities windows, the web server,
password protection and every language. Results (with screenshots) go to `Tests\.run\results\report.md`.

- Your accounts and settings are never used: the copy runs in its own folder.
- Input is sent as window messages to the app's own windows, so nothing else on your desktop is clicked
  (the app's windows do pop up while they run).
- The browser tests start the app's own Chrome (in `%LOCALAPPDATA%\RobloxAccountManager\Chrome`), never yours.
- Left out on purpose: the Discord button, turning on the Roblox Watcher if its Sysinternals EULA was already
  accepted (it could close a running Roblox), and "Run on Windows Startup" if an entry with that name already exists.
- Your clipboard is restored at the end, and a startup entry left by a failed test is removed.
