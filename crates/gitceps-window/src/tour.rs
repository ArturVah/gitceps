//! A checked tour, shown in a floating panel beside the editor: tour.html, with the steps in a sidebar. Push
//! Bro stands beside the panel's bottom edge in a see-through window of his own that moves with it, and plays
//! the reactions the page asks for. Closing the panel quits the program, and opening another tour closes this
//! one. There's no push lock, so pushes still play during a tour.
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tao::dpi::Position;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder};
use wry::WebView;

use crate::platform::{self, Area};
use crate::webview::{self, Note};

/// How often the panel looks at whether a newer tour has opened, and saves where it was moved to.
const TICK: Duration = Duration::from_millis(250);
/// Push Bro's window, and how far it reaches over the panel.
const BRO: (f64, f64) = (240.0, 233.0);
const REACH: f64 = 70.0;

struct Tour {
    panel: Window,
    page: WebView,
    bro: Option<(Window, WebView)>,
}

pub fn show(title: String, subtitle: String, tour: String) -> ExitCode {
    let mut event_loop = EventLoopBuilder::<Note>::with_user_event().build();
    platform::stay_in_background(&mut event_loop);
    let notes = event_loop.create_proxy();
    #[cfg(target_os = "macos")]
    let _menu = platform::edit_menu();
    let this = std::process::id();
    let mut open: Option<Tour> = None;
    let mut saved: Option<Value> = None;
    let mut started = false;

    event_loop.run(move |event, target, control_flow| {
        if matches!(event, Event::NewEvents(StartCause::Init)) {
            match open_tour(target, &notes, &title, &subtitle) {
                Ok(tour) => {
                    saved = place(&tour.panel);
                    newest_tour(Some(this));
                    open = Some(tour);
                    *control_flow = ControlFlow::WaitUntil(Instant::now() + TICK);
                }
                Err(error) => {
                    eprintln!("gitceps: can't show the tour: {error}");
                    *control_flow = ControlFlow::ExitWithCode(1);
                    return;
                }
            }
        }
        let Some(shown) = &open else { return };
        match event {
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let now = place(&shown.panel);
                if now != saved {
                    remember(now.as_ref());
                    saved = now;
                }
                // Only another tour's id closes this one, not a file that can't be read.
                *control_flow = if newest_tour(None).is_some_and(|newest| newest != this) {
                    ControlFlow::Exit
                } else {
                    ControlFlow::WaitUntil(Instant::now() + TICK)
                };
            }
            // Once: a page can report that it loaded again, after a link it wasn't allowed to follow.
            Event::UserEvent(Note::Loaded("tour")) if !started => {
                started = true;
                webview::call(&shown.page, "start", &Value::String(tour.clone()));
            }
            Event::UserEvent(Note::Said(message)) => {
                let message: Value = serde_json::from_str(&message).unwrap_or_default();
                match (message["name"].as_str(), &shown.bro) {
                    // { react, bubbles } for Push Bro.
                    (Some("react"), Some((_, bro))) => webview::call(bro, "cue", &message["body"]),
                    // Whether he's shown. His page fades, since hiding the window would untie it from the panel.
                    (Some("bro"), Some((_, bro))) => {
                        let opacity = if message["body"] == true { 1 } else { 0 };
                        let _ = bro.evaluate_script(&format!("document.documentElement.style.opacity = {opacity}"));
                    }
                    // Esc on the page, its Close button, or the shortcut for closing a window.
                    (Some("done"), _) => *control_flow = ControlFlow::Exit,
                    _ => {}
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Moved(_) | WindowEvent::Resized(_),
                window_id,
                ..
            } if window_id == shown.panel.id() => {
                if let (Some((bro, _)), Some(at)) = (&shown.bro, bro_position(target, &shown.panel)) {
                    bro.set_outer_position(at);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    })
}

fn open_tour(
    target: &EventLoopWindowTarget<Note>,
    notes: &EventLoopProxy<Note>,
    title: &str,
    subtitle: &str,
) -> Result<Tour, String> {
    // Where the last tour was left, if that is still on a screen. Otherwise by the right edge of the screen
    // under the mouse.
    let last = remembered().and_then(|at| Some((at, platform::screen_at(target, at.x + at.width / 2.0, at.y)?)));
    let (at, screen) = match last {
        Some(last) => last,
        None => {
            let screen = platform::screen_under_mouse(target).ok_or("there is no screen")?;
            let (free, scale) = (screen.usable, screen.scale);
            let width = (1100.0 * scale).min(free.width * 0.66);
            let height = (820.0 * scale).min(free.height * 0.9);
            let (x, y) = (
                free.x + free.width - width - 24.0 * scale,
                free.y + (free.height - height) / 2.0,
            );
            (Area { x, y, width, height }, screen)
        }
    };
    let (free, scale) = (screen.usable, screen.scale);
    let panel = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(platform::size(at.width.min(free.width), at.height.min(free.height)))
        .with_min_inner_size(platform::size(760.0 * scale, 480.0 * scale))
        .with_position(platform::position(at.x, at.y))
        .with_always_on_top(true) // beside the editor, never lost behind it
        .with_focused(false)
        .with_minimizable(false)
        .with_maximizable(false)
        .build(target)
        .map_err(|error| error.to_string())?;
    // A Mac reads the position above as the corner under the title bar. This is the frame's corner everywhere.
    panel.set_outer_position(platform::position(at.x, at.y));
    // Only a Mac has a material to put behind the sidebar. Elsewhere the page paints it.
    let address = if cfg!(target_os = "macos") {
        "tour/tour.html"
    } else {
        "tour/tour.html?plain"
    };
    let page = webview::show(&panel, address, "tour", notes).map_err(|error| error.to_string())?;
    platform::dress_panel(&panel, &page, subtitle);

    let mut tour = Tour { panel, page, bro: None };
    if platform::can_place_windows() {
        let mut bro = platform::see_through(WindowBuilder::new())
            .with_focused(false)
            .with_focusable(false)
            .with_inner_size(platform::size(BRO.0 * scale, BRO.1 * scale));
        if let Some(at) = bro_position(target, &tour.panel) {
            bro = bro.with_position(at);
        }
        let bro = platform::beside(bro, &tour.panel)
            .build(target)
            .map_err(|error| error.to_string())?;
        bro.set_ignore_cursor_events(true).map_err(|error| error.to_string())?;
        let stage = webview::show(&bro, "overlay/overlay.html?stage=tour", "bro", notes);
        tour.bro = Some((bro, stage.map_err(|error| error.to_string())?));
    }
    Ok(tour)
}

/// He stands beside the panel's bottom edge, on whichever side has room on the screen, feet level with it.
fn bro_position(target: &EventLoopWindowTarget<Note>, panel: &Window) -> Option<Position> {
    let (panel, scale) = platform::frame(panel)?;
    let (width, height, reach) = (BRO.0 * scale, BRO.1 * scale, REACH * scale);
    let screen = platform::screen_at(target, panel.x + panel.width / 2.0, panel.y + panel.height / 2.0);
    let room = screen.map_or(0.0, |screen| panel.x - screen.usable.x);
    let x = if room >= width - reach {
        panel.x - width + reach
    } else {
        panel.x + panel.width - reach
    };
    Some(platform::position(x, panel.y + panel.height - height + 8.0 * scale))
}

/// The panel's place as it goes into the file that remembers it: the frame's corner, and the size of its
/// inside, which is what a new panel is asked for.
fn place(panel: &Window) -> Option<Value> {
    let (frame, _) = platform::frame(panel)?;
    let (width, height) = platform::inner_size(panel);
    Some(json!({ "x": frame.x, "y": frame.y, "width": width, "height": height }))
}

fn folder() -> Option<PathBuf> {
    Some(std::env::home_dir()?.join(".gitceps"))
}

fn remember(place: Option<&Value>) {
    if let (Some(place), Some(folder)) = (place, folder()) {
        let _ = std::fs::create_dir_all(&folder);
        let _ = std::fs::write(folder.join("tour-window.json"), place.to_string());
    }
}

fn remembered() -> Option<Area> {
    let place: Value = serde_json::from_slice(&std::fs::read(folder()?.join("tour-window.json")).ok()?).ok()?;
    let number = |name: &str| place[name].as_f64();
    Some(Area {
        x: number("x")?,
        y: number("y")?,
        width: number("width")?,
        height: number("height")?,
    })
}

/// Each tour writes its process id here as it opens, and every other tour closes when it reads a newer one.
fn newest_tour(opened: Option<u32>) -> Option<u32> {
    let file = folder()?.join("tour.pid");
    if let Some(id) = opened {
        let _ = std::fs::create_dir_all(file.parent()?);
        let _ = std::fs::write(&file, id.to_string());
    }
    std::fs::read_to_string(file).ok()?.trim().parse().ok()
}
