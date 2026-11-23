// A push: overlay/overlay.html in a see-through, click-through window in the bottom-right corner of the
// screen under the mouse. It plays one push, then the app quits.
import AppKit
import WebKit

final class Overlay: NSObject, NSApplicationDelegate, WKNavigationDelegate, WKScriptMessageHandler {
    private let push: [String: String]
    private let page: URL
    private var window: NSWindow?

    init(push: [String: String], page: URL) {
        self.push = push
        self.page = page
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let size = NSSize(width: 480, height: 352)
        let visible = screenUnderMouse().visibleFrame
        let frame = NSRect(x: visible.maxX - size.width - 12, y: visible.minY + 12, width: size.width, height: size.height)

        let window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
        makeSeeThrough(window)
        window.level = .statusBar
        window.ignoresMouseEvents = true
        window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]

        let webView = seeThroughWebView(frame: .zero, messages: ["done"], handler: self)
        webView.navigationDelegate = self
        window.contentView = webView
        webView.loadFileURL(page, allowingReadAccessTo: page.deletingLastPathComponent())
        window.orderFrontRegardless()
        self.window = window

        // Safety net: WebKit pauses animation in windows nobody can see (locked or sleeping display),
        // so the page may never say it's done. The longest scene is about 16 s.
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) { NSApp.terminate(nil) }
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        webView.callAsyncJavaScript("play(push)", arguments: ["push": push], in: nil, in: .page)
        log("playing \(push)")
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        log("done")
        NSApp.terminate(nil)
    }
}

/// Plays one push. One bro on screen at a time: a push that arrives while he's busy waits its turn, for
/// up to a minute. The lock belongs to this process, so the system releases it when the app quits, even on a crash.
@MainActor func playPush(_ push: [String: String], home: URL) {
    let page = home.appending(path: "overlay/overlay.html")
    guard FileManager.default.fileExists(atPath: page.path) else {
        log("missing \(page.path)")
        exit(1)
    }

    let lockPath = FileManager.default.temporaryDirectory.appending(path: "gitceps.lock").path
    let lock = open(lockPath, O_CREAT | O_RDWR, 0o600)
    let deadline = Date(timeIntervalSinceNow: 60)
    while lock >= 0 && flock(lock, LOCK_EX | LOCK_NB) != 0 {
        if Date() > deadline {
            log("still busy after a minute, skipping this one")
            exit(0)
        }
        Thread.sleep(forTimeInterval: 0.25)
    }

    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)   // no Dock icon, never takes focus
    let overlay = Overlay(push: push, page: page)   // app.delegate is weak, so keep a strong reference
    app.delegate = overlay
    app.run()
}
