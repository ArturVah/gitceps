use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSScreen, NSStatusWindowLevel, NSWindowCollectionBehavior};
use objc2_foundation::{NSRect, NSString};
use tao::event_loop::{EventLoop, EventLoopWindowTarget};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS, WindowBuilderExtMacOS, WindowExtMacOS};
use tao::window::{Window, WindowBuilder};
use window_vibrancy::NSVisualEffectMaterial;
use wry::{WebView, WebViewBuilder, WebViewExtMacOS};

use super::{Area, Screen};

/// No Dock icon, and never the app in front.
pub fn stay_in_background<T>(event_loop: &mut EventLoop<T>) {
    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    event_loop.set_activate_ignoring_other_apps(false);
}

pub fn can_place_windows() -> bool {
    true
}

/// Every screen with its frame and its free part. Cocoa measures up from the bottom of the first screen, and
/// tao down from its top.
fn screens() -> Option<Vec<(Area, Area)>> {
    let screens = NSScreen::screens(MainThreadMarker::new()?);
    let top = screens.firstObject()?.frame().size.height;
    let area = |rect: NSRect| Area {
        x: rect.origin.x,
        y: top - rect.origin.y - rect.size.height,
        width: rect.size.width,
        height: rect.size.height,
    };
    Some(
        screens
            .iter()
            .map(|screen| (area(screen.frame()), area(screen.visibleFrame())))
            .collect(),
    )
}

pub fn mouse<T>(_: &EventLoopWindowTarget<T>) -> Option<(f64, f64)> {
    let (first, _) = *screens()?.first()?;
    let mouse = NSEvent::mouseLocation();
    Some((mouse.x, first.height - mouse.y))
}

/// The screen a point is on, without its menu bar and Dock.
pub fn screen_at<T>(_: &EventLoopWindowTarget<T>, x: f64, y: f64) -> Option<Screen> {
    let (_, usable) = screens()?.into_iter().find(|(frame, _)| frame.holds(x, y))?;
    Some(Screen { usable, scale: 1.0 })
}

pub fn without_shadow(window: WindowBuilder) -> WindowBuilder {
    window.with_has_shadow(false)
}

/// Nothing to do: a Mac lists no window of an app that has no Dock icon.
pub fn unlisted(window: WindowBuilder) -> WindowBuilder {
    window
}

pub fn build(page: WebViewBuilder, window: &Window) -> wry::Result<WebView> {
    page.build(window)
}

/// Above full-screen apps and on every desktop, without ever coming forward as a window of its own.
pub fn float_above_everything(page: &WebView) {
    let window = page.ns_window();
    window.setLevel(NSStatusWindowLevel);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    window.orderFrontRegardless();
}

/// The tour panel's look: the system's sidebar material behind the page, which leaves its sidebar see-through,
/// and a second line in the title bar.
pub fn dress_panel(panel: &Window, page: &WebView, subtitle: &str) {
    let _ = window_vibrancy::apply_vibrancy(panel, NSVisualEffectMaterial::Sidebar, None, None);
    let window = page.ns_window();
    window.setSubtitle(&NSString::from_str(subtitle));
    window.orderFrontRegardless(); // in front, without taking the keyboard from the app being typed in
}

/// Ties Push Bro's window to the panel, so the two move and close as one.
pub fn beside(bro: WindowBuilder, panel: &Window) -> WindowBuilder {
    bro.with_parent_window(panel.ns_window())
}

/// Copy and Select All only work on a Mac when a menu offers them, even one nobody sees.
pub fn edit_menu() -> Option<muda::Menu> {
    let menu = muda::Menu::new();
    let items: [&dyn muda::IsMenuItem; 2] = [
        &muda::PredefinedMenuItem::copy(None),
        &muda::PredefinedMenuItem::select_all(None),
    ];
    menu.append(&muda::Submenu::with_items("Edit", true, &items).ok()?)
        .ok()?;
    menu.init_for_nsapp();
    Some(menu)
}
