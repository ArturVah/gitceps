//! The post-checkout hook. Git runs it after checkout, switch, clone and `worktree add`. Only a fresh clone
//! gets a reaction (he walks into an empty gym).
use std::ffi::OsString;

use super::{is_zero, origin_name, own_hook};
use crate::git::{ask, each_line};
use crate::switches::Switch;
use crate::window::play;

pub fn run(arguments: &[OsString]) -> u8 {
    // Git uses this hook's exit status as the checkout's own, so the repo's hook decides it.
    let status = own_hook("post-checkout", arguments, None);

    // A clone checks out from nothing (the previous HEAD is all zeros), and git logs it as "clone: from ...".
    // `worktree add`, submodules and a new repo's first checkout also start from zeros but log otherwise.
    let from_nothing = arguments
        .first()
        .is_some_and(|previous| is_zero(&previous.to_string_lossy()));
    let cloned = from_nothing && ask(&["reflog", "-1", "--format=%gs"]).starts_with("clone: ");
    if cloned && Switch::All.on() && Switch::Clone.on() {
        let mut files = 0;
        each_line(&["ls-files"], |_| {
            files += 1;
            true
        });
        let subject = ask(&["log", "-1", "--format=%s"]);
        play(
            "clone",
            &[
                ("--ins", &files.to_string()),
                ("--msg", &subject),
                ("--remote", &origin_name()),
            ],
        );
    }
    status
}
