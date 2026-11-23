// Checks a tour file against the real diff between its base and head, and builds the page from it.
//
// Every changed line must be shown in a step or listed under "skipped" with a reason. Only blank lines are
// listed without the tour placing them. Judging what's routine is the agent's job: no pattern can tell a
// comment from a build directive (//go:embed) or an attribute (#[IsGranted]) that only looks like one.
import Foundation

struct TourProblems: Error, CustomStringConvertible {
    let list: [String]
    var description: String { list.joined(separator: "\n") }
}

/// The reactions a step may name: REVIEW in overlay/scenes.js, without "finale", which the page plays itself.
let reactions = ["intro", "nod", "inspect", "guard", "alarm", "nitpick", "unsure"]
let bubbleLimit = 28   // characters that fit his speech bubble
let stepLimit = 7
let cardLimit = 300   // rows a card carries; for more, its link opens the file

final class TourCheck {
    private let tour: TourFile
    let repo: URL
    private let head: String
    private let diffs: [FileDiff]
    private let byPath: [String: FileDiff]
    private var contents: [String: [String]?]
    private var problems: [String] = []

    init(file: URL) throws {
        let tour: TourFile
        do {
            tour = try JSONDecoder().decode(TourFile.self, from: Data(contentsOf: file))
        } catch let error as DecodingError {
            throw TourProblems(list: ["\(file.path): \(explain(error))"])
        }
        let repo = URL(fileURLWithPath: tour.repo, relativeTo: file.deletingLastPathComponent()).standardizedFileURL
        let resolve = { (rev: String, kind: String) in
            try git(["rev-parse", "--verify", "\(rev)^{\(kind)}"], in: repo).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        let head = try resolve(tour.head, "commit")
        let base = try resolve(tour.base, "tree")   // a tree, so a root commit can use the empty tree
        // The format is pinned, so the user's own diff settings (noprefix, interHunkContext, …) can't change it.
        let diffs = parseDiff(try git(["diff", "-U0", "-M", "--no-color", "--no-ext-diff", "--no-textconv", "--no-relative",
                                       "--src-prefix=a/", "--dst-prefix=b/", "--inter-hunk-context=0", "--submodule=short",
                                       base, head, "--"], in: repo))
        let readable = diffs.filter { !$0.deleted && !$0.binary }.map(\.path)
        self.tour = tour
        self.repo = repo
        self.head = head
        self.diffs = diffs
        byPath = Dictionary(uniqueKeysWithValues: diffs.map { ($0.path, $0) })   // parseDiff merges entries by path
        contents = try files(readable, at: head, in: repo).mapValues { Optional($0) }
    }

    /// The page, or every problem found, so the agent can fix them all in one go.
    func page() throws -> Page {
        if !(1...stepLimit).contains(tour.steps.count) {
            problems.append("A tour has 1 to \(stepLimit) steps; this one has \(tour.steps.count). Split a bigger change into several tours.")
        }

        var shown = Set<Change>()
        var steps: [Page.Step] = []
        for (i, step) in tour.steps.enumerated() {
            let ranges = step.ranges.filter { valid($0, in: "step \(i + 1)") }
            let found = changes(in: ranges)
            steps.append(checked(step, in: "step \(i + 1)", covers: found.subtracting(shown).count, cards: cards(for: ranges)))
            shown.formUnion(found)
        }

        var listed = Set<Change>()
        var groups: [Page.Group] = []
        for (i, skip) in (tour.skipped ?? []).enumerated() {
            let ranges = skip.ranges.filter { valid($0, in: "skipped group \(i + 1)") }
            let found = changes(in: ranges).subtracting(shown).subtracting(listed)
            if found.isEmpty { continue }   // nothing in it that isn't shown or listed already
            groups.append(Page.Group(reason: skip.reason, count: found.count, cards: cards(listing: found)))
            listed.formUnion(found)
        }

        let left = diffs.flatMap(\.changes).filter { !shown.contains($0) && !listed.contains($0) }
        let blank = left.filter(isBlank), unplaced = left.filter { !isBlank($0) }
        if !blank.isEmpty {
            groups.append(Page.Group(reason: "Blank lines", count: blank.count, cards: cards(listing: Set(blank))))
            listed.formUnion(blank)
        }
        if !unplaced.isEmpty {
            problems.append("These changed lines aren't in the tour. Show them in a step, or list them under \"skipped\" with a reason. As ranges:")
            problems += describe(ranges(covering: unplaced))
        }
        if !problems.isEmpty { throw TourProblems(list: problems) }

        let overview = diffs.map { diff in
            Page.FileStat(path: diff.path, ins: diff.added.count, del: diff.changes.count - diff.added.count, facts: diff.facts)
        }
        return Page(number: tour.number, title: tour.title, summary: tour.summary, flag: tour.flag, overview: overview,
                    shownLines: shown.count, skippedLines: listed.count, steps: steps, skipped: groups)
    }

    // MARK: - Steps

    private func checked(_ step: TourFile.Step, in place: String, covers: Int, cards: [Card]) -> Page.Step {
        for (name, text) in [("title", step.title), ("why", step.why), ("bro", step.bro)]
        where text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            problems.append("\(place): \(name) is empty")
        }
        if !reactions.contains(step.react) {
            problems.append("\(place): react \"\(step.react)\" isn't one of \(reactions.joined(separator: ", "))")
        }
        for bubble in step.bubbles where bubble.count > bubbleLimit {
            problems.append("\(place): \"\(bubble)\" is \(bubble.count) characters; a bubble fits \(bubbleLimit)")
        }
        return Page.Step(title: step.title, why: step.why, bro: step.bro,
                         react: step.react, bubbles: step.bubbles,
                         note: step.note.map { checked($0, in: place) }, covers: covers, cards: cards)
    }

