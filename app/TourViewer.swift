// A checked tour, shown in a floating panel beside your editor: tour/tour.html, with the steps in a sidebar.
// Push Bro stands beside the panel's bottom edge in a see-through window of his own that moves with it, and
// plays the reactions the page asks for. Closing the panel quits the app, and opening another tour closes
// this one. There's no push lock, so pushes still play during a tour.
import AppKit
import WebKit

/// Each panel posts this as it opens, carrying its process id, and every other panel closes.
private let tourOpened = Notification.Name("app.gitceps.tour-opened")

/// Checks the tour and prints the result, or the problems and exits 1. Then a copy of this app shows it, so
/// whoever asked gets an answer now rather than when the tour is closed.
@MainActor func runTour(file: String) {
    let check: TourCheck, page: Page
    do {
        check = try TourCheck(file: URL(fileURLWithPath: file))
        page = try check.page()
    } catch {
        FileHandle.standardError.write(Data("\(error)\n".utf8))
        exit(1)
    }
    let json = try! JSONEncoder().encode(page)
    if ProcessInfo.processInfo.environment["GITCEPS_DRY"] != nil {
        print(String(decoding: json, as: UTF8.self))
        exit(0)
    }
    let steps = "\(page.steps.count) step\(page.steps.count == 1 ? "" : "s")"
    print("Tour checked: \(steps), \(page.shownLines) of \(page.shownLines + page.skippedLines) changed lines shown, \(page.skippedLines) skipped.")

    let files = page.overview.count
    let ins = page.overview.reduce(0) { $0 + $1.ins }, del = page.overview.reduce(0) { $0 + $1.del }
    let title = (page.number.map { "#\($0) · " } ?? "") + page.title
    let subtitle = "\(check.repo.lastPathComponent) · \(files) file\(files == 1 ? "" : "s") · +\(ins) −\(del)"
    guard let feed = startShowing(["--show", title, "--subtitle", subtitle]) else {
        log("can't open the tour")
        exit(1)
    }
    try! feed.write(contentsOf: json)   // the copy reads all of it before it does anything else
    try! feed.close()
    print("Tour opened.")
    exit(0)
}

/// Starts a copy of this app in a session of its own, so the tour stays open whatever happens to the command
/// that asked for it, writing to the log rather than to that command's output. Returns where to write the
/// checked tour for it.
private func startShowing(_ arguments: [String]) -> FileHandle? {
    let path = Bundle.main.executablePath!
    let feed = Pipe()
    let logPath = FileManager.default.homeDirectoryForCurrentUser.appending(path: "Library/Logs/gitceps.log").path
    let opened = open(logPath, O_WRONLY | O_APPEND | O_CREAT, 0o644)
    let logFile = opened >= 0 ? opened : open("/dev/null", O_WRONLY)   // a sandbox may not allow the log
    defer { close(logFile) }

    var actions = posix_spawn_file_actions_t(nil as OpaquePointer?)
    posix_spawn_file_actions_init(&actions)
    defer { posix_spawn_file_actions_destroy(&actions) }
    posix_spawn_file_actions_adddup2(&actions, feed.fileHandleForReading.fileDescriptor, 0)
    for output: Int32 in [1, 2] { posix_spawn_file_actions_adddup2(&actions, logFile, output) }

    var attributes = posix_spawnattr_t(nil as OpaquePointer?)
    posix_spawnattr_init(&attributes)
    defer { posix_spawnattr_destroy(&attributes) }
    posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_SETSID | POSIX_SPAWN_CLOEXEC_DEFAULT))   // only 0, 1 and 2 go along

    let argv = ([path] + arguments).map { strdup($0) } + [nil]
    defer { argv.forEach { free($0) } }
    var child = pid_t()
    guard posix_spawn(&child, path, &actions, &attributes, argv, environ) == 0 else { return nil }
    try? feed.fileHandleForReading.close()
    return feed.fileHandleForWriting
}

/// Shows a checked tour, which `runTour` hands over on standard input.
@MainActor func showTour(title: String, subtitle: String, home: URL) {
    let json = String(decoding: FileHandle.standardInput.readDataToEndOfFile(), as: UTF8.self)
    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)   // no Dock icon
    let viewer = TourViewer(json: json, title: title, subtitle: subtitle, home: home)
    app.delegate = viewer   // app.delegate is weak; `viewer` lives until the app quits
    app.run()
}

final class TourViewer: NSObject, NSApplicationDelegate, NSWindowDelegate, WKNavigationDelegate, WKScriptMessageHandler {
    private let json, title, subtitle: String
    private let home: URL
    private var panel: NSPanel?
    private var bro: NSWindow?
    private var broView: WKWebView?

