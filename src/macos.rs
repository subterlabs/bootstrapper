use super::{create_folder_if_not_exists, debug, download_file, http_get, info, print_startup_text, ui};
use colored::*;
use reqwest::Client;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BASE_URL: &str = "www.subter.org";
const SETUP_URL: &str = "setup.subter.org";
const URL_SCHEME: &str = "subter-player";

pub async fn main(args: Vec<String>) {
    if args.get(1).map(String::as_str) == Some("--console") {
        run(args.get(2).cloned().unwrap_or_default()).await;
    } else if let Some(url) = args.get(1).filter(|a| a.starts_with(URL_SCHEME)) {
        app::run(Some(url.clone()));
    } else {
        app::run(None);
    }
}

// The installer window: logo, status text, green progress bar and a Cancel button.
// The install runs on a worker thread and the window polls ui.rs on a timer.
mod app {
    use super::super::ui;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, ProtocolObject};
    use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSApplication, NSApplicationActivationPolicy,
        NSApplicationDelegate, NSBackingStoreType, NSButton, NSColor, NSFont, NSImage, NSImageScaling, NSImageView,
        NSProgressIndicator, NSProgressIndicatorStyle, NSTextAlignment, NSTextField, NSWindow, NSWindowDelegate, NSWindowStyleMask,
    };
    use objc2_foundation::{NSArray, NSData, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSTimer, NSURL};
    use std::cell::{OnceCell, RefCell};

    const URL_WAIT_SECONDS: f64 = 1.5;
    const WIDTH: f64 = 500.0;
    const HEIGHT: f64 = 290.0;
    const LOGO_SIZE: f64 = 100.0;
    const BAR_X: f64 = 24.0;
    const LOGO: &[u8] = include_bytes!("../assets/Bootstrapper.ico");

    struct Views {
        // Kept so the window stays alive
        _window: Retained<NSWindow>,
        label: Retained<NSTextField>,
        bar: Retained<NSProgressIndicator>,
        button: Retained<NSButton>,
    }

    #[derive(Default)]
    struct Ivars {
        url: RefCell<Option<String>>,
        views: OnceCell<Views>,
        state: RefCell<Option<ui::State>>,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "SubterLauncherDelegate"]
        #[ivars = Ivars]
        struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl NSApplicationDelegate for Delegate {
            #[unsafe(method(application:openURLs:))]
            fn application_open_urls(&self, _app: &NSApplication, urls: &NSArray<NSURL>) {
                let url = urls
                    .firstObject()
                    .and_then(|url| url.absoluteString())
                    .map(|url| url.to_string())
                    .unwrap_or_default();
                self.start(url);
            }

            #[unsafe(method(applicationDidFinishLaunching:))]
            fn did_finish_launching(&self, _notification: &NSNotification) {
                if let Some(url) = self.ivars().url.take() {
                    self.start(url);
                    return;
                }
                unsafe {
                    let _: () = msg_send![
                        self,
                        performSelector: sel!(launchedWithoutURL),
                        withObject: std::ptr::null::<NSObject>(),
                        afterDelay: URL_WAIT_SECONDS
                    ];
                }
            }
        }

        unsafe impl NSWindowDelegate for Delegate {
            #[unsafe(method(windowWillClose:))]
            fn window_will_close(&self, _notification: &NSNotification) {
                std::process::exit(0);
            }
        }

        impl Delegate {
            #[unsafe(method(launchedWithoutURL))]
            fn launched_without_url(&self) {
                if self.ivars().views.get().is_none() {
                    super::open_games_page();
                    std::process::exit(0);
                }
            }

            #[unsafe(method(cancel:))]
            fn cancel(&self, _sender: &AnyObject) {
                std::process::exit(0);
            }

            #[unsafe(method(tick:))]
            fn tick(&self, _timer: &NSTimer) {
                self.update();
            }
        }
    );

    impl Delegate {
        // Shows the window and starts installing/launching, once per process
        fn start(&self, url: String) {
            if self.ivars().views.get().is_some() {
                return;
            }
            let mtm = self.mtm();
            ui::set_windowed();
            ui::status("Connecting to Subter...");
            let _ = self.ivars().views.set(self.build_window(mtm));
            self.update();
            unsafe {
                NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(0.03, self, sel!(tick:), None, true);
            }
            std::thread::spawn(move || super::super::tokio_runtime().block_on(super::run(url)));
        }

        fn build_window(&self, mtm: MainThreadMarker) -> Views {
            let app = NSApplication::sharedApplication(mtm);
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, HEIGHT)),
                    NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable,
                    NSBackingStoreType::Buffered,
                    false,
                )
            };
            unsafe { window.setReleasedWhenClosed(false) };
            window.setTitle(&NSString::from_str("Subter"));
            // Always dark, like the Windows window
            window.setAppearance(NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }).as_deref());
            window.setDelegate(Some(ProtocolObject::from_ref(self)));
            let content = window.contentView().unwrap();

            let logo = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(LOGO)).unwrap();
            let logo_view = NSImageView::imageViewWithImage(&logo, mtm);
            logo_view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
            logo_view.setFrame(frame((WIDTH - LOGO_SIZE) / 2.0, 24.0, LOGO_SIZE, LOGO_SIZE));
            content.addSubview(&logo_view);

            let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
            label.setAlignment(NSTextAlignment::Center);
            label.setFont(Some(&NSFont::systemFontOfSize(13.0)));
            label.setTextColor(Some(&NSColor::labelColor()));
            label.setFrame(frame(16.0, 140.0, WIDTH - 32.0, 44.0));
            content.addSubview(&label);

            let bar = NSProgressIndicator::new(mtm);
            bar.setStyle(NSProgressIndicatorStyle::Bar);
            bar.setMinValue(0.0);
            bar.setMaxValue(1.0);
            bar.setFrame(frame(BAR_X, 192.0, WIDTH - BAR_X * 2.0, 20.0));
            content.addSubview(&bar);

            let button = unsafe {
                NSButton::buttonWithTitle_target_action(&NSString::from_str("Cancel"), Some(self), Some(sel!(cancel:)), mtm)
            };
            button.setFrame(frame((WIDTH - 130.0) / 2.0, 232.0, 130.0, 34.0));
            button.setKeyEquivalent(&NSString::from_str("\u{1b}"));
            content.addSubview(&button);

            window.center();
            window.makeKeyAndOrderFront(None);
            unsafe {
                let _: () = msg_send![&*app, activateIgnoringOtherApps: true];
            }
            Views { _window: window, label, bar, button }
        }

        fn update(&self) {
            let Some(views) = self.ivars().views.get() else { return };
            let state = ui::snapshot();
            let mut last = self.ivars().state.borrow_mut();
            if last.as_ref() == Some(&state) {
                return;
            }
            if last.as_ref().map(|s| &s.status) != Some(&state.status) {
                views.label.setStringValue(&NSString::from_str(&state.status));
            }
            if state.failed && !last.as_ref().is_some_and(|s| s.failed) {
                views.label.setTextColor(Some(&NSColor::systemRedColor()));
                views.button.setTitle(&NSString::from_str("Close"));
            }

            views.bar.setHidden(state.failed);
            match state.progress {
                Some(fraction) => {
                    unsafe { views.bar.stopAnimation(None) };
                    views.bar.setIndeterminate(false);
                    views.bar.setDoubleValue(fraction);
                }
                None => {
                    views.bar.setIndeterminate(true);
                    unsafe { views.bar.startAnimation(None) };
                }
            }
            *last = Some(state);
        }
    }

    // Layout is written top-down like the Windows window, AppKit counts from the bottom
    fn frame(x: f64, top: f64, width: f64, height: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, HEIGHT - top - height), NSSize::new(width, height))
    }

    pub fn run(url: Option<String>) {
        let mtm = MainThreadMarker::new().expect("must run on the main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
        let delegate = Delegate::alloc(mtm).set_ivars(Ivars { url: RefCell::new(url), ..Default::default() });
        let delegate: Retained<Delegate> = unsafe { msg_send![super(delegate), init] };
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        app.run();
    }

}

