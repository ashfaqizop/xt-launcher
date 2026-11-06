//! Turn a crashed game session into something a person can act on.

use serde::Serialize;
use std::path::Path;
use std::time::{Duration, SystemTime};

#[derive(Clone, Serialize)]
pub struct CrashInfo {
    pub title: String,
    pub hint: String,
    /// Path of the crash report file, when the game wrote one
    pub report: Option<String>,
}

/// (text to look for in lowercase, headline, what to do)
const PATTERNS: &[(&str, &str, &str)] = &[
    (
        "unsupportedclassversionerror",
        "Wrong Java version",
        "This version needs a newer Java. Set Java to Automatic in Settings.",
    ),
    (
        "could not reserve enough space",
        "Not enough memory",
        "Lower the memory setting or close other programs, then try again.",
    ),
    (
        "outofmemoryerror",
        "The game ran out of memory",
        "Give the game more memory in Settings > Minecraft.",
    ),
    (
        "could not find or load main class",
        "Damaged installation",
        "Launch again to repair the game files.",
    ),
    (
        "mixin apply failed",
        "A mod failed to load",
        "Remove or update the mod you added most recently.",
    ),
    (
        "incompatible mod set",
        "Some mods don't work together",
        "Open the game log to see which mod needs another version.",
    ),
    (
        "could not find required mod",
        "A required mod is missing",
        "Install the mod named in the game log.",
    ),
];

/// Look at the newest crash report written since `started` and the tail of the log.
pub fn analyze(game_dir: &Path, started: SystemTime, log_tail: &str) -> Option<CrashInfo> {
    let (report_path, report_text) = newest_report(game_dir, started);

    let mut haystack = log_tail.to_lowercase();
    if let Some(text) = &report_text {
        haystack.push('\n');
        haystack.push_str(&text.to_lowercase());
    }

    for (needle, title, hint) in PATTERNS {
        if haystack.contains(needle) {
            return Some(CrashInfo {
                title: title.to_string(),
                hint: hint.to_string(),
                report: report_path,
            });
        }
    }

    let text = report_text?;
    let description = text
        .lines()
        .find_map(|line| line.strip_prefix("Description:"))
        .map(|d| d.trim().to_string())
        .unwrap_or_else(|| "Open the crash report for details.".to_string());
    Some(CrashInfo {
        title: "The game crashed".to_string(),
        hint: description,
        report: report_path,
    })
}

fn newest_report(game_dir: &Path, started: SystemTime) -> (Option<String>, Option<String>) {
    let dir = game_dir.join("crash-reports");
    let cutoff = started
        .checked_sub(Duration::from_secs(2))
        .unwrap_or(started);

    let newest = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            (modified >= cutoff).then(|| (modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified);

    match newest {
        Some((_, path)) => {
            let text = std::fs::read_to_string(&path)
                .ok()
                .map(|t| t.chars().take(200_000).collect::<String>());
            (Some(path.to_string_lossy().into_owned()), text)
        }
        None => (None, None),
    }
}
