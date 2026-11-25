use gtk::gdk::Display;
use gtk::prelude::*;
use tao::event_loop::{EventLoop, EventLoopWindowTarget};
use tao::platform::unix::{WindowBuilderExtUnix, WindowExtUnix};
use tao::window::{Window, WindowBuilder};
use wry::{WebView, WebViewBuilder, WebViewBuilderExtUnix};

use super::{Area, Screen};

/// Asks for X11 where there is a choice, before any window exists. Under Wayland a program can't place its
/// own windows or keep them on top, and XWayland lets it.
pub fn prefer_x11() {
    gtk::gdk::set_allowed_backends("x11,wayland");
}

/// Nothing to do: a window here comes forward only when it asks to.
pub fn stay_in_background<T>(_: &mut EventLoop<T>) {}

/// Whether windows can be put in a corner or beside another one, which Wayland alone doesn't allow.
pub fn can_place_windows() -> bool {
    Display::default().is_some_and(|display| display.type_().name() == "GdkX11Display")
}

pub fn mouse<T>(_: &EventLoopWindowTarget<T>) -> Option<(f64, f64)> {
    let (_, x, y) = Display::default()?.default_seat()?.pointer()?.position();
    Some((f64::from(x), f64::from(y)))
}

/// The screen a point is on, without panels and docks.
pub fn screen_at<T>(_: &EventLoopWindowTarget<T>, x: f64, y: f64) -> Option<Screen> {
    let area = |rectangle: gtk::gdk::Rectangle| Area {
        x: f64::from(rectangle.x()),
        y: f64::from(rectangle.y()),
        width: f64::from(rectangle.width()),
        height: f64::from(rectangle.height()),
    };
    // GDK answers with the nearest monitor, so whether the point is really on it is checked here.
    let monitor = Display::default()?.monitor_at_point(x as i32, y as i32)?;
    area(monitor.geometry()).holds(x, y).then(|| Screen {
        usable: area(monitor.workarea()),
        scale: 1.0,
    })
}

/// A window manager draws shadows for the windows it frames, and these have no frame.
pub fn without_shadow(window: WindowBuilder) -> WindowBuilder {
    window
}

/// Kept out of the taskbar and the window switcher.
pub fn unlisted(window: WindowBuilder) -> WindowBuilder {
    window.with_skip_taskbar(true)
}

pub fn build(page: WebViewBuilder, window: &Window) -> wry::Result<WebView> {
    page.build_gtk(window.default_vbox().expect("tao gives every window a box to fill"))
}

/// Being always on top is all a window manager offers.
pub fn float_above_everything(_: &WebView) {}

/// A title bar here has one line, so the subtitle joins the title.
pub fn dress_panel(panel: &Window, _: &WebView, subtitle: &str) {
    panel.set_title(&format!("{} · {subtitle}", panel.title()));
}

/// Push Bro's window belongs to the panel, so the window manager keeps it above the panel.
pub fn beside(bro: WindowBuilder, panel: &Window) -> WindowBuilder {
    unlisted(bro).with_transient_for(panel.gtk_window())
}
