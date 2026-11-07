//! Disk use: what the launcher stores, per category and per profile, and cleaning up
//! Java runtimes and game versions that no profile uses any more.
//!
//! "In use" comes from a small marker each profile writes into its folder when it
//! launches (`.xt-launch.json`). Cleanup is only offered once every profile has one, so a
//! profile that simply hasn't been played since the marker was introduced never loses
//! the version it relies on.

use super::paths::Paths;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tauri::{AppHandle, Manager};

pub const MARKER: &str = ".xt-launch.json";

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Total size of every file under `path`.
fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

fn child_dirs(path: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Marker {
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    java_component: String,
}

fn read_marker(paths: &Paths, profile_id: &str) -> Option<Marker> {
    let bytes = std::fs::read(paths.instance(profile_id).join(MARKER)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

struct Used {
    versions: HashSet<String>,
    runtimes: HashSet<String>,
    /// Every profile has been launched with a version that writes the marker
    complete: bool,
}

fn used_by(paths: &Paths, profile_ids: &[String]) -> Used {
    let mut used = Used {
        versions: HashSet::new(),
        runtimes: HashSet::new(),
        complete: true,
    };
    for id in profile_ids.iter().filter(|id| safe_id(id)) {
        match read_marker(paths, id) {
            Some(marker) => {
                used.versions.extend(marker.versions);
                if !marker.java_component.is_empty() {
                    used.runtimes.insert(marker.java_component);
                }
            }
            None => used.complete = false,
        }
    }
    used
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub bytes: u64,
    pub in_use: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceEntry {
    pub id: String,
    pub bytes: u64,
    pub saves: u64,
    pub mods: u64,
    pub screenshots: u64,
    /// No profile uses this folder any more
    pub orphan: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageReport {
    pub total: u64,
    pub assets: u64,
    pub libraries: u64,
    pub cache: u64,
    pub runtimes: Vec<Entry>,
    pub versions: Vec<Entry>,
    pub instances: Vec<InstanceEntry>,
    /// Bytes a cleanup would free (unused runtimes and versions)
    pub reclaimable: u64,
    /// Every profile has been played since XT started tracking what it uses
    pub known: bool,
}

fn build_report(paths: &Paths, profile_ids: &[String]) -> StorageReport {
    let used = used_by(paths, profile_ids);

    let runtimes: Vec<Entry> = child_dirs(&paths.runtimes())
        .into_iter()
        .map(|name| Entry {
            bytes: dir_size(&paths.runtimes().join(&name)),
            in_use: !used.complete || used.runtimes.contains(&name),
            name,
        })
        .collect();

    let versions: Vec<Entry> = child_dirs(&paths.versions())
        .into_iter()
        .map(|name| Entry {
            bytes: dir_size(&paths.versions().join(&name)) + dir_size(&paths.natives(&name)),
            in_use: !used.complete || used.versions.contains(&name),
            name,
        })
        .collect();

    let known: HashSet<&String> = profile_ids.iter().collect();
    let instances: Vec<InstanceEntry> = child_dirs(&paths.instances())
        .into_iter()
        .map(|id| {
            let dir = paths.instances().join(&id);
            InstanceEntry {
                bytes: dir_size(&dir),
                saves: dir_size(&dir.join("saves")),
                mods: dir_size(&dir.join("mods")),
                screenshots: dir_size(&dir.join("screenshots")),
                orphan: !known.contains(&id),
                id,
            }
        })
        .collect();

    let reclaimable: u64 = runtimes
        .iter()
        .chain(versions.iter())
        .filter(|e| !e.in_use)
        .map(|e| e.bytes)
        .sum();

    StorageReport {
        total: dir_size(&paths.root),
        assets: dir_size(&paths.assets()),
        libraries: dir_size(&paths.libraries()),
        cache: dir_size(&paths.cache()),
        runtimes,
        versions,
        instances,
        reclaimable,
        known: used.complete,
    }
}

#[tauri::command]
pub async fn storage_report(app: AppHandle, profile_ids: Vec<String>) -> Result<StorageReport, String> {
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || build_report(&paths, &profile_ids))
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

/// Delete what `what` names: "runtimes" and "versions" (the ones no profile uses) or
/// "cache" (downloaded loader installers). Returns the bytes freed.
#[tauri::command]
pub async fn storage_clean(app: AppHandle, profile_ids: Vec<String>, what: Vec<String>) -> Result<u64, String> {
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || -> Result<u64, String> {
        let used = used_by(&paths, &profile_ids);
        let wants_unused = what.iter().any(|w| w == "runtimes" || w == "versions");
        if wants_unused && !used.complete {
            return Err("Play each profile once so XT knows which versions it needs.".to_string());
        }

        let mut freed = 0u64;
        if what.iter().any(|w| w == "runtimes") {
            for name in child_dirs(&paths.runtimes()) {
                if !used.runtimes.contains(&name) {
                    let dir = paths.runtimes().join(&name);
                    let size = dir_size(&dir);
                    if std::fs::remove_dir_all(&dir).is_ok() {
                        freed += size;
                    }
                }
            }
        }
        if what.iter().any(|w| w == "versions") {
            for name in child_dirs(&paths.versions()) {
                if !used.versions.contains(&name) {
                    let dir = paths.versions().join(&name);
                    let size = dir_size(&dir) + dir_size(&paths.natives(&name));
                    if std::fs::remove_dir_all(&dir).is_ok() {
                        freed += size;
                        let _ = std::fs::remove_dir_all(paths.natives(&name));
                    }
                }
            }
        }
        if what.iter().any(|w| w == "cache") {
            let dir = paths.cache().join("installers");
            let size = dir_size(&dir);
            if std::fs::remove_dir_all(&dir).is_ok() {
                freed += size;
            }
        }
        Ok(freed)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete the leftover game folder of a profile that no longer exists (worlds included),
/// refusing if any existing profile still points at it.
#[tauri::command]
pub async fn storage_delete_instance(app: AppHandle, id: String, keep: Vec<String>) -> Result<u64, String> {
    if !safe_id(&id) {
        return Err("invalid profile".to_string());
    }
    if keep.contains(&id) {
        return Err("That profile still exists.".to_string());
    }
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || -> Result<u64, String> {
        let dir = paths.instance(&id);
        let size = dir_size(&dir);
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(size)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete a profile's whole game folder (worlds, mods, screenshots) when the profile is
/// deleted. Not allowed while a game is starting or running.
#[tauri::command]
pub async fn delete_profile_files(app: AppHandle, id: String) -> Result<u64, String> {
    if !safe_id(&id) {
        return Err("invalid profile".to_string());
    }
    if app.state::<super::session::Session>().is_busy() {
        return Err("Wait for the game to close before deleting its files.".to_string());
    }
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || -> Result<u64, String> {
        let dir = paths.instance(&id);
        if !dir.exists() {
            return Ok(0);
        }
        let size = dir_size(&dir);
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(size)
    })
    .await
    .map_err(|e| e.to_string())?
}
