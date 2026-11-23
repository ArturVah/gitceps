---
name: gitceps-tour
description: Shows a pull request, a commit or a range of commits as a step-by-step tour in the Gitceps window, with Push Bro reacting to each step. Use when the user asks for a tour or a walkthrough of a PR, a commit or a range of commits, for example "tour PR #18", "walk me through my last commit" or "Push Bro, tour abc123". Not for a plain "review this PR" request, which the agent's usual code review handles.
---

# Gitceps tour

You review a change and write a tour of it: a few steps in the order the code runs, each showing only the
lines that matter, with a plain explanation and Push Bro's take. Gitceps checks the tour against the real
diff and shows it in a floating panel on the Mac. The person reviewing reads every step, clicks "Got It" to
move on, and decides what to do. Your job is to make that review fast and correct, never to replace it.

Do the review yourself, as described below. Don't hand it to another review command, skill or agent: the
tour needs your own reading of the code, and a separate review would cost the user time and tokens twice.

Run Gitceps with the `gitceps` in this skill's folder, a link to the Gitceps this skill belongs to. Before
anything else, run `<this skill's folder>/gitceps status`. It prints `on` or `off`, and either is fine. If it
fails, this copy of the skill has no Gitceps behind it: it was copied on its own, or Gitceps was removed. Tell
the user to install Gitceps from https://github.com/ArturVah/gitceps, which adds this skill by itself, and
stop there, without looking for another copy of Gitceps.

## 1. Find base and head

Work in the repo's local clone, so you can read any file, not only the diff.

- **GitHub PR:** `gh pr view <number> --json number,title,url,baseRefName,headRefOid`, then
  `git fetch origin <baseRefName> pull/<number>/head`. Head is `headRefOid`, and base is
  `git merge-base origin/<baseRefName> <headRefOid>`.
- **Bitbucket or another host:** use the user's tools for that host if they have them, otherwise ask for the
  source and target branches. Fetch both. Head is the source branch's commit, and base is
  `git merge-base origin/<target> origin/<source>`.
- **A commit:** head is the commit and base is its first parent (`<sha>^`). A root commit has no parent: use
  the empty tree, `4b825dc642cb6eb9a060e54bf8d69288fbee4904`.
- **Several commits or a branch:** base is where they start (`HEAD~3`, or `git merge-base main <branch>`),
  and head is the last one.

Use full commit hashes in the tour file.

**A tour that already exists is shown again, not redone.** If
`$TMPDIR/gitceps-tours/<repo>-<PR number or short hash>.json` exists and its `base` and `head` are
the hashes you just found, skip the review and show that file (step 6). A commit hash covers
every line of the commit, so equal hashes mean the same code. Review again when either hash has changed (new
commits, a force-push, a new target branch), or when the user asks for a fresh tour.

## 2. Review the change

1. **Learn the intent** from the PR description, the commit messages, a linked ticket, and the docs the
   repo keeps (AGENTS.md, ADRs, README). The tour's "why" comes from these, not from guessing.
2. **Map the change.** Run `git diff --stat <base> <head>`, then read the full diff. For each changed function,
   read the code around it: its callers, what it calls, schemas, migrations and config.
3. **Find the flow.** Start where a request or event enters (a route, a handler, a CLI command, a job, a UI
   event) and follow it through the changed code. Related changes become one step, and the steps go in the
   order the code runs, not the order of the files.
4. **Review each step:**
   - Does it do what the PR or commit says? Check edge cases: empty or missing input, errors, wrong or
     inverted conditions, off-by-one.
   - Security: who can reach it, ownership and permission checks, injection, secrets or personal data in logs.
   - Data: migrations, uniqueness, transactions, and old callers of an API or contract that changed.
   - Errors: the right status or code, nothing swallowed.
   - Tests: which behavior each test covers, and which behavior has none.
5. **Prove every finding.** Point each finding or guarantee at the line that shows it (`path:line`), and check
   it by reading that code or running a quick command, such as grepping a migration or running one test. If
   you can't confirm a step, give it the `unsure` reaction. Push Bro then tells the reviewer to check it
   themselves instead of joking about it.

## 3. Choose what to show

Show the lines a reviewer must read: logic, conditions, queries, permission checks, data writes, public
contracts, error handling, and anything risky. Show 2 to 20 lines a step. A step may also show unchanged
lines, or lines from another file, when they prove a point (the migration that makes a column unique).

