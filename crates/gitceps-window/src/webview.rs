//! What a push and a tour share: a see-through web view on one of the built-in pages, whose page draws all
//! there is to see and talks back through messages.
use tao::event_loop::EventLoopProxy;
use tao::window::Window;
use wry::{NewWindowResponse, PageLoadEvent, WebView, WebViewBuilder};

use crate::pages;

/// What a page tells the window that shows it.
pub enum Note {
    /// The page has loaded and can be called.
    Loaded(&'static str),
    /// A message from the page, as JSON: { name, body }.
    Said(String),
}

/// Shows a page in a window. `name` comes back in `Note::Loaded`, to tell the pages of one program apart.
pub fn show(window: &Window, page: &str, name: &'static str, notes: &EventLoopProxy<Note>) -> wry::Result<WebView> {
    let (loaded, said) = (notes.clone(), notes.clone());
    let builder = WebViewBuilder::new()
        .with_transparent(true)
        .with_focused(false)
        .with_accept_first_mouse(true)
        .with_custom_protocol(pages::SCHEME.to_string(), |_, request| pages::serve(&request))
        .with_url(pages::address(page))
        // A clicked web link opens in the default browser. Nothing else replaces the page: not another kind
        // of link, nor a file dropped on the window.
        .with_navigation_handler(|address| {
            let ours = pages::is_ours(&address);
            if !ours {
                open_in_browser(&address);
            }
            ours
        })
        .with_new_window_req_handler(|address, _| {
            open_in_browser(&address);
            NewWindowResponse::Deny
        })
        .with_on_page_load_handler(move |event, _| {
            if matches!(event, PageLoadEvent::Finished) {
                let _ = loaded.send_event(Note::Loaded(name));
            }
        })
        .with_ipc_handler(move |message| {
            let _ = said.send_event(Note::Said(message.into_body()));
        });
    crate::platform::build(builder, window)
}

fn open_in_browser(address: &str) {
    if address.starts_with("http://") || address.starts_with("https://") {
        let _ = open::that_detached(address);
    }
}

/// Calls a function of the page with one argument.
pub fn call(page: &WebView, function: &str, argument: &serde_json::Value) {
    if let Err(error) = page.evaluate_script(&format!("{function}({argument})")) {
        eprintln!("gitceps: can't call {function}: {error}");
    }
}
