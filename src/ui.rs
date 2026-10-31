use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Clone, PartialEq)]
pub struct State {
    pub status: String,
    pub progress: Option<f64>,
    pub failed: bool,
}

static STATE: Mutex<State> = Mutex::new(State { status: String::new(), progress: None, failed: false });
static WINDOWED: AtomicBool = AtomicBool::new(false);
static SEGMENT: Mutex<(f64, f64)> = Mutex::new((0.0, 1.0));

pub fn set_windowed() {
    WINDOWED.store(true, Ordering::SeqCst);
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);
        let message = info.payload().downcast_ref::<&str>().map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown error".into());
        show_error(&format!("Something went wrong: {}", message));
    }));
}

pub fn snapshot() -> State {
    STATE.lock().unwrap().clone()
}

pub fn status(status: &str) {
    let mut state = STATE.lock().unwrap();
    if !state.failed {
        state.status = status.to_string();
        state.progress = None;
    }
}

pub fn progress(fraction: f64) {
    let mut state = STATE.lock().unwrap();
    if !state.failed {
        state.progress = Some(fraction.clamp(0.0, 1.0));
    }
}

pub fn segment(index: usize, count: usize) {
    *SEGMENT.lock().unwrap() = (index as f64 / count as f64, 1.0 / count as f64);
}

pub fn download_progress(downloaded: u64, total: u64) {
    let (start, length) = *SEGMENT.lock().unwrap();
    let fraction = if total == 0 { 1.0 } else { downloaded as f64 / total as f64 };
    progress(start + fraction * length);
}

fn show_error(message: &str) {
    let mut state = STATE.lock().unwrap();
    state.status = message.to_string();
    state.progress = Some(0.0);
    state.failed = true;
}

pub fn fail(message: &str) -> ! {
    super::error(message);
    if !WINDOWED.load(Ordering::SeqCst) {
        std::process::exit(1);
    }
    show_error(message);
    loop {
        std::thread::park();
    }
}
