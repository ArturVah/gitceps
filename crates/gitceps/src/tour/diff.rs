//! What changed between two commits, line by line, from `git diff -U0`.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

use regex::Regex;

use crate::git::text_lines;

/// One changed line: added at a line of the new file, or the n-th of the lines removed just before one.
#[derive(PartialEq, Eq, Hash)]
pub enum Change {
    Added { path: String, line: usize },
    Removed { path: String, before: usize, index: usize },
}

/// What the diff says about one file. Removed lines are kept with the new line they were removed before.
#[derive(Default)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub created: bool,
    pub deleted: bool,
    pub binary: bool,
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
    pub added: BTreeSet<usize>,
    pub removed: BTreeMap<usize, Vec<String>>,
}

impl FileDiff {
    fn submodule(&self) -> bool {
        self.old_mode.as_deref() == Some("160000") || self.new_mode.as_deref() == Some("160000")
    }

    pub fn changes(&self) -> Vec<Change> {
        let added = self.added.iter().map(|&line| Change::Added {
            path: self.path.clone(),
            line,
        });
        let removed = self.removed.iter().flat_map(|(&before, lines)| {
            (0..lines.len()).map(move |index| Change::Removed {
                path: self.path.clone(),
                before,
                index,
            })
        });
        added.chain(removed).collect()
    }

    pub fn removed_count(&self) -> usize {
        self.removed.values().map(Vec::len).sum()
    }

    /// What the tour's file list says about the file besides its line counts.
    pub fn facts(&self) -> Vec<String> {
        let mode = match (&self.old_mode, &self.new_mode) {
            (Some(old), Some(new)) if old != new => Some(format!("mode {old} → {new}")),
            _ => None,
        };
        [
            self.created.then(|| "new".to_string()),
            self.deleted.then(|| "deleted".to_string()),
            self.old_path.as_ref().map(|old| format!("renamed from {old}")),
            self.binary.then(|| "binary".to_string()),
            self.submodule().then(|| "submodule".to_string()),
            mode,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Takes in a second entry for the same path, which git writes when a file changes type (a symlink
    /// becomes a regular file, say): one entry deletes it, the other creates it, and each has one of its modes.
    fn absorb(&mut self, other: FileDiff) {
        let replaced = (self.created || other.created) && (self.deleted || other.deleted);
        self.created = (self.created || other.created) && !replaced;
        self.deleted = (self.deleted || other.deleted) && !replaced;
        self.binary = self.binary || other.binary;
        self.old_path = self.old_path.take().or(other.old_path);
        self.old_mode = self.old_mode.take().or(other.old_mode);
        self.new_mode = self.new_mode.take().or(other.new_mode);
        self.added.extend(other.added);
        for (before, lines) in other.removed {
            self.removed.entry(before).or_default().extend(lines);
        }
    }
}

static HUNK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@").expect("the pattern is valid"));

/// Reads `git diff -U0 -M` with the a/ and b/ prefixes. Paths git quotes, those with a quote, a backslash or
/// a control character in them, aren't read.
pub fn parse_diff(text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut before = 0; // the new line the current hunk's removed lines sit before
    let mut in_hunk = false;

    for line in text_lines(text) {
        if let Some(pair) = line.strip_prefix("diff --git ") {
            // "a/P b/P": without a rename both halves are the same path, spaces and all. A renamed file gets
            // its path from "rename to".
            let path = pair.get(2..2 + pair.len().saturating_sub(5) / 2).unwrap_or_default();
            files.push(FileDiff {
                path: path.to_string(),
                ..FileDiff::default()
            });
            in_hunk = false;
            continue;
        }
        let Some(file) = files.last_mut() else { continue };
        if let Some(hunk) = HUNK.captures(&line) {
            let start: usize = hunk[1].parse().unwrap_or(0);
            let count: usize = hunk.get(2).map_or(1, |count| count.as_str().parse().unwrap_or(0));
            file.added.extend(start..start + count);
            before = if count > 0 { start } else { start + 1 };
            in_hunk = true;
        } else if in_hunk {
            if let Some(removed) = line.strip_prefix('-') {
                file.removed.entry(before).or_default().push(removed.to_string());
            }
        } else if let Some(path) = line.strip_prefix("rename from ") {
            file.old_path = Some(path.to_string());
        } else if let Some(path) = line.strip_prefix("rename to ") {
            file.path = path.to_string();
        } else if let Some(mode) = line.strip_prefix("new file mode ") {
            file.created = true;
            file.new_mode = Some(mode.to_string());
        } else if let Some(mode) = line.strip_prefix("deleted file mode ") {
            file.deleted = true;
            file.old_mode = Some(mode.to_string());
        } else if let Some(mode) = line.strip_prefix("old mode ") {
            file.old_mode = Some(mode.to_string());
        } else if let Some(mode) = line.strip_prefix("new mode ") {
            file.new_mode = Some(mode.to_string());
        } else if line.starts_with("index ") {
            // "index <old>..<new> <mode>", where the mode is there only when it didn't change.
            if let [_, _, mode] = line.split(' ').filter(|part| !part.is_empty()).collect::<Vec<_>>()[..] {
                file.old_mode = Some(mode.to_string());
                file.new_mode = Some(mode.to_string());
            }
        } else if line.starts_with("Binary files ") {
            file.binary = true;
        }
    }

    let mut merged: Vec<FileDiff> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for mut file in files {
        // A submodule's diff is only the commit it points at, which isn't a line of code. Cleared before
        // merging, so a text file that became a submodule (or the reverse) keeps its own lines.
        if file.submodule() {
            file.added.clear();
            file.removed.clear();
        }
        match at.get(&file.path) {
            Some(&index) => merged[index].absorb(file),
            None => {
                at.insert(file.path.clone(), merged.len());
                merged.push(file);
            }
        }
    }
    merged
}
