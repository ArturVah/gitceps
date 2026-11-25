//! Checks a tour file against the real diff between its base and head, and builds the page from it.
//!
//! Every changed line must be shown in a step or listed under "skipped" with a reason. Only blank lines are
//! listed without the tour placing them. Judging what's routine is the agent's job: no pattern can tell a
//! comment from a build directive (//go:embed) or an attribute (#[IsGranted]) that only looks like one.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use unicode_segmentation::UnicodeSegmentation;

use super::diff::{Change, FileDiff, parse_diff};
use super::file::{self, LineRange, Part, TourFile};
use super::page::{Card, FileStat, Group, Line, Note, Page, Step, blank, dedented};
use crate::git::{files, git};

/// The reactions a step may name: REVIEW in overlay/scenes.js, without "finale", which the page plays itself.
const REACTIONS: [&str; 7] = ["intro", "nod", "inspect", "guard", "alarm", "nitpick", "unsure"];
const BUBBLE_LIMIT: usize = 28; // characters that fit his speech bubble
const STEP_LIMIT: usize = 7;
const CARD_LIMIT: usize = 300; // rows a card carries; for more, its link opens the file

pub struct TourCheck {
    tour: TourFile,
    repo: PathBuf,
    head: String,
    diffs: Vec<FileDiff>,
    by_path: HashMap<String, usize>,
    contents: HashMap<String, Option<Vec<String>>>,
    problems: Vec<String>,
}