Skip routine lines, and give each group of them a reason: imports, comments that say what the code already
says, version bumps, lockfiles and generated files, wrappers and boilerplate, formatting, test bodies (show
the test names instead), and docs that repeat the tour. Judge each line yourself. A line that looks like a
comment can be code: a build directive (`//go:embed`, `//go:build`), an attribute (`#[IsGranted]`), a type
checker switch (`// @ts-nocheck`). Show those when they matter.

Every changed line must be either shown in a step or listed as skipped. Gitceps lists blank lines by itself
and refuses a tour that leaves anything else out, telling you exactly which lines. The lines of a deleted file
are `["path", 1, 1, "del"]`. New, deleted, renamed and binary files, mode changes and submodules appear in
the tour's file list by themselves; say so in a step when one matters (a script that became executable, a
file that held a security check).

Use at most 7 steps. A change that needs more is really several changes: write one tour for each part.

## 4. Write the tour file

Save it as `$TMPDIR/gitceps-tours/<repo>-<PR number or short hash>.json`, creating the folder if needed. The
temporary folder is one that coding agents' sandboxes let them write to.

```json
{
  "repo": "/Users/me/code/app",
  "base": "<full hash>",
  "head": "<full hash>",
  "number": 18,
  "title": "Add password reset by email",
  "links": "https://github.com/acme/app/blob/{head}/{path}#L{from}-L{to}",
  "steps": [
    {
      "title": "Check the token",
      "why": "When the link is opened, the token is looked up and rejected if it's older than an hour. Then the new password is set.",
      "bro": "Token under an hour old? Good form. But you never rack it: same link, two reps. Nope.",
      "react": "alarm",
      "bubbles": ["under an hour? ok…", "NEVER RACKED?!", "same link. two reps."],
      "note": { "kind": "warn", "text": "The token is never deleted, so the same link works twice.", "source": "src/reset.ts:42" },
      "ranges": [["src/reset.ts", 38, 43]]
    }
  ],
  "skipped": [
    { "reason": "Test bodies (the test names are in step 6)", "ranges": [["test/reset.test.ts", 12, 60]] }
  ],
  "summary": "Two or three sentences: what the change does, and whether it holds up.",
  "flag": "What to fix before merging, if anything. Leave it out when there's nothing."
}
```

- `ranges`: `["path", from, to]` in the file as of head. It shows those lines together with the lines removed
  just before them. Add `"nodel"` to leave the removed lines out, or `"del"` to show only the removed lines
  (`["path", 58, 58, "del"]` for lines removed before line 58, or before where the file ends).
- `links`: the host's link to a line range, with `{head}`, `{path}`, `{from}` and `{to}`. Bitbucket:
  `https://bitbucket.org/<workspace>/<repo>/src/{head}/{path}#lines-{from}:{to}`. Leave it out for a repo
  with no remote.
- `number`: the PR number. Leave it out for commits.
- `note`: one finding for the step. `good` for a check that holds, `warn` for a problem. `source` is the line
  that proves it, as `path:line`.
- `bro`: Push Bro's take on the step, shown in quotes under it. One or two short sentences.
- The first step usually has `"ranges": []`. It says what the change is for, and the window shows the list
  of changed files.

## 5. Push Bro

Push Bro is a gym bro who reviews code like a spotter. Gym talk is welcome: reps, form, spotting, racking the
bar, "PR" as a personal record. His line must carry the same facts as `why`, and a joke never hides a
problem. He's loudest at the most important finding. Joke about the code, never about the person who wrote it.

`react` picks what he does, and `bubbles` are the short lines in his speech bubble, in order, 28 characters
at most each:

| react | when | bubbles |
|---|---|---|
| `intro` | the first step | 2: an opener, then a one-liner about the change |
| `nod` | a plain step | 2 |
| `inspect` | a step worth a close look that holds up | 2: while he reads, then his verdict |
| `guard` | a check that refuses what it should | 4: while he reads, a shout (the camera zooms in), the refusal (usually "NOPE."), then praise |
| `alarm` | a real bug | 3: while he reads, a shout, then the problem. He ends with "NOPE." himself |
| `nitpick` | good work with one small thing | 3: praise, the doubt, the advice |
| `unsure` | a step you couldn't confirm | 1: while he reads. He then says "check this one yourself." |

## 6. Show it

Run `<this skill's folder>/gitceps tour <file>`. When it lists problems, fix the file and run it again until
it says "Tour opened." Then tell the user in one or two sentences what the tour found: the most important
finding, or that the change holds up.
