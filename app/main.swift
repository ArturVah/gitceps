// Gitceps' Mac app. Shows overlay/overlay.html in a see-through, click-through window in the
// bottom-right corner of the screen under the mouse, plays one push, then quits.
// Usage: gitceps-app --mode tiny|normal|huge|forklift|force|rejected|secret --ins N --del N --msg TEXT
//                [--secret FILE] [--remote REPO]
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
        let mouse = NSEvent.mouseLocation
        let screen = NSScreen.screens.first { NSMouseInRect(mouse, $0.frame, false) } ?? NSScreen.screens[0]
        let visible = screen.visibleFrame
        let frame = NSRect(x: visible.maxX - size.width - 12, y: visible.minY + 12, width: size.width, height: size.height)

        let window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
        window.isOpaque = false
        window.backgroundColor = .clear
        window.hasShadow = false
        window.level = .statusBar
        window.ignoresMouseEvents = true
        window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]

        let config = WKWebViewConfiguration()
        config.userContentController.add(self, name: "done")
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.setValue(false, forKey: "drawsBackground")   // macOS has no public API for a transparent WKWebView
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

func log(_ text: String) {
    FileHandle.standardError.write(Data("gitceps: \(text)\n".utf8))
}

// Arguments come in "--name value" pairs. The page fills in defaults and converts numbers.
var push: [String: String] = [:]
var args = CommandLine.arguments.dropFirst().makeIterator()
while let flag = args.next(), let value = args.next() {
    push[String(flag.dropFirst(2))] = value
}

// The binary lives in <home>/app/, the page in <home>/overlay/.
let page = Bundle.main.executableURL!.resolvingSymlinksInPath()
    .deletingLastPathComponent().deletingLastPathComponent()
    .appending(path: "overlay/overlay.html")
guard FileManager.default.fileExists(atPath: page.path) else {
    log("missing \(page.path)")
    exit(1)
}

// One bro on screen at a time. A push that arrives while he's busy waits its turn, for up to a
// minute. The lock belongs to this process, so the system releases it when the app quits, even on a crash.
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