fn open_games_page() {
    let _ = Command::new("open").arg(format!("https://{}/games", BASE_URL)).status();
}

fn get_installation_directory() -> PathBuf {
    dirs::data_local_dir().unwrap().join("Subter Launcher")
}

struct LaunchInfo {
    launch_mode: String,
    authentication_ticket: String,
    join_script: String,
    client_year: String,
}

fn parse_launch_url(url: &str) -> LaunchInfo {
    let mut info = LaunchInfo {
        launch_mode: String::new(),
        authentication_ticket: String::new(),
        join_script: String::new(),
        client_year: String::new(),
    };
    let body = url
        .trim_start_matches(&format!("{}:", URL_SCHEME))
        .trim_start_matches('/');
    for part in body.split('+') {
        let (key, value) = part.split_once(':').unwrap_or((part, ""));
        match key {
            "launchmode" => info.launch_mode = value.to_string(),
            "gameinfo" => info.authentication_ticket = value.to_string(),
            "placelauncherurl" => info.join_script = value.to_string(),
            "clientyear" => info.client_year = value.to_string(),
            _ => {}
        }
    }
    info
}

enum MacClient {
    Native { zip_name: &'static str, folder: &'static str },
    Wine { zip_name: &'static str, folder: &'static str, exe: &'static str },
}

fn client_for_year(year: &str) -> Option<MacClient> {
    match year {
        "2021" => Some(MacClient::Native { zip_name: "2021macclient.zip", folder: "Client2021" }),
        "2018" => Some(MacClient::Native { zip_name: "2018macclient.zip", folder: "Client2018" }),
        "2016" => Some(MacClient::Wine { zip_name: "2016macclient.zip", folder: "Client2016", exe: "SubterPlayerBeta.exe" }),
        _ => None,
    }
}

async fn run(url: String) {
    print_startup_text(BASE_URL);

    if url.is_empty() {
        info("Opening the games page. Click Play on a game to launch it.");
        open_games_page();
        return;
    }

    let launch = parse_launch_url(&url);
    if launch.launch_mode != "play" {
        ui::fail("Unknown launch mode, exiting.");
    }
    let client = match client_for_year(&launch.client_year) {
        Some(client) => client,
        None => ui::fail(&format!(
            "{} games are not available on Mac yet. Supported years: 2016, 2018, 2021.",
            if launch.client_year.is_empty() { "This year's" } else { &launch.client_year }
        )),
    };

    let http_client: Client = reqwest::Client::builder().no_gzip().build().unwrap();
    let latest_client_version = match http_get(&http_client, &format!("https://{}/version", SETUP_URL)).await {
        Ok(version) => version.trim().to_string(),
        Err(e) => ui::fail(&format!(
            "Failed to fetch the latest client version: [{}], are you connected to the internet?",
            e.to_string().bright_red()
        )),
    };
    info(&format!("Latest Client Version: {}", latest_client_version.cyan().underline()));

    let installation_directory = get_installation_directory();
    let versions_directory = installation_directory.join("Versions");
    let current_version_directory = versions_directory.join(&latest_client_version);
    let downloads_directory = installation_directory.join("Downloads");
    create_folder_if_not_exists(&current_version_directory).await;
    create_folder_if_not_exists(&downloads_directory).await;

    let (zip_name, folder) = match &client {
        MacClient::Native { zip_name, folder } => (*zip_name, *folder),
        MacClient::Wine { zip_name, folder, .. } => (*zip_name, *folder),
    };
    let client_directory = current_version_directory.join(folder);
    install_client_if_needed(&http_client, &latest_client_version, zip_name, &client_directory, &downloads_directory).await;
    remove_old_versions(&versions_directory, &current_version_directory);

    let log_directory = dirs::home_dir().unwrap().join("Library/Logs/Subter");
    std::fs::create_dir_all(&log_directory).unwrap();

    match client {
        MacClient::Native { .. } => launch_native(&client_directory, &launch, &log_directory),
        MacClient::Wine { exe, .. } => launch_wine(&installation_directory, &client_directory.join(exe), &launch, &log_directory),
    }
    // Leave "Starting Subter..." up while the client opens its window
    std::thread::sleep(std::time::Duration::from_secs(3));
    std::process::exit(0);
}

fn detach_from_terminal(command: &mut Command) -> &mut Command {
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        })
    }
}


