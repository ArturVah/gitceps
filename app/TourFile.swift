// A tour of a PR or a commit, as the gitceps-tour skill writes it (skills/gitceps-tour/SKILL.md).
import Foundation

struct TourFile: Decodable {
    struct Step: Decodable {
        let title, why, bro, react: String
        let bubbles: [String]
        let note: Note?
        let ranges: [LineRange]
    }
    struct Note: Decodable {
        let kind: String   // "good" or "warn"
        let text: String
        let source: String?   // "path:line"
    }
    struct Skip: Decodable {
        let reason: String
        let ranges: [LineRange]
    }

    let repo, base, head, title, summary: String
    let number: Int?
    let links, flag: String?
    let steps: [Step]
    let skipped: [Skip]?
}

/// Lines of one file as of head: ["path", from, to], optionally followed by "nodel" (leave out the lines
/// removed there) or "del" (only the lines removed there).
struct LineRange {
    enum Part: String, Decodable { case both, nodel, del }
    let path: String
    let from: Int
    let to: Int
    let part: Part
}

extension LineRange: Decodable {
    init(from decoder: any Decoder) throws {
        var list = try decoder.unkeyedContainer()
        path = try list.decode(String.self)
        from = try list.decode(Int.self)
        to = try list.decode(Int.self)
        part = list.isAtEnd ? .both : try list.decode(Part.self)
    }
}

/// "steps[2].why is missing" instead of Swift's full description of a decoding error.
func explain(_ error: DecodingError) -> String {
    let at = { (path: [any CodingKey]) in
        String(path.map { $0.intValue.map { "[\($0)]" } ?? ".\($0.stringValue)" }.joined().drop { $0 == "." })
    }
    switch error {
    case .keyNotFound(let key, let context): return "\(at(context.codingPath + [key])) is missing"
    case .dataCorrupted(let context) where context.codingPath.isEmpty:   // not JSON: where it stops being JSON
        return (context.underlyingError as NSError?)?.userInfo[NSDebugDescriptionErrorKey] as? String ?? context.debugDescription
    case .typeMismatch(_, let context), .valueNotFound(_, let context), .dataCorrupted(let context):
        return "\(at(context.codingPath)): \(context.debugDescription)"
    @unknown default: return "\(error)"
    }
}
