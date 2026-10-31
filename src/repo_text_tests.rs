//! Repository-wide text checks (P14 step 10 acceptance, handoff/P14-air-tasking.md §8).
//!
//! They read the source and documents from disk, so a stale shortcut name or a
//! leftover dead-code allow fails here instead of waiting for a reader to notice.

use std::path::{Path, PathBuf};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .path();
        if path.is_dir() {
            files_under(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Repository-relative path with forward slashes, for messages and exemptions.
fn repo_path(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

/// The file's text, or None for a file that is not UTF-8 (images, binary
/// fixtures). Any other read error fails the test.
fn read_text(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => None,
        Err(e) => panic!("{}: {e}", path.display()),
    }
}

/// The rail has seven tabs, so the tab keys are Ctrl 1–7. Exempt: the README
/// §9 phase history, the frozen design mockups, and decision records (never
/// edited once decided). The Map's plain tool keys 1–6 stay.
#[test]
fn no_ctrl_1_6_left() {
    let old = format!("Ctrl 1{}6", '\u{2013}'); // not a literal, or this file would match
    let root = repo_root();
    let mut files = vec![root.join("USER_MANUAL.md")];
    files_under(&root.join("src"), &mut files);
    files_under(&root.join("docs"), &mut files);
    for path in files {
        let rel = repo_path(&path);
        if rel.starts_with("docs/ui-redesign/design/") || rel.starts_with("docs/decisions/") {
            continue;
        }
        let Some(mut text) = read_text(&path) else {
            continue;
        };
        if rel == "docs/ui-redesign/README.md" {
            let start = text.find("\n## 9.").expect("README §9");
            let end = text[start + 1..]
                .find("\n## ")
                .map_or(text.len(), |i| start + 1 + i);
            text.replace_range(start..end, "");
        }
        assert!(!text.contains(&old), "{rel} still says {old}");
    }
    let shortcuts = crate::help::section_for(crate::help::HelpTopic::Shortcuts);
    assert!(
        shortcuts.contains("**1–6**"),
        "Keyboard shortcuts names the map keys 1–6"
    );
    let map = crate::help::section_for(crate::help::HelpTopic::Front);
    assert!(map.contains("keys **1–6**"), "Map names the tool keys 1–6");
    assert!(
        include_str!("help.rs").contains("\"1–6\","),
        "the help shortcuts test checks 1–6"
    );
}

/// Dead-code allows tagged for P14 are gone, except those for step 12
/// (multi-player). The tag sits on the allow's line or the line above it.
#[test]
fn no_p14_dead_code_allows_left() {
    // Built, not literal, so this test's own lines do not match.
    let allow = concat!("allow(", "dead_code)");
    let tag = concat!("// ", "P14");
    let kept = concat!("// ", "P14 step 12");
    let mut files = Vec::new();
    files_under(&repo_root().join("src"), &mut files);
    let mut left = Vec::new();
    for path in files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
    {
        let text = read_text(path).unwrap_or_else(|| panic!("{} is not UTF-8", path.display()));
        let lines: Vec<&str> = text.lines().collect();
        for (n, line) in lines.iter().enumerate() {
            if !line.contains(allow) {
                continue;
            }
            let above = if n > 0 { lines[n - 1] } else { "" };
            let tagged = |l: &str| l.contains(tag) && !l.contains(kept);
            if tagged(line) || (tagged(above) && !line.contains(kept)) {
                left.push(format!("{}:{}", repo_path(path), n + 1));
            }
        }
    }
    assert!(left.is_empty(), "P14 dead-code allows left: {left:?}");
}