async fn install_client_if_needed(http_client: &Client, version: &str, zip_name: &str, client_directory: &Path, downloads_directory: &Path) {
    let installed_marker = client_directory.join(".installed");
    if installed_marker.exists() {
        debug(&format!("{} is already installed", zip_name));
        return;
    }
    if client_directory.exists() {
        std::fs::remove_dir_all(client_directory).unwrap();
    }

    info("Downloading the client, this may take a while.");
    ui::status("Downloading Subter...");
    ui::progress(0.0);
    let zip_path = downloads_directory.join(zip_name);
    let zip_url = format!("https://{}/{}-{}", SETUP_URL, version, zip_name);
    match http_client.head(&zip_url).send().await {
        Ok(response) if response.status().is_success() => {}
        Ok(response) => ui::fail(&format!("The Mac client is not available on the setup server ({} returned {}).", zip_url, response.status())),
        Err(e) => ui::fail(&format!("Failed to reach the setup server: {}", e)),
    }
    download_file(http_client, &zip_url, &zip_path.to_path_buf()).await;

    info(&format!("Extracting {}", zip_name.bright_blue()));
    ui::status("Installing Subter...");
    std::fs::create_dir_all(client_directory).unwrap();
    let status = Command::new("ditto").arg("-x").arg("-k").arg(&zip_path).arg(client_directory).status().unwrap();
    let _ = std::fs::remove_file(&zip_path);
    if !status.success() {
        let _ = std::fs::remove_dir_all(client_directory);
        ui::fail("Failed to extract the client, please try again.");
    }
    std::fs::write(installed_marker, version).unwrap();
    info("Client installed.");
}

