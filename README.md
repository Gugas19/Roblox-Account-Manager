<div align="center">

<img src="icons/icon.svg" width="96" alt="">

# Roblox Account Manager

**A modern, translated, Rust rewrite of [ic3w0lf22's Roblox Account Manager](https://github.com/ic3w0lf22/Roblox-Account-Manager), with Anti-AFK and a command line built in**

[![Download](https://img.shields.io/github/v/release/Gugas19/Roblox-Account-Manager?label=Download&style=for-the-badge&logo=windows&color=4F7CFF)](https://github.com/Gugas19/Roblox-Account-Manager/releases/latest)
[![License](https://img.shields.io/badge/license-GPLv3-blue?style=for-the-badge)](LICENSE)
![Platform](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6?style=for-the-badge)
![Rust](https://img.shields.io/badge/Rust-Tauri%202-B7410E?style=for-the-badge&logo=rust)
[![CI](https://img.shields.io/github/actions/workflow/status/Gugas19/Roblox-Account-Manager/ci.yml?branch=master&label=CI&style=for-the-badge)](https://github.com/Gugas19/Roblox-Account-Manager/actions/workflows/ci.yml)

![Roblox Account Manager](Images/main.png)

[Download](#download) · [Highlights](#highlights) · [Command line](#command-line) · [Languages](#languages) · [Screenshots](#screenshots) · [Building](#building-from-source) · [FAQ](#frequently-asked-questions)

</div>

Keep all your Roblox accounts in one place and join games with any of them in one click: alt accounts, storage
accounts, grinding accounts, with as many Roblox windows open at the same time as you like. Leave them in game for
hours with the built in Anti-AFK, or drive everything from scripts with `ram-cli`.

## Download

1. From the [latest release](https://github.com/Gugas19/Roblox-Account-Manager/releases/latest), download
   `RobloxAccountManager.exe` (just the app) or the zip (the app, `ram-cli.exe` and the license).
2. Put it in a folder of its own, for example on your desktop (extract the zip first, it won't run from inside it).
3. Run `RobloxAccountManager.exe`.

The files in a release are built by GitHub from this repository, not on anyone's PC, and GitHub publicly records it
(a [build provenance attestation](https://docs.github.com/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds)).
To check a download with the [GitHub CLI](https://cli.github.com/):

```
gh attestation verify RobloxAccountManager.exe --repo Gugas19/Roblox-Account-Manager
```

In the zip, besides the license:

| File | What it is |
| --- | --- |
| `RobloxAccountManager.exe` | the app, about 6 MB |
| `ram-cli.exe` | the [command line](#command-line) |

- It uses the WebView2 runtime that comes with Windows 10 and 11. Nothing else to install: no .NET, and no bundled browser engine.
- From version 4.0.1 on it [tells you about new versions and updates in one click](#updates). Version 4.0.0 has to be replaced by hand once.
- The browser used to add accounts (Chrome for Testing) is downloaded automatically the first time you need it, into
  `%LOCALAPPDATA%\RobloxAccountManager\Chrome`. A progress bar shows while it downloads.

### Coming from version 3 or the original app

Copy your `AccountData.json`, `RAMSettings.ini` and `RAMTheme.ini` next to `RobloxAccountManager.exe`.
Your accounts, groups, aliases, settings and theme carry over as they are. That includes password-protected files,
which open with the same password, and the local web API, which answers the same requests (so
[`RAMAccount.lua`](RAMAccount.lua) and other scripts keep working, as long as they send the web API password for
the methods that change something, see [Developer API](#developer-api)). Don't run both versions on the same files at
the same time: they refuse to start while the other is open.

## Highlights

### A modern interface
- Built with [shadcn/ui](https://ui.shadcn.com): dark by default, with more [themes](#themes) and colors of your own.
- **Command palette** (`Ctrl+K`): find any account or action and run it from the keyboard.
- Search (`Ctrl+F`), select all (`Ctrl+A`), join with `Enter` or a double click, remove with `Delete`.
- **Undo** after removing accounts, and double click an alias to edit it in place.
- Paste a game link straight into Place ID. Recent games are one click away.
- A progress bar with a cancel button while several accounts launch.
- Drag the divider between the account list and the side panel to make either one wider (double click it to go back).
- A guided first start, and every setting explained where you change it. Changes apply instantly, no restart.

### Themes
- Four ready-made themes, **Dark** (the default), **Midnight**, **Forest** and **Light**, one click each in the theme
  editor (the palette button in the top right).
- Or make your own: change the accent, background, text, cards and account list, buttons and text boxes, each with a
  color picker. Every change shows instantly, and `Reset to the default theme` goes back to Dark.
- Give accounts and groups a color of their own from the right click menus, to tell them apart at a glance.
- Your theme is kept in `RAMTheme.ini`, so a theme from the original app or version 3 carries over.

### Anti-AFK built in
Roblox kicks you after 20 minutes without input. Anti-AFK keeps every Roblox window active, without getting in your way:
- **It doesn't interrupt you**: it waits until you stop typing and moving the mouse, skips the window you're playing in,
  never acts while a full screen game or video is in front, and leaves your windows as they were afterwards: the one
  you were using back in front, the others stacked in the same order, minimized Roblox windows minimized again.
  (Roblox only accepts input while it has the focus: keys and mouse sent in the background are ignored, we tested them
  all, so a short switch to the Roblox window is the only way.)
- **Rejoins disconnected accounts** (on by default, in Rejoin & extras): it reads the Roblox log, waits for the internet to
  come back if it dropped, closes the disconnected window and launches that account into the same game again, into the
  same server if you want (a private server too, like Roblox's Reconnect). When that server closed (a game update, for
  example) or is full, it joins any server of the game instead. It also works for windows opened from the Roblox app or
  website, as long as the account is in the list.
- Choice of key (jump, walk, turn the camera, zoom), interval (fixed, or random between two limits so the rounds aren't regular), presses per window, and resetting the character every round.
- Mute the windows you're not looking at, put background windows in Windows' efficiency mode (less CPU and fan noise),
  arrange them in a grid, hide them or make them see-through.
- Discord notifications when it starts, stops, an account disconnects or something fails.
- Statistics (resetting them keeps the old ones in a history, delete each one when you like), starts on its own when Roblox opens or when you're away, keeps the PC awake, keeps running in the tray.

### Command line
`ram-cli.exe` joins, follows, lists, closes and imports accounts, and controls Anti-AFK, from scripts, shortcuts or a
scheduled task. When the app is open the command runs inside it, so the window updates live. See [Command line](#command-line).

### Every window in your language
English, Português (Portugal), Español, Français and Deutsch. Switch at any time with the flag button in the top
right, and the whole app changes instantly.

### Updates
The app checks the [releases](https://github.com/Gugas19/Roblox-Account-Manager/releases) when it starts and every
30 minutes and tells you when there's a new version. On the first start it asks how you want updates to arrive. **Nothing is downloaded or installed until you click**: `Update` in
the top bar shows what's new in every version since yours (in the app's language), then downloads it, and `Restart to update` installs it. It
always goes straight to the newest version, even when yours is several behind. If you'd rather have it happen by itself, choose
`Automatic` in `Settings` > `General` (it then installs on the next start and never restarts while you use it), or
`Off` to never contact GitHub.
- Every release is signed with the developer's private key, and the app checks that signature before it keeps a
  download and again before it installs it. A changed download, or a release not signed with that key, is refused,
  even if it comes from this repository.
- The version is part of what's signed, so an old release can't be published again as a newer one, and only newer
  versions are installed.
- Only the two exes are taken from the download, and your accounts and settings are never touched.
- `Check now` in `Settings` > `General` looks right away. `ram-cli update` downloads it from the command line.

### Security
- Your accounts file is encrypted for your Windows user, or with a password if you choose one.
  A password-protected file works on any PC. Account passwords are only kept if you turn on *Save Passwords*.
- The local web API (off by default) only listens to this PC unless you allow other devices, refuses web pages open in
  your browser and other host names (so a website can't use it behind your back), and needs its password for anything
  that changes an account or reveals a cookie or token.
- The command line only talks to the app in its own folder, run by the same Windows user, and never prints cookies.
- Updates are installed only when you ask and only when they're signed with the developer's key (see [Updates](#updates)).
- Nothing else is downloaded at run time: Chrome for Testing only from Google's own storage address (its size and
  SHA-256 go to `log.txt`), and the watcher uses the patterns built into the app.
- The temporary browser profile used to log in (it holds that session's cookie) is deleted when the browser closes,
  and at the next start if the app crashed.
- An independent [security audit](https://github.com/ZackWitAK/Roblox-Account-Manager/blob/hardening/AUDIT.md) of
  version 4.1.13 found nothing malicious; its medium and low findings are fixed from 4.1.15 on.

## Command line

```
ram-cli list [--group <name>] [--json]
ram-cli join <accounts> [--place <id or game link>] [--job <job id or private server link>]
ram-cli follow <accounts> --user <username>
ram-cli close <accounts> | --all
ram-cli import [file]
ram-cli antiafk [start|stop|now|status]
ram-cli status
ram-cli open
ram-cli update
ram-cli cpu <accounts> <1-99|none>
```

Accounts are named by username, user id or alias (any case). `--all` picks every account, `--group <name>` a group.

```
ram-cli join Main Alt2 --place 920587237
ram-cli join --group Farm --place https://www.roblox.com/games/920587237
ram-cli join --group Farm                     # each account goes to its saved place
ram-cli follow Alt1 --user Builderman
ram-cli list --json
```

- With the app open, the command runs inside it.
- With the app closed, `ram-cli` opens the accounts file itself, without a window. A password-protected file asks
  for the password, or reads it from the `RAM_PASSWORD` environment variable. Anti-AFK needs the app open.
- Exit codes: `0` done, `1` failed, `2` wrong command line. `ram-cli help` explains each command.

## Languages

| | Language |
| :---: | :--- |
| <img src="ui/public/flags/gb.png" width="24"> | English |
| <img src="ui/public/flags/pt.png" width="24"> | Português (Portugal) |
| <img src="ui/public/flags/es.png" width="24"> | Español |
| <img src="ui/public/flags/fr.png" width="24"> | Français |
| <img src="ui/public/flags/de.png" width="24"> | Deutsch |

The app starts in your Windows language when it's one of these, otherwise in English.

| | |
| :---: | :---: |
| ![Português](Images/main-pt.png) | ![Español](Images/main-es.png) |
| ![Français](Images/main-fr.png) | ![Deutsch](Images/main-de.png) |

<details>
<summary>Adding or improving a translation</summary>

Translations live in [`lang/`](lang), one JSON file per language. The key is the English text and the value is its
translation. `{0}`, `{1}` are values filled in by the app and must be kept. To add a language, create `<code>.json`
with the same keys as `pt-PT.json`, add a flag to `ui/public/flags` and add the language to `LANGUAGES` in
[`src/core/lang.rs`](src/core/lang.rs). `python tools/strings.py` lists any text without a translation and checks
that every language has the same keys and placeholders.

</details>

## Screenshots

| | |
| :---: | :---: |
| ![Anti-AFK](Images/antiafk.png) Anti-AFK | ![Anti-AFK rejoin](Images/antiafk-rejoin.png) Anti-AFK: rejoin and extras |
| ![Command palette](Images/command-palette.png) Command palette (`Ctrl+K`) | ![Search](Images/search.png) Search |
| ![Account menu](Images/account-menu.png) Account menu | ![Account utilities](Images/account-utilities.png) Account utilities |
| ![Utilities](Images/utilities.png) Utilities: servers, games, favorites, universes, outfits, watcher | ![Theme editor](Images/theme.png) Theme editor |
| ![Colors](Images/colors.png) Colors for accounts and groups | ![CPU limit](Images/cpu-limit.png) CPU limit per account |

![Settings](Images/settings.png)

The screenshots use made-up demo accounts and are taken by the interface tests (see [Tests](#tests)).

## Building from source

Requirements: Windows 10 or 11, [Rust](https://rustup.rs) and [Node.js](https://nodejs.org) 20 or later.

```powershell
npm --prefix ui install
npm --prefix ui run build
cargo build --release
```

`RobloxAccountManager.exe` and `ram-cli.exe` end up in `target\release`. While working on the interface,
`npm --prefix ui run dev` serves it with hot reload at localhost:1420, and a debug build (`cargo build`) uses it.

### Layout

```
src/
  app.rs        state shared by everything
  core/         files: settings, accounts, encryption, languages (same formats as version 3)
  roblox/       the Roblox web APIs
  platform/     Windows: processes, windows, mutexes, ping, log file
  services/     launching Roblox, browser, local web API, watcher, Anti-AFK, updater
  commands/     what the interface can ask for, one file per part of the interface
  cli/          ram-cli: options, commands, the pipe to the open app
ui/src/
  components/   one folder per part of the interface (shadcn/ui underneath)
  hooks/ lib/   actions, store, translations, theme
lang/           translations
tests/          interface, command line and updater tests, fixtures
tools/          strings.py (translation check), release.ps1
examples/       sign-release.rs, signs a release for the updater
```

### Tests

```powershell
cargo test --lib                    # file formats, encryption (including a file encrypted by version 3), settings, dates
node tests/cli.mjs [--release]      # the command line, on its own and through an open app
node tests/ui.mjs  [--release]      # the interface, clicking through every window over WebView2's DevTools protocol
node tests/update.mjs               # the updater: a signed update installs, a changed or relabelled one is refused
python tools/strings.py             # every text translated
```

The tests run copies of the app with demo accounts. They never launch Roblox, close it or press keys, and they
run next to your own Roblox Account Manager without touching it or taking the focus, so your accounts and a game you have open are left alone.

### Releasing

```powershell
cargo run --example sign-release -- new-key       # once: makes the signing key (keep a backup of it)
pwsh -File tools\release.ps1                      # builds and runs every test (the app can stay open, ~3 minutes)
git push fork modernized:master
pwsh -File tools\publish.ps1 -Notes notes.md      # GitHub builds it, then it's signed and published
```

`publish.ps1` tags the commit, and [the release workflow](.github/workflows/release.yml) builds the exes and the zip on
GitHub from that commit, attests where they were built, and leaves a draft (the updater doesn't see drafts). The
script then checks the attestations, signs the zip for the updater with the private key, which stays in
`%USERPROFILE%\.roblox-account-manager\` and never goes to GitHub, uploads the `.sig` and publishes the release with
the notes, how to verify the files and their SHA-256. Translations of the notes, saved next to it as `notes.pt-PT.md`,
`notes.es.md`, `notes.fr.md` and `notes.de.md`, are attached to the release, and the app shows the one in its
language. Installed apps only accept updates signed with that key.

Every push is also built and unit tested on GitHub ([CI](.github/workflows/ci.yml)), without any secret.

### Updating the screenshots

After changing the interface, run the following and commit the `Images` folder. The README picks up the new images.

```powershell
node tests/ui.mjs --readme
```

# Warning
If someone asks you to generate an "rbx-player link", **DO NOT** do it. They can use it to join any game with your account, or even launch Roblox Studio with one of your games. They can do many things in game, such as spend your Robux or do things that get your account terminated. **USE THESE FEATURES AT YOUR OWN RISK**

# Extra Features
Extra features can be enabled in `Settings` > `Developer` > `Enable Developer Mode`.
Beware of the risks you are taking if you accidentally send something to someone.

If you ever want a friend to join a game using your account, make sure the Place ID and Job ID are entered correctly, then right click the account and click "Copy rbx-player Link". DO NOT do this if someone asks you for it.

# Developer API
Enable it in `Settings` > `Developer` > `Enable Web Server`. It answers the same requests as the original app, see the [documentation](https://ic3w0lf22.gitbook.io/roblox-account-manager/), and [`RAMAccount.lua`](RAMAccount.lua) is a ready-made client for it.
Requests from web pages in a browser are refused, and so is any host name other than `localhost`/`127.0.0.1` (unless
you allow other devices). Every method that changes an account, launches it, imports a cookie or reveals a cookie or
CSRF token needs the password: set one (6 characters or more) and send it as `Password=` (`RAMAccount.lua` does it
with `SetPassword`). Be careful running random scripts with dangerous methods enabled.

# Frequently Asked Questions
## **Q:** Why is this program detected as a virus?

**A:** Tools that launch games and manage logins often get flagged because real malware uses similar techniques. The whole source is here: you can [build it yourself](#building-from-source) and compare.


## **Q:** How do I enable multi-roblox?

**A:** Open the settings with the gear icon in the top right and, in the `General` tab, turn on `Multi Roblox`. It works as soon as every Roblox window is closed, no restart needed.


## **Q:** Is multi-roblox safe?

**A:** It's on by default. A Byfron developer has stated that using multiple clients may be considered malicious behavior, so use it at your own risk, and turn it off in `Settings` > `General` > `Multi Roblox` if you'd rather not.
![2023-05-06 23_58_33-Clipboard](https://user-images.githubusercontent.com/11778654/236662271-ce6bc2c8-7690-436a-97d0-1cfea56b541f.png)


## **Q:** Will Anti-AFK get my accounts banned?

**A:** It only presses keys in the Roblox windows, like you would. Some games have their own rules against being AFK, so check the rules of the game you play.


## **Q:** Why do my accounts have yellow/red dots on them?

**A:** The dot means the account hasn't been used in over 20 days, and it turns redder as the days go up. Joining a game with the account removes it. You can also turn the alert off in `Settings` > `General` > `Disable Aging Alert`, or turn on `Auto Refresh Cookies` so unused accounts don't get logged out.


## **Q:** How do I back up my accounts file?

**A:** `Settings` > `General` > `Backup` > `Export` saves your accounts encrypted with a password you choose, to keep on Google Drive, a flash drive or another PC. `Import` adds the accounts of a backup that aren't there yet. The backup never holds your accounts in plain text, and it can even be used as an `AccountData.json` with that password.


## **Q:** How do I prevent Windows Defender from deleting the app's files?

**A:** Add an exclusion for the Roblox Account Manager folder. Here's a video on how to add one: https://youtu.be/1r93NtwZt4o


## **Q:** Can I join VIP servers with it?

**A:** Yes. Paste the private server link into the Job ID box and press Join Server. Both kinds work:

- the new share links, `https://www.roblox.com/share?code=…&type=Server`: the game comes from the link, the Place ID box doesn't matter;
- the old links, `https://www.roblox.com/games/<place>/…?privateServerLinkCode=…`.

The account needs *Who can invite me to private servers* set to Everyone in its Roblox privacy settings; the app offers to change it. Private servers the account already has access to show up with a VIP tag in Utilities → Servers (select the account first).


## **Q:** Can you use this on Mac?

**A:** No, it only runs on Windows.


## **Q:** I’ve encountered a bug/issue

**A:** Open an [issue](https://github.com/Gugas19/Roblox-Account-Manager/issues) with screenshots (if possible) and a description of what happened. The app writes a `log.txt` next to it that helps too (check it doesn't contain anything private before sharing it).


## **Q:** I can’t launch multiple accounts repeatedly.

**A:** That's Roblox's rate limiting. Raise `Launch Delay` in the settings.


## **Q:** Can you get banned for using this?

**A:** It doesn't break the Roblox Terms of Service, but some games don't allow alt accounts, so check the rules of the game you play if you're unsure.

## **Q:** My AccountData file gets corrupted often
**A:** Use password protection instead of the default encryption, it doesn't depend on Windows. As a last resort you can turn encryption off by creating a file called `NoEncryption.IUnderstandTheRisks.iautamor` next to the app. Do this at your own risk: anyone who gets that file gets your accounts.

# Features
| Feature | Description | How to |
| :--- | :---: | ---: |
| Account Encryption | Your account data is encrypted for your Windows user, so someone who gets the file can't read it | **DO NOT SHARE YOUR `AccountData.json` FILE AT ALL** |
| Password Encryption | Encrypt your data with a password | Recommended: the file can be stored in Google Drive or moved to another computer |
| [Multi Roblox](#q-is-multi-roblox-safe) | [ON BY DEFAULT, READ FAQ] Several Roblox clients open at once | **If it doesn't work, make sure no Roblox process is running in the background (Task Manager)** |
| Anti-AFK | Keeps Roblox windows from being kicked for being idle, rejoins disconnected accounts, mutes and arranges windows | Click `Anti-AFK` in the top bar, or `ram-cli antiafk start` |
| Command Line | Join, follow, list, close and import accounts from scripts and shortcuts | `ram-cli help` |
| Command Palette | Any account or action from the keyboard | `Ctrl+K` |
| Languages | English, Português (Portugal), Español, Français, Deutsch | Click the flag button in the top right |
| Account Search | Find an account by username, alias, description or group | Type in the search box above the list, or press `Ctrl+F` |
| Undo | Bring back accounts you just removed | Click `Undo` on the message that shows after removing |
| Server List | See a game's servers with their player count and ping | Click `Utilities` in the main window |
| Load Region | See where a server is and get an accurate ping | Right click a server in the server list, then click `Load Region` **(requires a valid account to be selected)** |
| [Join Small Servers](https://youtu.be/Red66cV6vVI) | Join small servers in games that use a lobby place to teleport you | Put the real game's Place ID next to `Refresh` in the server list, refresh, then right click a server and click `Join Game` |
| Player Finder | Find a player even with their follows off, as long as you know their game | In the server list, type the username and click search |
| Account Utilities | Change an account's password, email, display name, follow privacy, add friends, block, sign out other sessions | Click `Account Utilities` **(requires a valid account to be selected)** |
| Account Sorting | Order accounts the way you like | Drag and drop an account in the list, or `Sort Alphabetically` |
| Account Grouping | Put accounts in groups, drag them between groups, rename a group | Right click an account, `Groups`, `Move Account To`; right click a group's header to rename it |
| Group Sorting | Order groups by giving them a number | Name a group with a number from 0-999 in front (ex. `1Main`, `007 Bank`), the number is hidden afterwards |
| Games List | Search Roblox games | `Utilities` > `Games` |
| Favorite Games | Keep the games you play close | `Utilities` > `Favorites` |
| Recent Games | The games you joined last | The clock icon above `Place ID`, or type the game's name into `Place ID` |
| Game Links | Paste a game or private server link (old or `share?code=` links) | The Place ID and the private server are taken from the link |
| Open Browser | A browser logged in as the selected account | Click `Open Browser` with an account selected |
| Join Group | Join a group with several accounts | The arrow next to `Open Browser` > `Join Group` |
| Custom URL & Javascript | Open a page and run a script in several browsers | The arrow next to `Open Browser` |
| Shuffle Job ID | A random server every time you press Join, unless a Job ID is set | Click the shuffle icon above `Job ID` |
| Save Place ID & Job ID | Save a place and server to specific accounts, they join it every time | Enter them, then click the save icon next to `Job ID` |
| Universe Viewer | Every place of a game | `Utilities` > `Universe` |
| Outfit Viewer | See other players' outfits and wear them, buy what's missing | `Utilities` > `Outfits` |
| Roblox Watcher | Close Roblox on the beta home menu, without connection, with too little memory or the wrong window title | `Utilities` > `Watcher` |
| Prevent Duplicate Instances | Closes the account's previous Roblox when you launch it again | Each account gets a `BrowserTrackerID` so the app knows which Roblox is whose |
| Save Window Positions | Each account's Roblox opens where it was last time | `Utilities` > `Watcher` |
| Save Passwords | Off by default. When on, the password you type when logging in is saved, to copy it later or log in again when the cookie expires. Turning it off offers to forget the saved ones | Turn it on in `Settings` > `General`, then right click the account > `Copy Password` |
| Themes | Dark, Midnight, Forest, Light, and your own colors | The palette button in the top right |
| Developer Mode | Extra account menu items: fields, authentication tickets, cookies and launch links | `Settings` > `Developer` > `Enable Developer Mode` |
| Local Web API | Use the app from scripts with plain http requests | [Documentation](https://ic3w0lf22.gitbook.io/roblox-account-manager/), [`RAMAccount.lua`](RAMAccount.lua) |
| Import Cookies | Add accounts from their .ROBLOSECURITY cookies | Drop cookies onto the app, the arrow next to `Add Account` > `Cookie(s)`, or `ram-cli import` |
| Bulk User Importing | Add accounts from username:password lines | The arrow next to `Add Account` > `User:Pass` |
| Custom ClientAppSettings | Use your own ClientAppSettings.json (Roblox only applies the flags on its allowlist; raise the FPS cap in Roblox's own settings, *Maximum Frame Rate*) | `Settings` > `Miscellaneous` |
| CPU Limit per Account | Each account's Roblox window can be held to a share of the PC's CPU (5% for an alt that just stays online, none for your main), by Windows itself, right away | Right click an account > `CPU limit`, or `ram-cli cpu` |
| Colors | Give accounts and groups a color | Right click an account > `Color`, or a group's header > `Group color` |
| Backups | Export your accounts encrypted with a password, import them on any PC | `Settings` > `General` > `Backup` |
| Proxies | Browsers use a random proxy from `proxies.txt` | `Settings` > `Miscellaneous` > `Use proxies` |
| Account Presence | A dot when an account is online, in game or in Studio | `Settings` > `Miscellaneous` |
| Automatic Cookie Refresh | Unused accounts get new cookies so they don't get logged out | `Settings` > `General` > `Auto Refresh Cookies` (signs out other sessions) |
| Quick Log In | Log in to an account on another device with Roblox's Quick Log In | Right click an account > `Quick Log In` |
| Tray | Keep the app, Anti-AFK and the web API running with the window closed | Anti-AFK settings |
| Updates | Tells you about a new version, it downloads and installs when you click, only when signed by the developer (or `Automatic`, or `Off`) | `Settings` > `General` |

# Credits

- [ic3w0lf22](https://github.com/ic3w0lf22/Roblox-Account-Manager), the original Roblox Account Manager, and everyone who contributed to it.
- [Campionnn](https://github.com/Campionnn/Roblox-Account-Manager), who kept it updated.
- [Agzes/AntiAFK-RBX](https://github.com/Agzes/AntiAFK-RBX), whose features inspired the Anti-AFK (written from scratch here).
- [Tauri](https://tauri.app), [shadcn/ui](https://ui.shadcn.com), [Lucide](https://lucide.dev) icons.
- Flags: [flagcdn.com](https://flagcdn.com) (public domain).

# License

[GNU General Public License v3.0](LICENSE), like the original project.
