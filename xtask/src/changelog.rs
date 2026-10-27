//! `cargo xtask changelog [--check]`: what CHANGELOG.md and HANDOFF.md may
//! hold, and the git facts they may not (see ~/.agents/coding.md, "Facts that
//! come from git").
//!
//! With no flag it prints, for each CHANGELOG entry, when and in which merge
//! commit its branch landed, read from git in UTC. With `--check` it fails if
//! an entry has no UTC timestamp or holds a commit id, a pull request number
//! or a merge state, apart from entries marked `Backfilled history`, which
//! keep what they had.

use std::fs;
use std::path::Path;
use std::process::Command;

/// What a file must not say, whatever the wording: git has it.
const STATES: [&str; 5] = [
    "awaiting review",
    "pending merge",
    "in flight: pr",
    "merged at",
    "merged on",
];

pub fn run(root: &Path, check: bool) -> Result<(), String> {
    let changelog = read(root, "CHANGELOG.md")?;
    let handoff = read(root, "HANDOFF.md")?;
    if check {
        let mut problems = Vec::new();
        for (name, text) in [("CHANGELOG.md", &changelog), ("HANDOFF.md", &handoff)] {
            problems.extend(problems_in(name, text));
        }
        if problems.is_empty() {
            println!("CHANGELOG.md and HANDOFF.md are in order");
            return Ok(());
        }
        return Err(problems.join("\n"));
    }
    for entry in entries(&changelog) {
        // Backfilled history stands for many branches, so it has no one merge.
        let Some(branch) = entry.branch().filter(|_| !entry.backfilled()) else {
            continue;
        };
        println!("{branch}: {}", merge_of(root, &branch));
    }
    Ok(())
}

fn read(root: &Path, name: &str) -> Result<String, String> {
    fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))
}

/// One `---`-separated entry of the changelog.
struct Entry<'a> {
    text: &'a str,
}

impl Entry<'_> {
    fn title(&self) -> &str {
        self.text
            .lines()
            .find_map(|l| l.strip_prefix("### "))
            .unwrap_or("")
    }

    /// The `Branch:` line's branch, in backticks.
    fn branch(&self) -> Option<String> {
        let line = self.text.lines().find(|l| l.starts_with("- Branch:"))?;
        Some(line.split('`').nth(1)?.to_string())
    }

    fn backfilled(&self) -> bool {
        self.text.to_lowercase().contains("backfilled history")
    }
}

fn entries(changelog: &str) -> Vec<Entry<'_>> {
    changelog
        .split("\n---\n")
        .filter(|part| part.lines().any(|l| l.starts_with("### ")))
        .map(|text| Entry { text })
        .collect()
}

/// An ISO 8601 UTC timestamp, such as `2026-10-16T15:30:01Z`.
fn has_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    let shape = b"dddd-dd-ddTdd:dd:ddZ";
    bytes.windows(shape.len()).any(|w| {
        w.iter().zip(shape).all(|(b, s)| match s {
            b'd' => b.is_ascii_digit(),
            _ => b == s,
        })
    })
}

/// A word that reads as a commit id: 7 to 40 hex digits, with both a digit
/// and a letter, so a year or a plain word isn't one.
fn is_commit_id(word: &str) -> bool {
    (7..=40).contains(&word.len())
        && word.chars().all(|c| c.is_ascii_hexdigit())
        && word.chars().any(|c| c.is_ascii_digit())
        && word.chars().any(|c| c.is_ascii_alphabetic())
}

/// `#123`, as a pull request or issue is cited.
fn has_pull_number(line: &str) -> bool {
    line.split(|c: char| c.is_whitespace() || "([,;".contains(c))
        .any(|w| {
            w.strip_prefix('#').is_some_and(|n| {
                !n.is_empty()
                    && n.trim_end_matches([')', '.', ','])
                        .chars()
                        .all(|c| c.is_ascii_digit())
            })
        })
}

fn git_facts_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let lower = line.to_lowercase();
        let words = line.split(|c: char| !c.is_ascii_alphanumeric());
        if words.clone().any(is_commit_id) {
            found.push(format!("line {}: a commit id", n + 1));
        }
        if has_pull_number(line) || lower.contains("pull request #") {
            found.push(format!("line {}: a pull request number", n + 1));
        }
        if let Some(state) = STATES.iter().find(|s| lower.contains(*s)) {
            found.push(format!("line {}: \"{state}\", which git knows", n + 1));
        }
    }
    found
}

fn problems_in(name: &str, text: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if name == "HANDOFF.md" {
        if !has_timestamp(text) {
            problems.push(format!("{name}: no ISO 8601 UTC timestamp"));
        }
        problems.extend(
            git_facts_in(text)
                .into_iter()
                .map(|p| format!("{name}: {p}")),
        );
        return problems;
    }
    for entry in entries(text) {
        if entry.backfilled() {
            continue;
        }
        let title = entry.title();
        if !has_timestamp(entry.text) {
            problems.push(format!("{name}: \"{title}\" has no ISO 8601 UTC timestamp"));
        }
        problems.extend(
            git_facts_in(entry.text)
                .into_iter()
                .map(|p| format!("{name}: \"{title}\" {p}")),
        );
    }
    problems
}

/// The merge commit that landed `branch`, as the hosting service names it
/// ("Merge pull request #n from owner/branch"), with its time in UTC.
fn merge_of(root: &Path, branch: &str) -> String {
    let output = Command::new("git")
        .args(["log", "--merges", "--format=%ct%x09%h%x09%s"])
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return "git isn't available".into();
    };
    let log = String::from_utf8_lossy(&output.stdout);
    for line in log.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(time), Some(id), Some(subject)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if subject.ends_with(&format!("/{branch}")) {
            let when = time.parse().map_or_else(|_| "?".into(), utc);
            return format!("merged {when} in {id} ({subject})");
        }
    }
    "not merged".into()
}

/// A Unix time as `YYYY-MM-DDTHH:MM:SSZ`.
fn utc(epoch: i64) -> String {
    let (days, secs) = (epoch.div_euclid(86_400), epoch.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_are_utc_iso_8601() {
        assert_eq!(utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc(1_792_000_000), "2026-10-14T17:46:40Z");
        assert_eq!(utc(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn git_facts_are_found_and_plain_text_is_not() {
        assert!(git_facts_in("Added on 2026-10-24: a deadbeef of notes").is_empty());
        assert_eq!(git_facts_in("Fixed in 5e18e9f").len(), 1);
        assert_eq!(git_facts_in("see PR (#77)").len(), 1);
        assert_eq!(git_facts_in("Awaiting review").len(), 1);
        assert!(git_facts_in("OWLY-3 held up in 2026, with 1234567 bytes").is_empty());
    }

    #[test]
    fn entries_need_a_timestamp_unless_backfilled() {
        let text = "# Log\n\n---\n### a: one\n\n- Branch: `a`\n\n---\n### b: two\n\n- Written: 2026-10-24T21:39:33Z\n\n---\n### c: old\n\nBackfilled history, 5e18e9f.\n";
        let problems = problems_in("CHANGELOG.md", text);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("\"a: one\""));
    }
}