impl TourCheck {
    pub fn new(file: &Path) -> Result<TourCheck> {
        let text = std::fs::read(file).with_context(|| format!("can't read {}", file.display()))?;
        // The error names the field it's about: "steps[2]: missing field `why` at line 40 column 5".
        let tour: TourFile = serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_slice(&text))
            .map_err(|error| anyhow!("{}: {error}", file.display()))?;
        // Joined as written: the system resolves "..", after following any symlinked folder on the way.
        let repo = std::path::absolute(file)?
            .parent()
            .unwrap_or(Path::new("/"))
            .join(&tour.repo);
        let resolve = |rev: &str, kind: &str| -> Result<String> {
            Ok(git(&["rev-parse", "--verify", &format!("{rev}^{{{kind}}}")], &repo)?
                .trim()
                .to_string())
        };
        let head = resolve(&tour.head, "commit")?;
        let base = resolve(&tour.base, "tree")?; // a tree, so a root commit can use the empty tree
        // The format is pinned, so the user's own diff settings (noprefix, interHunkContext, …) can't change it.
        let diffs = parse_diff(&git(
            &[
                "diff",
                "-U0",
                "-M",
                "--no-color",
                "--no-ext-diff",
                "--no-textconv",
                "--no-relative",
                "--src-prefix=a/",
                "--dst-prefix=b/",
                "--inter-hunk-context=0",
                "--submodule=short",
                &base,
                &head,
                "--",
            ],
            &repo,
        )?);
        // Git has decided which changed files are text. The others, and deleted files, have no lines in head.
        let readable: Vec<String> = diffs
            .iter()
            .filter(|diff| !diff.deleted && !diff.binary)
            .map(|diff| diff.path.clone())
            .collect();
        let mut read = files(&readable, &head, &repo)?;
        let contents = diffs
            .iter()
            .map(|diff| (diff.path.clone(), read.remove(&diff.path)))
            .collect();
        let by_path = diffs
            .iter()
            .enumerate()
            .map(|(index, diff)| (diff.path.clone(), index))
            .collect();
        Ok(TourCheck {
            tour,
            repo,
            head,
            diffs,
            by_path,
            contents,
            problems: Vec::new(),
        })
    }

    /// The page, or every problem found, so the agent can fix them all in one go.
    pub fn page(mut self) -> Result<Page> {
        let step_count = self.tour.steps.len();
        if !(1..=STEP_LIMIT).contains(&step_count) {
            self.problems.push(format!(
                "A tour has 1 to {STEP_LIMIT} steps; this one has {step_count}. Split a bigger change into several tours."
            ));
        }

        let mut shown: HashSet<Change> = HashSet::new();
        let mut steps = Vec::new();
        for (index, step) in std::mem::take(&mut self.tour.steps).into_iter().enumerate() {
            let place = format!("step {}", index + 1);
            let ranges = self.valid_ranges(&step.ranges, &place);
            let found = self.changes_in(&ranges);
            let covers = found.difference(&shown).count();
            let cards = self.cards(&ranges, None);
            steps.push(self.checked_step(step, &place, covers, cards));
            shown.extend(found);
        }

        let mut listed: HashSet<Change> = HashSet::new();
        let mut groups = Vec::new();
        for (index, skip) in self.tour.skipped.take().unwrap_or_default().into_iter().enumerate() {
            let ranges = self.valid_ranges(&skip.ranges, &format!("skipped group {}", index + 1));
            let found: HashSet<Change> = self
                .changes_in(&ranges)
                .into_iter()
                .filter(|change| !shown.contains(change) && !listed.contains(change))
                .collect();
            if found.is_empty() {
                continue; // nothing in it that isn't shown or listed already
            }
            groups.push(Group {
                reason: skip.reason,
                count: found.len(),
                cards: self.cards_listing(&found),
            });
            listed.extend(found);
        }

        let left: Vec<Change> = self
            .diffs
            .iter()
            .flat_map(FileDiff::changes)
            .filter(|change| !shown.contains(change) && !listed.contains(change))
            .collect();
        let (blank, unplaced): (HashSet<Change>, HashSet<Change>) =
            left.into_iter().partition(|change| self.is_blank(change));
        if !blank.is_empty() {
            groups.push(Group {
                reason: "Blank lines".to_string(),
                count: blank.len(),
                cards: self.cards_listing(&blank),
            });
            listed.extend(blank);
        }
        if !unplaced.is_empty() {
            self.problems.push(
                "These changed lines aren't in the tour. Show them in a step, or list them under \"skipped\" with a reason. As ranges:"
                    .to_string(),
            );
            self.problems.extend(describe(&ranges_covering(&unplaced)));
        }
        if !self.problems.is_empty() {
            bail!("{}", self.problems.join("\n"));
        }

        let overview = self
            .diffs
            .iter()
            .map(|diff| FileStat {
                path: diff.path.clone(),
                ins: diff.added.len(),
                del: diff.removed_count(),
                facts: diff.facts(),
            })
            .collect();
        Ok(Page {
            number: self.tour.number,
            title: self.tour.title,
            summary: self.tour.summary,
            flag: self.tour.flag,
            overview,
            shown_lines: shown.len(),
            skipped_lines: listed.len(),
            steps,
            skipped: groups,
        })
    }

    // Steps

    fn checked_step(&mut self, step: file::Step, place: &str, covers: usize, cards: Vec<Card>) -> Step {
        for (name, text) in [("title", &step.title), ("why", &step.why), ("bro", &step.bro)] {
            if text.trim().is_empty() {
                self.problems.push(format!("{place}: {name} is empty"));
            }
        }
        if !REACTIONS.contains(&step.react.as_str()) {
            self.problems.push(format!(
                "{place}: react \"{}\" isn't one of {}",
                step.react,
                REACTIONS.join(", ")
            ));
        }
        for bubble in &step.bubbles {
            let length = bubble.graphemes(true).count();
            if length > BUBBLE_LIMIT {
                self.problems.push(format!(
                    "{place}: \"{bubble}\" is {length} characters; a bubble fits {BUBBLE_LIMIT}"
                ));
            }
        }
        let note = step.note.map(|note| self.checked_note(note, place));
        Step {
            title: step.title,
            why: step.why,
            bro: step.bro,
            react: step.react,
            bubbles: step.bubbles,
            note,
            covers,
            cards,
        }
    }

    fn checked_note(&mut self, note: file::Note, place: &str) -> Note {
        if !["good", "warn"].contains(&note.kind.as_str()) {
            self.problems
                .push(format!("{place}: note kind \"{}\" isn't good or warn", note.kind));
        }
        let mut link = None;
        if let Some(source) = &note.source {
            // "path:line", where the path may itself contain colons
            let line = source
                .rsplit_once(':')
                .and_then(|(path, line)| Some((path, line.parse::<usize>().ok()?)));
            match line {
                Some((path, line)) if line >= 1 && self.lines_of(path).is_some_and(|text| line <= text.len()) => {
                    link = self.link(path, line, line);
                }
                _ => self.problems.push(format!(
                    "{place}: note source \"{source}\" isn't path:line of a line in head"
                )),
            }
        }
        Note {
            kind: note.kind,
            text: note.text,
            source: note.source,
            link,
        }
    }

    // Lines

    fn diff(&self, path: &str) -> Option<&FileDiff> {
        self.by_path.get(path).map(|&index| &self.diffs[index])
    }

    /// A file's lines in head. Changed files were read up front, as text wherever git diffs them as text. A file
    /// outside the diff, which a step shows for context, is read the first time it's asked for, and has no
    /// lines if it holds a NUL byte, as a binary file does.
    fn lines_of(&mut self, path: &str) -> Option<&Vec<String>> {
        if !self.contents.contains_key(path) {
            let found = files(&[path.to_string()], &self.head, &self.repo)
                .ok()
                .and_then(|mut found| found.remove(path))
                .filter(|lines| !lines.iter().any(|line| line.contains('\0')));
            self.contents.insert(path.to_string(), found);
        }
        self.loaded(path)
    }

    /// A file's lines in head, if `lines_of` has read them already.
    fn loaded(&self, path: &str) -> Option<&Vec<String>> {
        self.contents.get(path).and_then(Option::as_ref)
    }

    /// The ranges that name real lines, as inclusive line numbers. The others become problems.
    fn valid_ranges(&mut self, ranges: &[LineRange], place: &str) -> Vec<Lines> {
        let mut valid = Vec::new();
        for range in ranges {
            let LineRange { path, from, to, part } = range;
            if *from < 1 || from > to {
                self.problems
                    .push(format!("{place}: {path} {from}-{to} isn't a range of lines"));
                continue;
            }
            let span = *from as usize..=*to as usize;
            if *part == Part::Del {
                let problem = match self.diff(path).map(|diff| &diff.removed) {
                    Some(removed) if removed.range(span.clone()).next().is_some() => None,
                    Some(removed) if !removed.is_empty() => {
                        let at = removed.keys().map(usize::to_string).collect::<Vec<_>>().join(", ");
                        Some(format!(
                            "{place}: no lines were removed at {path} {from}-{to}; they were removed before lines {at}"
                        ))
                    }
                    _ => Some(format!("{place}: no lines were removed from {path}")),
                };
                if let Some(problem) = problem {
                    self.problems.push(problem);
                    continue;
                }
            } else {
                let Some(count) = self.lines_of(path).map(Vec::len) else {
                    self.problems.push(format!("{place}: {path} isn't a text file in head"));
                    continue;
                };
                if *to as usize > count {
                    self.problems.push(format!(
                        "{place}: {path} has {count} lines, so {from}-{to} runs past its end"
                    ));
                    continue;
                }
            }
            valid.push(Lines {
                path: path.clone(),
                span,
                part: *part,
            });
        }
        valid
    }

    fn changes_in(&self, ranges: &[Lines]) -> HashSet<Change> {
        let mut found = HashSet::new();
        for range in ranges {
            let Some(diff) = self.diff(&range.path) else { continue };
            if range.part != Part::Del {
                for &line in diff.added.range(range.span.clone()) {
                    found.insert(Change::Added {
                        path: range.path.clone(),
                        line,
                    });
                }
            }
            if range.part != Part::Nodel {
                for (&before, removed) in diff.removed.range(range.span.clone()) {
                    for index in 0..removed.len() {
                        found.insert(Change::Removed {
                            path: range.path.clone(),
                            before,
                            index,
                        });
                    }
                }
            }
        }
        found
    }

    fn is_blank(&self, change: &Change) -> bool {
        let text = match change {
            Change::Added { path, line } => self.loaded(path).and_then(|lines| lines.get(line - 1)),
            Change::Removed { path, before, index } => self
                .diff(path)
                .and_then(|diff| diff.removed.get(before))
                .and_then(|lines| lines.get(*index)),
        };
        text.is_some_and(|text| blank(text))
    }

    // Cards

    /// One card per file, in line order, with a gap row between pieces that don't touch. A removed line sits
    /// just before the new line it was removed before. With `only`, other changed lines are left out.
    fn cards(&self, ranges: &[Lines], only: Option<&HashSet<Change>>) -> Vec<Card> {
        let mut paths: Vec<&str> = Vec::new();
        let mut new_lines: HashMap<&str, BTreeSet<usize>> = HashMap::new();
        let mut removed_before: HashMap<&str, BTreeSet<usize>> = HashMap::new();
        for range in ranges {
            let path = range.path.as_str();
            if !new_lines.contains_key(path) && !removed_before.contains_key(path) {
                paths.push(path);
            }
            if range.part != Part::Del {
                new_lines.entry(path).or_default().extend(range.span.clone());
            }
            if range.part != Part::Nodel
                && let Some(diff) = self.diff(path)
            {
                removed_before
                    .entry(path)
                    .or_default()
                    .extend(diff.removed.range(range.span.clone()).map(|(&before, _)| before));
            }
        }

        let wanted = |change: Change| only.is_none_or(|only| only.contains(&change));
        let mut cards = Vec::new();
        for path in paths {
            let text = self.loaded(path);
            let diff = self.diff(path);
            let shown_new = new_lines.remove(path).unwrap_or_default();
            let shown_removed = removed_before.remove(path).unwrap_or_default();
            let positions: Vec<usize> = shown_new.union(&shown_removed).copied().collect();
            let (Some(&from), Some(&to)) = (positions.first(), positions.last()) else {
                continue;
            };

            let mut rows: Vec<Line> = Vec::new();
            let mut piece: Vec<Line> = Vec::new();
            for (index, &number) in positions.iter().enumerate() {
                if index > 0 && number > positions[index - 1] + 1 {
                    rows.extend(dedented(std::mem::take(&mut piece)));
                    rows.push(Line::gap());
                }
                if shown_removed.contains(&number) {
                    let removed = diff
                        .and_then(|diff| diff.removed.get(&number))
                        .map(Vec::as_slice)
                        .unwrap_or_default();
                    for (index, removed) in removed.iter().enumerate() {
                        if wanted(Change::Removed {
                            path: path.to_string(),
                            before: number,
                            index,
                        }) {
                            piece.push(Line {
                                number: None,
                                kind: "-",
                                text: removed.clone(),
                            });
                        }
                    }
                }
                if shown_new.contains(&number)
                    && let Some(text) = text
                    && wanted(Change::Added {
                        path: path.to_string(),
                        line: number,
                    })
                {
                    let added = diff.is_some_and(|diff| diff.added.contains(&number));
                    piece.push(Line {
                        number: Some(number),
                        kind: if added { "+" } else { " " },
                        text: text[number - 1].clone(),
                    });
                }
            }
            rows.extend(dedented(piece));

            // Lines removed at the end of a file sit one past its last line, which isn't a line to name or link to.
            let end = text.map_or(to, Vec::len).max(1);
            let (first, last) = (from.min(end), to.min(end));
            let more = rows.len().saturating_sub(CARD_LIMIT);
            rows.truncate(CARD_LIMIT);
            // a deleted file has no link
            let link = text.and_then(|_| self.link(path, first, last));
            cards.push(Card {
                path: path.to_string(),
                from: first,
                to: last,
                link,
                lines: rows,
                more,
            });
        }
        cards
    }

    /// Cards with just these changed lines, as a skipped group lists them.
    fn cards_listing(&self, changes: &HashSet<Change>) -> Vec<Card> {
        let ranges: Vec<Lines> = ranges_covering(changes);
        self.cards(&ranges, Some(changes))
    }

    fn link(&self, path: &str, from: usize, to: usize) -> Option<String> {
        let links = self.tour.links.as_ref()?;
        Some(
            links
                .replace("{head}", &self.head)
                .replace("{path}", &percent_encoded(path))
                .replace("{from}", &from.to_string())
                .replace("{to}", &to.to_string()),
        )
    }
}

