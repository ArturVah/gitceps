use tao::event_loop::{EventLoop, EventLoopWindowTarget};
use tao::platform::windows::{MonitorHandleExtWindows, WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Window, WindowBuilder};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO};
use wry::{WebView, WebViewBuilder};

use super::{Area, Screen};

/// Nothing to do: a window here comes forward only when it asks to.
pub fn stay_in_background<T>(_: &mut EventLoop<T>) {}

pub fn can_place_windows() -> bool {
    true
}

pub fn mouse<T>(target: &EventLoopWindowTarget<T>) -> Option<(f64, f64)> {
    let mouse = target.cursor_position().ok()?;
    Some((mouse.x, mouse.y))
}

/// The screen a point is on, without the taskbar.
pub fn screen_at<T>(target: &EventLoopWindowTarget<T>, x: f64, y: f64) -> Option<Screen> {
    let monitor = target.monitor_from_point(x, y)?;
    let empty = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        rcMonitor: empty,
        rcWork: empty,
        dwFlags: 0,
    };
    // SAFETY: the handle is the one Windows gave tao for this monitor, and `info` is a MONITORINFO whose
    // cbSize says so, which is all GetMonitorInfoW asks for.
    #[allow(unsafe_code)]
    let found = unsafe { GetMonitorInfoW(monitor.hmonitor() as HMONITOR, &mut info) };
    if found == 0 {
        return None;
    }
    let free = info.rcWork;
    let usable = Area {
        x: f64::from(free.left),
        y: f64::from(free.top),
        width: f64::from(free.right - free.left),
        height: f64::from(free.bottom - free.top),
    };
    Some(Screen {
        usable,
        scale: monitor.scale_factor(),
    })
}

/// Windows gives a window without a frame a shadow and a thin border unless it's told not to.
pub fn without_shadow(window: WindowBuilder) -> WindowBuilder {
    window.with_undecorated_shadow(false)
}

/// Kept out of the taskbar and Alt+Tab.
pub fn unlisted(window: WindowBuilder) -> WindowBuilder {
    window.with_skip_taskbar(true)
}

pub fn build(page: WebViewBuilder, window: &Window) -> wry::Result<WebView> {
    page.build(window)
}

/// Being always on top is all Windows offers.
pub fn float_above_everything(_: &WebView) {}

/// A title bar here has one line, so the subtitle joins the title.
pub fn dress_panel(panel: &Window, _: &WebView, subtitle: &str) {
    panel.set_title(&format!("{} · {subtitle}", panel.title()));
}

/// Push Bro's window is owned by the panel, which keeps it above the panel and closes it with it.
pub fn beside(bro: WindowBuilder, panel: &Window) -> WindowBuilder {
    unlisted(bro).with_owner_window(panel.hwnd())
}