    private func checked(_ note: TourFile.Note, in place: String) -> Page.Note {
        if !["good", "warn"].contains(note.kind) {
            problems.append("\(place): note kind \"\(note.kind)\" isn't good or warn")
        }
        var link: String?
        if let source = note.source {   // "path:line", where the path may itself contain colons
            if let colon = source.lastIndex(of: ":"), let line = Int(source[source.index(after: colon)...]),
               let text = lines(of: String(source[..<colon])), line >= 1, line <= text.count {
                link = self.link(String(source[..<colon]), line, line)
            } else {
                problems.append("\(place): note source \"\(source)\" isn't path:line of a line in head")
            }
        }
        return Page.Note(kind: note.kind, text: note.text, source: note.source, link: link)
    }

    // MARK: - Lines

    private func lines(of path: String) -> [String]? {
        if let known = contents[path] { return known }
        let found = try? files([path], at: head, in: repo)[path]   // a file a step shows for context
        contents[path] = found
        return found
    }

    private func valid(_ range: LineRange, in place: String) -> Bool {
        guard range.from >= 1, range.from <= range.to else {
            problems.append("\(place): \(range.path) \(range.from)-\(range.to) isn't a range of lines")
            return false
        }
        if range.part == .del {
            guard let removed = byPath[range.path]?.removed.keys.sorted(), !removed.isEmpty else {
                problems.append("\(place): no lines were removed from \(range.path)")
                return false
            }
            if !removed.contains(where: (range.from...range.to).contains) {
                let at = removed.map(String.init).joined(separator: ", ")
                problems.append("\(place): no lines were removed at \(range.path) \(range.from)-\(range.to); they were removed before lines \(at)")
                return false
            }
            return true
        }
        guard let text = lines(of: range.path) else {
            problems.append("\(place): \(range.path) isn't a text file in head")
            return false
        }
        if range.to > text.count {
            problems.append("\(place): \(range.path) has \(text.count) lines, so \(range.from)-\(range.to) runs past its end")
            return false
        }
        return true
    }

    private func changes(in ranges: [LineRange]) -> Set<Change> {
        var found = Set<Change>()
        for range in ranges {
            guard let diff = byPath[range.path] else { continue }
            if range.part != .del {
                for line in range.from...range.to where diff.added.contains(line) { found.insert(.added(path: range.path, line: line)) }
            }
            if range.part != .nodel {
                for (before, removed) in diff.removed where (range.from...range.to).contains(before) {
                    for index in removed.indices { found.insert(.removed(path: range.path, before: before, index: index)) }
                }
            }
        }
        return found
    }

    private func isBlank(_ change: Change) -> Bool {
        let text: String? = switch change {
        case .added(let path, let line): lines(of: path)?[line - 1]
        case .removed(let path, let before, let index): byPath[path]?.removed[before]?[index]
        }
        return text?.trimmingCharacters(in: .whitespaces).isEmpty == true
    }

    // MARK: - Cards

