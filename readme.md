# Syntax Boostrapper
A bootstrapper written in Rust meant to replace the old Roblox Launcher

You are welcome to add support to other platforms ( *MacOS* ) or add improvements to the bootstrapper

## Building
For Windows:
> win-build-release.bat

From a Mac (64-bit exe, needs `brew install mingw-w64`):
> rustup target add x86_64-pc-windows-gnu && cargo build --release --target x86_64-pc-windows-gnu

On Windows and macOS the bootstrapper shows an installer window (logo, status, progress bar, Cancel) instead of a console.

For Linux:
> cargo build --release

For macOS:
> ./mac-build-release.sh

This builds `target/macos/Subter Launcher.app` (Apple Silicon + Intel) and `target/macos/SubterMacLauncher.zip`.
macOS only delivers `subter-player:` links to an app bundle, so the Mac bootstrapper is an `.app`: it receives
the link and shows an installer window while it downloads and starts the client. Run the executable with
`--console <link>` to get the console output instead.
The app is ad-hoc signed, so users have to right-click > Open it the first time.

### Mac clients


Clients are installed to`~/Library/Application Support/Subter Launcher/Versions/{version}` and player logs go to `~/Library/Logs/Subter`.

If you want to build the debug version of the bootstrapper for development you can run
> cargo build

