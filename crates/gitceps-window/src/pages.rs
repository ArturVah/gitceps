//! The pages the windows show, built into the program and served to the web view under one address, so
//! nothing is read from disk.
use std::borrow::Cow;

use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};

/// The name the pages are served under, as gitceps://localhost/.
pub const SCHEME: &str = "gitceps";

/// Where the web view finds a page. Windows serves a custom scheme as a web address of its own.
pub fn address(page: &str) -> String {
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/{page}")
    } else {
        format!("{SCHEME}://localhost/{page}")
    }
}

/// Whether an address is one of the pages here, and not somewhere a link leads.
pub fn is_ours(address: &str) -> bool {
    address.starts_with(&self::address(""))
}

pub fn serve(request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let (content, kind): (&[u8], _) = match request.uri().path() {
        "/overlay/overlay.html" => (include_bytes!("../pages/overlay/overlay.html"), "text/html"),
        "/overlay/engine.js" => (include_bytes!("../pages/overlay/engine.js"), "text/javascript"),
        "/overlay/scenes.js" => (include_bytes!("../pages/overlay/scenes.js"), "text/javascript"),
        "/tour/tour.html" => (include_bytes!("../pages/tour/tour.html"), "text/html"),
        "/tour/tour.js" => (include_bytes!("../pages/tour/tour.js"), "text/javascript"),
        _ => return respond(Response::builder().status(404), &[]),
    };
    respond(
        Response::builder().header(CONTENT_TYPE, format!("{kind}; charset=utf-8")),
        content,
    )
}

fn respond(response: wry::http::response::Builder, content: &'static [u8]) -> Response<Cow<'static, [u8]>> {
    response
        .body(Cow::Borrowed(content))
        .expect("a status and one header make a response")
}