    /// One card per file, in line order, with a gap row between pieces that don't touch. A removed line sits
    /// just before the new line it was removed before. With `only`, other changed lines are left out.
    private func cards(for ranges: [LineRange], only: Set<Change>? = nil) -> [Card] {
        var paths: [String] = []
        var newLines: [String: Set<Int>] = [:], removedBefore: [String: Set<Int>] = [:]
        for range in ranges {
            if newLines[range.path] == nil && removedBefore[range.path] == nil { paths.append(range.path) }
            if range.part != .del { newLines[range.path, default: []].formUnion(range.from...range.to) }
            if range.part != .nodel, let removed = byPath[range.path]?.removed {
                removedBefore[range.path, default: []].formUnion(removed.keys.filter { (range.from...range.to).contains($0) })
            }
        }
        return paths.compactMap { path in
            let diff = byPath[path], text = lines(of: path)
            let shownNew = newLines[path] ?? [], shownRemoved = removedBefore[path] ?? []
            let positions = shownNew.union(shownRemoved).sorted()
            guard let from = positions.first, let to = positions.last else { return nil }
            var rows: [Line] = [], piece: [Line] = []
            for (i, number) in positions.enumerated() {
                if i > 0 && number > positions[i - 1] + 1 {
                    rows += dedented(piece) + [.gap]
                    piece = []
                }
                if shownRemoved.contains(number) {
                    for (index, removed) in (diff?.removed[number] ?? []).enumerated()
                    where only?.contains(.removed(path: path, before: number, index: index)) ?? true {
                        piece.append(Line(number: nil, kind: "-", text: removed))
                    }
                }
                if shownNew.contains(number), let text, only?.contains(.added(path: path, line: number)) ?? true {
                    let added = diff?.added.contains(number) ?? false
                    piece.append(Line(number: number, kind: added ? "+" : " ", text: text[number - 1]))
                }
            }
            rows += dedented(piece)
            // Lines removed at the end of a file sit one past its last line, which isn't a line to name or link to.
            let end = max(1, text?.count ?? to)
            let first = min(from, end), last = min(to, end)
            let kept = Array(rows.prefix(cardLimit))
            return Card(path: path, from: first, to: last, link: text == nil ? nil : link(path, first, last),   // a deleted file has no link
                        lines: kept, more: rows.count - kept.count)
        }
    }

    /// Cards with just these changed lines, as a skipped group lists them.
    private func cards(listing changes: Set<Change>) -> [Card] {
        cards(for: ranges(covering: changes), only: changes)
    }

    private func link(_ path: String, _ from: Int, _ to: Int) -> String? {
        tour.links?
            .replacing("{head}", with: head)
            .replacing("{path}", with: path.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? path)
            .replacing("{from}", with: String(from))
            .replacing("{to}", with: String(to))
    }
}

/// Ranges that cover these changes, in the tour's own format: a run of added lines takes in the lines removed
/// among them, and lines removed anywhere else get a "del" range (past the end of a file, too).
private func ranges(covering changes: some Sequence<Change>) -> [LineRange] {
    var added: [String: [Int]] = [:], removedBefore: [String: Set<Int>] = [:]
    for change in changes {
        switch change {
        case .added(let path, let line): added[path, default: []].append(line)
        case .removed(let path, let before, _): removedBefore[path, default: []].insert(before)
        }
    }
    return Set(added.keys).union(removedBefore.keys).sorted().flatMap { path in
        let addedRuns = runs(added[path] ?? [])
        let elsewhere = (removedBefore[path] ?? []).filter { before in !addedRuns.contains { $0.contains(before) } }
        return addedRuns.map { LineRange(path: path, from: $0.lowerBound, to: $0.upperBound, part: .both) }
            + elsewhere.sorted().map { LineRange(path: path, from: $0, to: $0, part: .del) }
    }
}

/// Ranges as the agent writes them: ["path", 12, 14] or ["path", 58, 58, "del"].
private func describe(_ ranges: [LineRange]) -> [String] {
    ranges.map { "  [\"\($0.path)\", \($0.from), \($0.to)\($0.part == .del ? ", \"del\"" : "")]" }
}

/// Consecutive numbers as ranges: [3, 4, 5, 9] becomes 3...5 and 9...9.
private func runs(_ numbers: [Int]) -> [ClosedRange<Int>] {
    numbers.sorted().reduce(into: []) { runs, n in
        if let last = runs.last, last.upperBound + 1 == n { runs[runs.count - 1] = last.lowerBound...n } else { runs.append(n...n) }
    }
}
