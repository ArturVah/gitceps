// What a push and a tour share: the screen under the mouse, and a see-through window whose page draws all
// there is to see.
import AppKit
import WebKit

@MainActor func screenUnderMouse() -> NSScreen {
    let mouse = NSEvent.mouseLocation
    return NSScreen.screens.first { NSMouseInRect(mouse, $0.frame, false) } ?? NSScreen.screens[0]
}

@MainActor func makeSeeThrough(_ window: NSWindow) {
    window.isOpaque = false
    window.backgroundColor = .clear
    window.hasShadow = false
}

/// A web view that draws nothing behind its page. The page's messages with these names go to `handler`.
@MainActor func seeThroughWebView(frame: NSRect, messages: [String], handler: any WKScriptMessageHandler) -> WKWebView {
    let config = WKWebViewConfiguration()
    for name in messages { config.userContentController.add(handler, name: name) }
    let webView = WKWebView(frame: frame, configuration: config)
    webView.setValue(false, forKey: "drawsBackground")   // macOS has no public API for a transparent WKWebView
    return webView
}
