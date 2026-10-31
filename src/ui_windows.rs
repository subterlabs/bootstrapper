use super::ui;
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_SELECTED};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WIDTH: i32 = 500;
const HEIGHT: i32 = 290;
const LOGO_SIZE: i32 = 100;
const CANCEL_ID: usize = 1;
const TIMER_ID: usize = 1;
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
const BACKGROUND: u32 = rgb(32, 32, 32);
const BAR: u32 = rgb(0, 120, 215);
const TRACK: u32 = rgb(60, 60, 60);
const TRACK_BORDER: u32 = rgb(85, 85, 85);
const TEXT: u32 = rgb(240, 240, 240);
const ERROR_TEXT: u32 = rgb(255, 110, 110);
const BUTTON: u32 = rgb(55, 55, 55);
const BUTTON_PRESSED: u32 = rgb(75, 75, 75);
const BUTTON_BORDER: u32 = rgb(100, 100, 100);

struct Window {
    dpi: i32,
    font: HFONT,
    logo: HICON,
    button: HWND,
    state: ui::State,
    tick: u32,
}

thread_local! {
    static WINDOW: RefCell<Option<Window>> = RefCell::new(None);
}

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn scale(dpi: i32, value: i32) -> i32 {
    value * dpi / 96
}

pub fn run() {
    unsafe {
        SetProcessDPIAware();
        let instance = GetModuleHandleW(null());
        let screen = GetDC(null_mut());
        let dpi = GetDeviceCaps(screen, LOGPIXELSX as _);
        ReleaseDC(null_mut(), screen);
        let s = |v| scale(dpi, v);

        let app_icon = LoadIconW(instance, 1 as _);
        let class_name = wide("SubterBootstrapper");
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: app_icon,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: CreateSolidBrush(BACKGROUND),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: app_icon,
        };
        RegisterClassExW(&class);

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut rect = RECT { left: 0, top: 0, right: s(WIDTH), bottom: s(HEIGHT) };
        AdjustWindowRect(&mut rect, style, 0);
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        let x = (GetSystemMetrics(SM_CXSCREEN) - width) / 2;
        let y = (GetSystemMetrics(SM_CYSCREEN) - height) / 2;
        let title = wide("Subter");
        let hwnd = CreateWindowExW(0, class_name.as_ptr(), title.as_ptr(), style, x, y, width, height, null_mut(), null_mut(), instance, null());

        let dark: i32 = 1;
        DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const i32 as _, 4);

        let button_class = wide("BUTTON");
        let cancel = wide("Cancel");
        let button = CreateWindowExW(
            0, button_class.as_ptr(), cancel.as_ptr(), WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW as u32,
            s(WIDTH / 2 - 65), s(232), s(130), s(34), hwnd, CANCEL_ID as _, instance, null(),
        );
        let face = wide("Segoe UI");
        let font = CreateFontW(
            -s(15), 0, 0, 0, FW_NORMAL as _, 0, 0, 0, DEFAULT_CHARSET as _, OUT_DEFAULT_PRECIS as _,
            CLIP_DEFAULT_PRECIS as _, CLEARTYPE_QUALITY as _, (DEFAULT_PITCH | FF_SWISS) as _, face.as_ptr(),
        );
        SendMessageW(button, WM_SETFONT, font as WPARAM, 1);
        let logo = LoadImageW(instance, 1 as _, IMAGE_ICON, s(LOGO_SIZE), s(LOGO_SIZE), LR_DEFAULTCOLOR) as HICON;

        WINDOW.with(|w| *w.borrow_mut() = Some(Window { dpi, font, logo, button, state: ui::snapshot(), tick: 0 }));
        SetTimer(hwnd, TIMER_ID, 30, None);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);

        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(hwnd, &message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    std::process::exit(0);
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_TIMER => {
            WINDOW.with(|w| {
                if let Some(window) = w.borrow_mut().as_mut() {
                    let state = ui::snapshot();
                    if state.failed && !window.state.failed {
                        let close = wide("Close");
                        SetWindowTextW(window.button, close.as_ptr());
                        InvalidateRect(window.button, null(), 0);
                    }
                    if state != window.state || state.progress.is_none() {
                        window.state = state;
                        window.tick = window.tick.wrapping_add(1);
                        InvalidateRect(hwnd, null(), 0);
                    }
                }
            });
            0
        }
        WM_COMMAND if wparam & 0xffff == CANCEL_ID => std::process::exit(0),
        WM_CLOSE | WM_DESTROY => std::process::exit(0),
        WM_ERASEBKGND => 1,
        WM_DRAWITEM => {
            let item = &*(lparam as *const DRAWITEMSTRUCT);
            WINDOW.with(|w| {
                if let Some(window) = w.borrow().as_ref() {
                    draw_button(item, window);
                }
            });
            1
        }
        WM_PAINT => {
            WINDOW.with(|w| {
                if let Some(window) = w.borrow().as_ref() {
                    paint(hwnd, window);
                }
            });
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe fn fill(dc: HDC, rect: &RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    FillRect(dc, rect, brush);
    DeleteObject(brush as _);
}

unsafe fn draw_button(item: &DRAWITEMSTRUCT, window: &Window) {
    let pressed = item.itemState & ODS_SELECTED != 0;
    fill(item.hDC, &item.rcItem, BUTTON_BORDER);
    let inner = RECT { left: item.rcItem.left + 1, top: item.rcItem.top + 1, right: item.rcItem.right - 1, bottom: item.rcItem.bottom - 1 };
    fill(item.hDC, &inner, if pressed { BUTTON_PRESSED } else { BUTTON });

    let mut text = [0u16; 32];
    let length = GetWindowTextW(item.hwndItem, text.as_mut_ptr(), text.len() as i32);
    let old_font = SelectObject(item.hDC, window.font as _);
    SetBkMode(item.hDC, TRANSPARENT as _);
    SetTextColor(item.hDC, TEXT);
    let mut rect = item.rcItem;
    DrawTextW(item.hDC, text.as_mut_ptr(), length, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    SelectObject(item.hDC, old_font);
}

unsafe fn paint(hwnd: HWND, window: &Window) {
    let s = |v| scale(window.dpi, v);
    let mut paint: PAINTSTRUCT = std::mem::zeroed();
    let target = BeginPaint(hwnd, &mut paint);
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);

    let dc = CreateCompatibleDC(target);
    let bitmap = CreateCompatibleBitmap(target, client.right, client.bottom);
    let old_bitmap = SelectObject(dc, bitmap as _);
    fill(dc, &client, BACKGROUND);

    if !window.logo.is_null() {
        DrawIconEx(dc, (client.right - s(LOGO_SIZE)) / 2, s(24), window.logo, s(LOGO_SIZE), s(LOGO_SIZE), 0, null_mut(), DI_NORMAL);
    }

    let old_font = SelectObject(dc, window.font as _);
    SetBkMode(dc, TRANSPARENT as _);
    SetTextColor(dc, if window.state.failed { ERROR_TEXT } else { TEXT });
    let mut text_rect = RECT { left: s(16), top: s(140), right: client.right - s(16), bottom: s(184) };
    let mut text: Vec<u16> = window.state.status.encode_utf16().collect();
    DrawTextW(dc, text.as_mut_ptr(), text.len() as i32, &mut text_rect, DT_CENTER | DT_WORDBREAK | DT_NOPREFIX | DT_END_ELLIPSIS);
    SelectObject(dc, old_font);

    let track = RECT { left: s(24), top: s(192), right: client.right - s(24), bottom: s(212) };
    fill(dc, &track, TRACK_BORDER);
    let inner = RECT { left: track.left + 1, top: track.top + 1, right: track.right - 1, bottom: track.bottom - 1 };
    fill(dc, &inner, TRACK);
    let inner_width = (inner.right - inner.left) as f64;
    let (start, end) = match window.state.progress {
        Some(fraction) => (0.0, fraction),
        None => {
            let position = (window.tick % 70) as f64 / 70.0;
            ((position * 1.3 - 0.3).max(0.0), (position * 1.3).min(1.0))
        }
    };
    if end > start {
        let bar = RECT {
            left: inner.left + (inner_width * start) as i32,
            top: inner.top,
            right: inner.left + (inner_width * end) as i32,
            bottom: inner.bottom,
        };
        fill(dc, &bar, BAR);
    }

    BitBlt(target, 0, 0, client.right, client.bottom, dc, 0, 0, SRCCOPY);
    SelectObject(dc, old_bitmap);
    DeleteObject(bitmap as _);
    DeleteDC(dc);
    EndPaint(hwnd, &paint);
}