fn remove_old_versions(versions_directory: &Path, current_version_directory: &Path) {
    for entry in std::fs::read_dir(versions_directory).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() && path != current_version_directory {
            debug(&format!("Removing old version {}", path.display()));
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn launch_native(client_directory: &Path, launch: &LaunchInfo, log_directory: &Path) {
    let player = client_directory.join("RobloxPlayer.app/Contents/MacOS/RobloxPlayer");
    if !player.exists() {
        let _ = std::fs::remove_file(client_directory.join(".installed"));
        ui::fail("The client is missing RobloxPlayer, it will be downloaded again on next launch.");
    }
    info("Launching SUBTER");
    ui::status("Starting Subter...");
    ui::progress(1.0);
    let log = std::fs::File::create(log_directory.join("player.log")).unwrap();
    detach_from_terminal(&mut Command::new(&player))
        .args([
            "-ticket", &launch.authentication_ticket,
            "-authURL", &format!("https://{}/Login/Negotiate.ashx", BASE_URL),
            "-scriptURL", &launch.join_script,
            "-browserTrackerId", "0",
            "-rloc", "en_us",
            "-gloc", "en_us",
        ])
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .unwrap_or_else(|e| ui::fail(&format!("Failed to start the client: {}", e)));
}

fn find_wine(installation_directory: &Path) -> Option<(PathBuf, Vec<(String, String)>, Vec<String>)> {
    let wine_path_file = installation_directory.join("winepath.txt");
    if let Ok(custom) = std::fs::read_to_string(&wine_path_file) {
        let custom = PathBuf::from(custom.trim());
        info(&format!("Using custom wine binary: {}", custom.display().to_string().bright_blue()));
        return Some((custom, vec![], vec![]));
    }

    let home = dirs::home_dir().unwrap();
    let whisky_wine = home.join("Library/Application Support/com.isaacmarovitz.Whisky/Libraries/Wine/bin/wine64");
    if whisky_wine.exists() {
        if let Some(bottle) = find_whisky_bottle(&home) {
            info(&format!("Using Whisky bottle {}", bottle.display().to_string().bright_blue()));
            return Some((whisky_wine, vec![("WINEPREFIX".into(), bottle.to_str().unwrap().into())], vec![]));
        }
    }

    let crossover_wine = PathBuf::from("/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine");
    if crossover_wine.exists() {
        let bottles = home.join("Library/Application Support/CrossOver/Bottles");
        let bottle = std::fs::read_dir(&bottles).ok().and_then(|entries| {
            let names: Vec<String> = entries.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            names.iter().find(|n| n.as_str() == "Subter").or(names.first()).cloned()
        });
        if let Some(bottle) = bottle {
            info(&format!("Using CrossOver bottle {}", bottle.bright_blue()));
            return Some((crossover_wine, vec![], vec!["--bottle".into(), bottle]));
        }
    }

    if Command::new("which").arg("wine").stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) {
        return Some((PathBuf::from("wine"), vec![], vec![]));
    }
    None
}

fn find_whisky_bottle(home: &Path) -> Option<PathBuf> {
    let container = home.join("Library/Containers/com.isaacmarovitz.Whisky");
    let mut bottles: Vec<PathBuf> = Vec::new();

    let list = container.join("BottleVM.plist");
    for i in 0.. {
        let output = Command::new("plutil").args(["-extract", &format!("paths.{}.relative", i), "raw"]).arg(&list).output();
        match output {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout).trim().trim_start_matches("file://").replace("%20", " ");
                bottles.push(PathBuf::from(path.trim_end_matches('/')));
            }
            _ => break,
        }
    }
    if let Ok(entries) = std::fs::read_dir(container.join("Bottles")) {
        bottles.extend(entries.flatten().map(|e| e.path()));
    }
    bottles.retain(|b| b.join("drive_c").is_dir());

    let named = bottles.iter().find(|b| {
        Command::new("plutil").args(["-extract", "info.name", "raw"]).arg(b.join("Metadata.plist")).output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "Subter").unwrap_or(false)
    });
    named.cloned().or_else(|| {
        bottles.into_iter().max_by_key(|b| {
            std::fs::metadata(b.join("user.reg")).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH)
        })
    })
}

