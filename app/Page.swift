// What tour/tour.html shows: the checked tour, with the lines of code each step and skipped group points at.
import Foundation

struct Page: Encodable {
    struct Step: Encodable {
        let title, why, bro, react: String
        let bubbles: [String]
        let note: Note?
        let covers: Int   // changed lines this step is the first to show
        let cards: [Card]
    }
    struct Note: Encodable {
        let kind, text: String
        let source, link: String?
    }
    struct Group: Encodable {
        let reason: String
        let count: Int
        let cards: [Card]
    }
    struct FileStat: Encodable {
        let path: String
        let ins, del: Int
        let facts: [String]
    }

    let number: Int?
    let title, summary: String
    let flag: String?
    let overview: [FileStat]
    let shownLines, skippedLines: Int
    let steps: [Step]
    let skipped: [Group]
}

/// Lines of one file, in line order. `more` counts rows left out of a long card.
struct Card: Encodable {
    let path: String
    let from, to: Int
    let link: String?
    let lines: [Line]
    let more: Int
}

/// One row of a card, sent as [number, kind, text]. Kind is "+" added, "-" removed, " " unchanged, or "gap"
/// for lines left out between two pieces of the same file.
struct Line: Encodable {
    let number: Int?
    let kind: String
    var text: String

    static let gap = Line(number: nil, kind: "gap", text: "")

    func encode(to encoder: any Encoder) throws {
        var row = encoder.unkeyedContainer()
        try row.encode(number)
        try row.encode(kind)
        try row.encode(text)
    }
}

/// Drops the indentation every line of a piece shares, so deep nesting doesn't push the code off the card.
func dedented(_ piece: [Line]) -> [Line] {
    let indent = { (line: Line) in line.text.prefix { $0 == " " }.count }
    guard let cut = piece.filter({ !$0.text.trimmingCharacters(in: .whitespaces).isEmpty }).map(indent).min(), cut > 0 else {
        return piece
    }
    return piece.map { line in
        var line = line
        line.text = String(line.text.dropFirst(min(cut, indent(line))))
        return line
    }
}
