//! A push: overlay.html in a see-through, click-through window in the bottom-right corner of the screen under
//! the mouse. It plays one reaction, then the program quits.
use std::fs::File;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use serde_json::Value;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder};
use wry::WebView;

use crate::platform;
use crate::webview::{self, Note};

const WIDTH: f64 = 480.0;
const HEIGHT: f64 = 352.0;
const MARGIN: f64 = 12.0;

/// Plays one push. One bro on screen at a time: a push that arrives while he's busy waits its turn.
pub fn play(push: Value) -> ExitCode {
    let Some(turn) = wait_for_turn() else {
        eprintln!("gitceps: still busy after a minute, skipping this one");
        return ExitCode::SUCCESS;
    };
    let mut event_loop = EventLoopBuilder::<Note>::with_user_event().build();
    platform::stay_in_background(&mut event_loop);
    if !platform::can_place_windows() {
        return ExitCode::SUCCESS; // nowhere to stand: the push goes on without him
    }
    let notes = event_loop.create_proxy();
    // A safety net: a web view pauses animation in a window nobody can see (a locked or sleeping display),
    // so the page may never say it's done. The longest scene is about 16 seconds.
    let give_up = Instant::now() + Duration::from_secs(30);
    let mut overlay = None;

    event_loop.run(move |event, target, control_flow| {
        let _ = &turn; // kept until the program quits, when the system lets the next push have it
        match event {
            Event::NewEvents(StartCause::Init) => match open(target, &notes) {
                Ok(opened) => {
                    overlay = Some(opened);
                    *control_flow = ControlFlow::WaitUntil(give_up);
                }
                Err(error) => {
                    eprintln!("gitceps: can't show the push: {error}");
                    *control_flow = ControlFlow::ExitWithCode(1);
                }
            },
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => *control_flow = ControlFlow::Exit,
            Event::UserEvent(Note::Loaded(_)) => {
                if let Some((_, page)) = &overlay {
                    webview::call(page, "play", &push);
                    eprintln!("gitceps: playing {push}");
                }
            }
            // The page's only message: the reaction is over.
            Event::UserEvent(Note::Said(_)) => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    })
}

/// Waits for the lock every push takes, for up to a minute. The lock belongs to this process, so the system
/// releases it when the program quits, even after a crash.
fn wait_for_turn() -> Option<File> {
    let lock = File::create(std::env::temp_dir().join("gitceps.lock")).ok()?;
    let give_up = Instant::now() + Duration::from_secs(60);
    while lock.try_lock().is_err() {
        if Instant::now() > give_up {
            return None;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Some(lock)
}

fn open(target: &EventLoopWindowTarget<Note>, notes: &EventLoopProxy<Note>) -> Result<(Window, WebView), String> {
    let screen = platform::screen_under_mouse(target).ok_or("there is no screen")?;
    let (width, height, margin) = (WIDTH * screen.scale, HEIGHT * screen.scale, MARGIN * screen.scale);
    let corner = screen.usable;
    let window = platform::unlisted(platform::see_through(WindowBuilder::new()))
        .with_focused(false)
        .with_focusable(false)
        .with_always_on_top(true)
        .with_visible_on_all_workspaces(true)
        .with_inner_size(platform::size(width, height))
        .with_position(platform::position(
            corner.x + corner.width - width - margin,
            corner.y + corner.height - height - margin,
        ))
        .build(target)
        .map_err(|error| error.to_string())?;
    window
        .set_ignore_cursor_events(true)
        .map_err(|error| error.to_string())?;
    let page = webview::show(&window, "overlay/overlay.html", "overlay", notes).map_err(|error| error.to_string())?;
    platform::float_above_everything(&page);
    Ok((window, page))
}
