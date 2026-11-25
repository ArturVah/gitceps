//! What tour/tour.html shows: the checked tour, with the lines of code each step and skipped group points at.
use serde::ser::{Serialize, SerializeSeq, Serializer};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<i64>,
    pub title: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    pub overview: Vec<FileStat>,
    pub shown_lines: usize,
    pub skipped_lines: usize,
    pub steps: Vec<Step>,
    pub skipped: Vec<Group>,
}

#[derive(serde::Serialize)]
pub struct Step {
    pub title: String,
    pub why: String,
    pub bro: String,
    pub react: String,
    pub bubbles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Note>,
    pub covers: usize, // changed lines this step is the first to show
    pub cards: Vec<Card>,
}

#[derive(serde::Serialize)]
pub struct Note {
    pub kind: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(serde::Serialize)]
pub struct Group {
    pub reason: String,
    pub count: usize,
    pub cards: Vec<Card>,
}

#[derive(serde::Serialize)]
pub struct FileStat {
    pub path: String,
    pub ins: usize,
    pub del: usize,
    pub facts: Vec<String>,
}

/// Lines of one file, in line order. `more` counts rows left out of a long card.
#[derive(serde::Serialize)]
pub struct Card {
    pub path: String,
    pub from: usize,
    pub to: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    pub lines: Vec<Line>,
    pub more: usize,
}

/// One row of a card, sent as [number, kind, text]. Kind is "+" added, "-" removed, " " unchanged, or "gap"
/// for lines left out between two pieces of the same file.
pub struct Line {
    pub number: Option<usize>,
    pub kind: &'static str,
    pub text: String,
}

impl Line {
    pub fn gap() -> Line {
        Line {
            number: None,
            kind: "gap",
            text: String::new(),
        }
    }
}

impl Serialize for Line {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_seq(Some(3))?;
        row.serialize_element(&self.number)?;
        row.serialize_element(self.kind)?;
        row.serialize_element(&self.text)?;
        row.end()
    }
}

/// Drops the indentation every line of a piece shares, so deep nesting doesn't push the code off the card.
pub fn dedented(mut piece: Vec<Line>) -> Vec<Line> {
    let indent = |line: &Line| line.text.bytes().take_while(|&byte| byte == b' ').count();
    let cut = piece
        .iter()
        .filter(|line| !blank(&line.text))
        .map(indent)
        .min()
        .unwrap_or(0);
    for line in &mut piece {
        let drop = cut.min(indent(line));
        line.text.drain(..drop);
    }
    piece
}

/// Whether a line holds nothing but spaces and tabs, counting every kind of space Unicode lists. A form feed or
/// a line separator is content.
pub fn blank(text: &str) -> bool {
    text.chars()
        .all(|c| c == '\t' || (c.is_whitespace() && !c.is_control() && !matches!(c, '\u{2028}' | '\u{2029}')))
}