fn launch_wine(installation_directory: &Path, exe: &Path, launch: &LaunchInfo, log_directory: &Path) {
    if !exe.exists() {
        let _ = std::fs::remove_file(exe.parent().unwrap().join(".installed"));
        ui::fail("The client is missing SubterPlayerBeta.exe, it will be downloaded again on next launch.");
    }
    let (wine, env, wine_args) = match find_wine(installation_directory) {
        Some(wine) => wine,
        None => ui::fail(&format!(
            "2016 games need Wine on Mac. Install Whisky (https://getwhisky.app) or CrossOver and create a bottle, or put the path to a wine binary in {}",
            installation_directory.join("winepath.txt").display()
        )),
    };

    info("Launching SUBTER through Wine");
    ui::status("Starting Subter...");
    ui::progress(1.0);
    let log = std::fs::File::create(log_directory.join("player-wine.log")).unwrap();
    detach_from_terminal(&mut Command::new(&wine))
        .envs(env)
        .env("WINEDEBUG", std::env::var("WINEDEBUG").unwrap_or_else(|_| "-all".into()))
        .args(wine_args)
        .arg(exe)
        .args([
            "--play",
            "-a", &format!("https://{}/Login/Negotiate.ashx", BASE_URL),
            "-t", &launch.authentication_ticket,
            "-j", &launch.join_script,
            "-b", "0",
        ])
        .current_dir(exe.parent().unwrap())
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .unwrap_or_else(|e| ui::fail(&format!("Failed to start Wine ({}): {}", wine.display(), e)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_website_launch_url() {
        let launch = parse_launch_url("subter-player:1+launchmode:play+gameinfo:ABC+placelauncherurl:https://www.subter.org/Game/placelauncher.ashx?placeId=1&t=XYZ+k:l+clientyear:2021");
        assert_eq!(launch.launch_mode, "play");
        assert_eq!(launch.authentication_ticket, "ABC");
        assert_eq!(launch.join_script, "https://www.subter.org/Game/placelauncher.ashx?placeId=1&t=XYZ");
        assert_eq!(launch.client_year, "2021");
    }

    #[test]
    fn parses_firefox_extra_slash() {
        let launch = parse_launch_url("subter-player:/1+launchmode:play+gameinfo:ABC+placelauncherurl:https://x/y+clientyear:2016");
        assert_eq!(launch.launch_mode, "play");
        assert_eq!(launch.client_year, "2016");
    }

    #[test]
    fn unsupported_years_have_no_client() {
        assert!(client_for_year("2014").is_none());
        assert!(client_for_year("2020").is_none());
        assert!(client_for_year("2021").is_some());
    }
}
