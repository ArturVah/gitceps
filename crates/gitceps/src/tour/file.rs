//! A tour of a PR or a commit, as the gitceps-tour skill writes it (skills/gitceps-tour/SKILL.md).
use serde::Deserialize;
use serde::de::{Deserializer, Error, SeqAccess, Visitor};

#[derive(Deserialize)]
pub struct TourFile {
    pub repo: String,
    pub base: String,
    pub head: String,
    pub title: String,
    pub summary: String,
    pub number: Option<i64>,
    pub links: Option<String>,
    pub flag: Option<String>,
    pub steps: Vec<Step>,
    pub skipped: Option<Vec<Skip>>,
}

#[derive(Deserialize)]
pub struct Step {
    pub title: String,
    pub why: String,
    pub bro: String,
    pub react: String,
    pub bubbles: Vec<String>,
    pub note: Option<Note>,
    pub ranges: Vec<LineRange>,
}

#[derive(Deserialize)]
pub struct Note {
    pub kind: String, // "good" or "warn"
    pub text: String,
    pub source: Option<String>, // "path:line"
}

#[derive(Deserialize)]
pub struct Skip {
    pub reason: String,
    pub ranges: Vec<LineRange>,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Part {
    Both,
    Nodel,
    Del,
}

/// Lines of one file as of head: ["path", from, to], optionally followed by "nodel" (leave out the lines
/// removed there) or "del" (only the lines removed there).
pub struct LineRange {
    pub path: String,
    pub from: i64,
    pub to: i64,
    pub part: Part,
}

impl<'de> Deserialize<'de> for LineRange {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Reader;
        impl<'de> Visitor<'de> for Reader {
            type Value = LineRange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(r#"a range like ["path", 12, 14] or ["path", 58, 58, "del"]"#)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut list: A) -> Result<LineRange, A::Error> {
                let path = list.next_element()?.ok_or_else(|| Error::invalid_length(0, &self))?;
                let from = list.next_element()?.ok_or_else(|| Error::invalid_length(1, &self))?;
                let to = list.next_element()?.ok_or_else(|| Error::invalid_length(2, &self))?;
                let part = list.next_element()?.unwrap_or(Part::Both);
                Ok(LineRange { path, from, to, part })
            }
        }
        deserializer.deserialize_seq(Reader)
    }
}
