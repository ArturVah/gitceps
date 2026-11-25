//! What looks like a secret in a push: a file that usually holds one, or a key written into the code.
use std::sync::LazyLock;

use regex::Regex;

fn pattern(text: &str) -> Regex {
    Regex::new(text).expect("the pattern is valid")
}

/// Key formats, which are case-sensitive. "sk-" covers OpenAI and Anthropic keys.
static KEY: LazyLock<Regex> = LazyLock::new(|| {
    pattern(concat!(
        r"ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk_live_[A-Za-z0-9]{10,}",
        r"|(^|[^A-Za-z0-9_-])sk-[A-Za-z0-9_-]{32,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,}",
        r"|-----BEGIN [A-Z ]*PRIVATE KEY-----",
    ))
});

/// A credential given a quoted value. Code that only reads one (password = request.body.password) passes.
static ASSIGNED: LazyLock<Regex> =
    LazyLock::new(|| pattern(r#"(?i)(password|passwd|secret|api_?key|token)["']?\s*[:=]\s*["'][^"'\s]{8,}["']"#));

/// Files that usually hold a secret.
static FILE: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"(^|/)(\.env(\.[^/]*)?|id_rsa|id_ed25519|credentials\.json)$|\.pem$"));

/// Examples of an .env file are meant to be shared.
static TEMPLATE: LazyLock<Regex> = LazyLock::new(|| pattern(r"\.env\.(example|sample|template|dist)$"));

pub fn is_secret_file(path: &str) -> bool {
    FILE.is_match(path) && !TEMPLATE.is_match(path)
}

/// Whether a line of `git log -p` adds a key. Removed lines and the "+++ b/file" headers don't count.
pub fn adds_key(line: &str) -> bool {
    line.starts_with('+') && !line.starts_with("+++ ") && (KEY.is_match(line) || ASSIGNED.is_match(line))
}