    init(json: String, title: String, subtitle: String, home: URL) {
        self.json = json
        self.title = title
        self.subtitle = subtitle
        self.home = home
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let visible = screenUnderMouse().visibleFrame
        let size = NSSize(width: min(1100, visible.width * 0.66), height: min(820, visible.height * 0.9))
        let frame = NSRect(x: visible.maxX - size.width - 24, y: visible.midY - size.height / 2, width: size.width, height: size.height)

        let panel = TourPanel(contentRect: frame, styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        panel.isFloatingPanel = true   // beside your editor, never lost behind it
        panel.hidesOnDeactivate = false
        panel.title = title
        panel.subtitle = subtitle
        panel.minSize = NSSize(width: 760, height: 480)
        panel.setFrameAutosaveName("Tour")   // opens where the last tour was left
        panel.delegate = self

        // The sidebar's material is the system's own: the page leaves the sidebar see-through and paints the rest.
        let material = NSVisualEffectView(frame: NSRect(origin: .zero, size: size))
        material.material = .sidebar
        material.blendingMode = .behindWindow
        material.autoresizingMask = [.width, .height]
        let page = seeThroughWebView(frame: material.bounds, messages: ["done", "react", "bro"], handler: self)
        page.autoresizingMask = [.width, .height]
        page.navigationDelegate = self
        material.addSubview(page)
        panel.contentView = material
        page.loadFileURL(home.appending(path: "tour/tour.html"), allowingReadAccessTo: home)

        // Push Bro: a see-through window that clicks pass through, tied to the panel so it moves with it.
        let bro = NSWindow(contentRect: .zero, styleMask: .borderless, backing: .buffered, defer: false)
        makeSeeThrough(bro)
        bro.ignoresMouseEvents = true
        let broView = seeThroughWebView(frame: .zero, messages: [], handler: self)
        bro.contentView = broView
        broView.loadFileURL(URL(string: "overlay/overlay.html?stage=tour", relativeTo: home)!, allowingReadAccessTo: home)
        panel.addChildWindow(bro, ordered: .above)

        self.panel = panel
        self.bro = bro
        self.broView = broView
        placeBro()
        panel.orderFrontRegardless()   // in front, without taking the keyboard from the app you're typing in; a click does

        // Delivered at once: a panel in the background would otherwise only hear it once it's clicked.
        let center = DistributedNotificationCenter.default()
        center.addObserver(self, selector: #selector(tourDidOpen(_:)), name: tourOpened, object: nil, suspensionBehavior: .deliverImmediately)
        center.post(name: tourOpened, object: String(getpid()))
    }

    func windowWillClose(_ notification: Notification) {
        NSApp.terminate(nil)
    }

    @objc private func tourDidOpen(_ notification: Notification) {
        if notification.object as? String != String(getpid()) { panel?.close() }
    }

    func windowDidMove(_ notification: Notification) { placeBro() }
    func windowDidResize(_ notification: Notification) { placeBro() }

    /// He stands beside the panel's bottom edge, on whichever side has room on the screen, feet level with it.
    private func placeBro() {
        guard let panel, let bro else { return }
        let size = NSSize(width: 240, height: 233), reach: CGFloat = 70   // how far his window reaches over the panel
        let screen = (panel.screen ?? screenUnderMouse()).visibleFrame
        let left = panel.frame.minX - screen.minX >= size.width - reach
        let x = left ? panel.frame.minX - size.width + reach : panel.frame.maxX - reach
        bro.setFrame(NSRect(x: x, y: panel.frame.minY - 8, width: size.width, height: size.height), display: true)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        webView.callAsyncJavaScript("start(json)", arguments: ["json": json], in: nil, in: .page)
    }

    // A clicked web link opens in the default browser. Nothing else replaces the tour: not another kind of
    // link, nor a file dropped on the panel.
    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async -> WKNavigationActionPolicy {
        guard let url = navigationAction.request.url else { return .cancel }
        if navigationAction.navigationType == .linkActivated, ["http", "https"].contains(url.scheme) {
            NSWorkspace.shared.open(url)
            return .cancel
        }
        return url.isFileURL && url.path.hasPrefix(home.path + "/") ? .allow : .cancel
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        switch message.name {
        case "react":   // { react, bubbles } for Push Bro
            broView?.callAsyncJavaScript("cue(reaction)", arguments: ["reaction": message.body], in: nil, in: .page)
        case "bro":   // whether he's shown; faded rather than hidden, which would untie his window from the panel
            if let shown = message.body as? Bool { bro?.alphaValue = shown ? 1 : 0 }
        default:   // "done": Esc on the page
            panel?.close()
        }
    }
}

/// The panel answers the shortcuts a tour needs without a menu bar: Copy and Select All in the code, Close
/// and Quit.
final class TourPanel: NSPanel {
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard event.modifierFlags.intersection([.command, .shift, .option, .control]) == .command else {
            return super.performKeyEquivalent(with: event)
        }
        switch event.charactersIgnoringModifiers {
        case "c": return NSApp.sendAction(#selector(NSText.copy(_:)), to: nil, from: self)
        case "a": return NSApp.sendAction(#selector(NSText.selectAll(_:)), to: nil, from: self)
        case "w", "q":
            performClose(nil)
            return true
        default: return super.performKeyEquivalent(with: event)
        }
    }
}
