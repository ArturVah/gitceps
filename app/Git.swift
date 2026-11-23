// Reading from git: running it, and reading many files of a commit at once.
import Foundation

struct GitFailure: Error, CustomStringConvertible {
    let description: String
}

/// Runs the git on the PATH in a repo, with `input` on its standard input, and returns what it printed.
func git(_ arguments: [String], in repo: URL, input: Data?) throws -> Data {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
    process.arguments = ["git", "-C", repo.path, "-c", "core.quotePath=false"] + arguments
    let output = Pipe(), errors = Pipe(), feed = Pipe()
    process.standardOutput = output
    process.standardError = errors
    if input != nil { process.standardInput = feed }
    try process.run()
    if let input {   // written while the output is read, so neither pipe can fill up and stall git
        let writer = feed.fileHandleForWriting
        DispatchQueue.global().async {
            try? writer.write(contentsOf: input)
            try? writer.close()
        }
    }
    let data = output.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    guard process.terminationStatus == 0 else {
        let said = String(decoding: errors.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
        throw GitFailure(description: "git \(arguments.joined(separator: " ")): \(said.trimmingCharacters(in: .whitespacesAndNewlines))")
    }
    return data
}

func git(_ arguments: [String], in repo: URL) throws -> String {
    String(decoding: try git(arguments, in: repo, input: nil), as: UTF8.self)
}

/// Text files as they are in a commit, read by one git process rather than one per file. A path that isn't a
/// text file there (missing, binary, or a submodule) is left out.
func files(_ paths: [String], at commit: String, in repo: URL) throws -> [String: [String]] {
    let data = try git(["cat-file", "--batch"], in: repo, input: Data(paths.map { "\(commit):\($0)\n" }.joined().utf8))
    var found: [String: [String]] = [:]
    var at = data.startIndex
    for path in paths {
        // Each answer is "<id> <type> <size>\n<content>\n", or "<name> missing\n".
        guard let end = data[at...].firstIndex(of: UInt8(ascii: "\n")) else { break }
        let header = String(decoding: data[at..<end], as: UTF8.self).split(separator: " ")
        at = data.index(after: end)
        guard header.count == 3, let size = Int(header[2]) else { continue }
        let content = data[at..<at + size]
        if header[1] == "blob" && !content.contains(0) { found[path] = textLines(String(decoding: content, as: UTF8.self)) }
        at += size + 1
    }
    return found
}

/// Splits on "\n" and drops the "\r" of a Windows line ending. Swift reads "\r\n" as one character, so
/// splitting a String on "\n" alone would leave those lines joined.
func textLines(_ text: String) -> [String] {
    var lines = text.components(separatedBy: "\n").map { $0.hasSuffix("\r") ? String($0.dropLast()) : $0 }
    if lines.last == "" { lines.removeLast() }
    return lines
}
