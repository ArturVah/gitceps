//! What each system does its own way: where the screen's free space is, how a window stays out of the way,
//! and how the web view is put into a window.
use tao::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Size};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::{Window, WindowBuilder};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

/// A rectangle on the desktop, in the units the system places windows in: pixels on Windows, and points,
/// which a sharp screen draws with several pixels each, on a Mac and on Linux.
#[derive(Clone, Copy)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(not(windows))]
impl Area {
    fn holds(&self, x: f64, y: f64) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }
}

pub struct Screen {
    /// The screen without the menu bar, Dock or taskbar.
    pub usable: Area,
    /// How many of the desktop's units one pixel of a page takes.
    pub scale: f64,
}

/// The screen under the mouse, or the first screen when the mouse can't be found.
pub fn screen_under_mouse<T>(target: &EventLoopWindowTarget<T>) -> Option<Screen> {
    let under_mouse = mouse(target).and_then(|(x, y)| screen_at(target, x, y));
    under_mouse.or_else(|| screen_at(target, 0.0, 0.0))
}

pub fn position(x: f64, y: f64) -> Position {
    if cfg!(windows) {
        PhysicalPosition::new(x, y).into()
    } else {
        LogicalPosition::new(x, y).into()
    }
}

pub fn size(width: f64, height: f64) -> Size {
    if cfg!(windows) {
        PhysicalSize::new(width, height).into()
    } else {
        LogicalSize::new(width, height).into()
    }
}

/// A window that shows nothing of its own: no frame, no background and no shadow.
pub fn see_through(window: WindowBuilder) -> WindowBuilder {
    without_shadow(window.with_decorations(false).with_transparent(true))
}

/// Where a window is, title bar and all, and how many desktop units a pixel of its page takes.
pub fn frame(window: &Window) -> Option<(Area, f64)> {
    let (at, size) = (window.outer_position().ok()?, window.outer_size());
    let pixels = pixels_per_unit(window);
    let area = Area {
        x: f64::from(at.x) / pixels,
        y: f64::from(at.y) / pixels,
        width: f64::from(size.width) / pixels,
        height: f64::from(size.height) / pixels,
    };
    Some((area, window.scale_factor() / pixels))
}

/// The size of a window's inside, which is what a window is asked for when it's built.
pub fn inner_size(window: &Window) -> (f64, f64) {
    let (size, pixels) = (window.inner_size(), pixels_per_unit(window));
    (f64::from(size.width) / pixels, f64::from(size.height) / pixels)
}

/// tao reports in pixels everywhere, which only Windows also places windows in.
fn pixels_per_unit(window: &Window) -> f64 {
    if cfg!(windows) { 1.0 } else { window.scale_factor() }
}
