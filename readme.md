# Syntax Boostrapper
A bootstrapper written in Rust meant to replace the old Roblox Launcher

You are welcome to add support to other platforms ( *MacOS* ) or add improvements to the bootstrapper

## Building
For Windows:
> win-build-release.bat

For Linux:
> cargo build --release

For macOS:
> ./mac-build-release.sh

This builds `target/macos/Subter Launcher.app` (Apple Silicon + Intel) and `target/macos/SubterMacLauncher.zip`.
macOS only delivers `subter-player:` links to an app bundle, so the Mac bootstrapper is an `.app`: it receives
the link, then opens Terminal to show the usual console output while it downloads and starts the client.
The app is ad-hoc signed, so users have to right-click > Open it the first time.

### Mac clients
The Mac bootstrapper downloads only the client a game needs, from the setup server:

| Year | File on the setup server | Runs as |
|------|--------------------------|---------|
| 2021 | `{version}-2021macclient.zip` (`RobloxPlayer.app`) | Native |
| 2018 | `{version}-2018macclient.zip` (`RobloxPlayer.app`) | Native |
| 2016 | `{version}-2016macclient.zip` (Windows client) | Wine: `winepath.txt`, Whisky, CrossOver, or `wine` on PATH |

2014 and 2020 games show a "not available on Mac yet" message. Clients are installed to
`~/Library/Application Support/Subter Launcher/Versions/{version}` and player logs go to `~/Library/Logs/Subter`.

If you want to build the debug version of the bootstrapper for development you can run
> cargo build

