// What changed between two commits, line by line, from `git diff -U0`.
import Foundation

/// One changed line: added at a line of the new file, or the n-th of the lines removed just before one.
enum Change: Hashable {
    case added(path: String, line: Int)
    case removed(path: String, before: Int, index: Int)
}

/// What the diff says about one file. Removed lines are kept with the new line they were removed before.
struct FileDiff {
    var path: String
    var oldPath: String?
    var created = false
    var deleted = false
    var binary = false
    var oldMode: String?
    var newMode: String?
    var added = Set<Int>()
    var removed: [Int: [String]] = [:]

    var submodule: Bool { oldMode == "160000" || newMode == "160000" }

    var changes: [Change] {
        added.map { .added(path: path, line: $0) }
            + removed.flatMap { before, lines in lines.indices.map { .removed(path: path, before: before, index: $0) } }
    }

    /// What the tour's file list says about the file besides its line counts.
    var facts: [String] {
        let mode: String? = if let oldMode, let newMode, oldMode != newMode { "mode \(oldMode) → \(newMode)" } else { nil }
        return [created ? "new" : nil, deleted ? "deleted" : nil, oldPath.map { "renamed from \($0)" },
                binary ? "binary" : nil, submodule ? "submodule" : nil, mode].compactMap { $0 }
    }

    /// Takes in a second entry for the same path, which git writes when a file changes type (a symlink
    /// becomes a regular file, say): one entry deletes it, the other creates it, and each has one of its modes.
    mutating func absorb(_ other: FileDiff) {
        let replaced = (created || other.created) && (deleted || other.deleted)
        created = (created || other.created) && !replaced
        deleted = (deleted || other.deleted) && !replaced
        binary = binary || other.binary
        oldMode = oldMode ?? other.oldMode
        newMode = newMode ?? other.newMode
        added.formUnion(other.added)
        removed.merge(other.removed) { $0 + $1 }
    }
}

/// Reads `git diff -U0 -M` with the a/ and b/ prefixes. Paths git quotes, those with a quote, a backslash or
/// a control character in them, aren't read.
func parseDiff(_ text: String) -> [FileDiff] {
    var files: [FileDiff] = []
    var before = 0   // the new line the current hunk's removed lines sit before
    var inHunk = false
    func update(_ change: (inout FileDiff) -> Void) { change(&files[files.count - 1]) }
    func value(_ line: String, after prefix: String) -> String { String(line.dropFirst(prefix.count)) }

    for line in textLines(text) {
        if line.hasPrefix("diff --git ") {
            // "a/P b/P": without a rename both halves are the same path, spaces and all. A renamed file gets
            // its path from "rename to".
            let pair = line.dropFirst("diff --git ".count)
            files.append(FileDiff(path: String(pair.dropFirst(2).prefix((pair.count - 5) / 2))))
            inHunk = false
        } else if let hunk = line.firstMatch(of: /^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/) {
            let start = Int(hunk.1)!, count = hunk.2.map { Int($0)! } ?? 1
            update { $0.added.formUnion(start..<start + count) }
            before = count > 0 ? start : start + 1
            inHunk = true
        } else if inHunk {
            if line.hasPrefix("-") { update { $0.removed[before, default: []].append(String(line.dropFirst())) } }
        } else if line.hasPrefix("rename from ") {
            update { $0.oldPath = value(line, after: "rename from ") }
        } else if line.hasPrefix("rename to ") {
            update { $0.path = value(line, after: "rename to ") }
        } else if line.hasPrefix("new file mode ") {
            update { $0.created = true; $0.newMode = value(line, after: "new file mode ") }
        } else if line.hasPrefix("deleted file mode ") {
            update { $0.deleted = true; $0.oldMode = value(line, after: "deleted file mode ") }
        } else if line.hasPrefix("old mode ") {
            update { $0.oldMode = value(line, after: "old mode ") }
        } else if line.hasPrefix("new mode ") {
            update { $0.newMode = value(line, after: "new mode ") }
        } else if line.hasPrefix("index ") {
            // "index <old>..<new> <mode>", where the mode is there only when it didn't change.
            let parts = line.split(separator: " ")
            if parts.count == 3 { update { $0.oldMode = String(parts[2]); $0.newMode = String(parts[2]) } }
        } else if line.hasPrefix("Binary files ") {
            update { $0.binary = true }
        }
    }

    var merged: [FileDiff] = [], at: [String: Int] = [:]
    for file in files {
        if let index = at[file.path] { merged[index].absorb(file) } else { at[file.path] = merged.count; merged.append(file) }
    }
    // A submodule's diff is only the commit it points at, which isn't a line of code.
    for index in merged.indices where merged[index].submodule {
        merged[index].added = []
        merged[index].removed = [:]
    }
    return merged
}