/// A valid range: lines `span` of `path`, inclusive.
struct Lines {
    path: String,
    span: RangeInclusive<usize>,
    part: Part,
}

/// Ranges that cover these changes, in the tour's own format: a run of added lines takes in the lines removed
/// among them, and lines removed anywhere else get a "del" range (past the end of a file, too).
fn ranges_covering(changes: &HashSet<Change>) -> Vec<Lines> {
    let mut added: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
    let mut removed_before: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
    for change in changes {
        match change {
            Change::Added { path, line } => added.entry(path).or_default().insert(*line),
            Change::Removed { path, before, .. } => removed_before.entry(path).or_default().insert(*before),
        };
    }
    let paths: BTreeSet<&str> = added.keys().chain(removed_before.keys()).copied().collect();
    let mut ranges = Vec::new();
    for path in paths {
        let added_runs = runs(added.get(path).into_iter().flatten().copied());
        let elsewhere = removed_before
            .get(path)
            .into_iter()
            .flatten()
            .filter(|before| !added_runs.iter().any(|run| run.contains(before)));
        let dels: Vec<Lines> = elsewhere
            .map(|&before| Lines {
                path: path.to_string(),
                span: before..=before,
                part: Part::Del,
            })
            .collect();
        ranges.extend(added_runs.iter().map(|run| Lines {
            path: path.to_string(),
            span: run.clone(),
            part: Part::Both,
        }));
        ranges.extend(dels);
    }
    ranges
}

/// Ranges as the agent writes them: ["path", 12, 14] or ["path", 58, 58, "del"].
fn describe(ranges: &[Lines]) -> Vec<String> {
    ranges
        .iter()
        .map(|range| {
            let del = if range.part == Part::Del { ", \"del\"" } else { "" };
            format!(
                "  [\"{}\", {}, {}{del}]",
                range.path,
                range.span.start(),
                range.span.end()
            )
        })
        .collect()
}

/// Consecutive numbers as ranges: [3, 4, 5, 9] becomes 3..=5 and 9..=9.
fn runs(sorted: impl Iterator<Item = usize>) -> Vec<RangeInclusive<usize>> {
    let mut runs: Vec<RangeInclusive<usize>> = Vec::new();
    for number in sorted {
        match runs.last_mut() {
            Some(last) if last.end() + 1 == number => *last = *last.start()..=number,
            _ => runs.push(number..=number),
        }
    }
    runs
}

/// A path as it goes into a link: everything but letters, digits and the characters a URL path may hold
/// becomes %XX.
fn percent_encoded(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"!$&'()*+,-./:;=@_~".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
