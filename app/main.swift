// Gitceps' Mac app.
//
//   gitceps-app --mode tiny|normal|huge|forklift|force|rejected|secret|pull|amend|rebase|clone --ins N --del N --msg TEXT
//               [--secret FILE] [--remote REPO]
//     Plays one push in the corner of the screen (Overlay.swift).
//
//   gitceps-app --tour FILE
//     Checks a PR or commit tour against its diff (TourCheck.swift), then shows it in a floating panel
//     (TourViewer.swift). GITCEPS_DRY=1 prints what the panel would get instead. The panel is a copy of the
//     app started with --show TITLE --subtitle TEXT, which reads the checked tour on standard input.
import AppKit

func log(_ text: String) {
    FileHandle.standardError.write(Data("gitceps: \(text)\n".utf8))
}

// Arguments come in "--name value" pairs. The push page fills in defaults and converts numbers.
var options: [String: String] = [:]
var args = CommandLine.arguments.dropFirst().makeIterator()
while let flag = args.next(), let value = args.next() {
    options[String(flag.dropFirst(2))] = value
}

// The binary lives in <home>/app/, the pages in <home>/overlay/ and <home>/tour/.
let home = Bundle.main.executableURL!.resolvingSymlinksInPath()
    .deletingLastPathComponent().deletingLastPathComponent()

if let title = options["show"] {
    showTour(title: title, subtitle: options["subtitle"] ?? "", home: home)
} else if let tour = options["tour"] {
    runTour(file: tour)
} else {
    playPush(options, home: home)
}
