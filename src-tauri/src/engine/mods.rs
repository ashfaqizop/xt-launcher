//! XT Optimized and XT Optimized+: curated Fabric mods resolved through Modrinth.
//!
//! The mod list lives in `manifest/xt-mods.json` in the repository, so it can change
//! without a launcher update (an embedded copy is the offline fallback). For the profile's
//! Minecraft version each mod's newest compatible build is found, required dependencies
//! are added automatically, and the jars are kept in the profile's `mods/` folder. Only
//! files this launcher installed are ever replaced or removed.

use super::http;
use super::install::{run_tasks, Task};
use super::paths::Paths;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/ashfaqizop/xt-launcher/main/manifest/xt-mods.json";
const EMBEDDED_MANIFEST: &str = include_str!("../../../manifest/xt-mods.json");
pub(super) const MODRINTH: &str = "https://api.modrinth.com/v2";
/// Don't re-query Modrinth more often than this for the same profile and Minecraft version.
const RECHECK_SECS: u64 = 6 * 60 * 60;
const STATE_FILE: &str = ".xt-managed.json";

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Manifest {
    tiers: HashMap<String, Tier>,
}

#[derive(Deserialize)]
struct Tier {
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    mods: Vec<String>,
}

async fn load_manifest() -> Manifest {
    let remote = tokio::time::timeout(Duration::from_secs(6), http::get_json::<Manifest>(MANIFEST_URL)).await;
    if let Ok(Ok(manifest)) = remote {
        return manifest;
    }
    serde_json::from_str(EMBEDDED_MANIFEST).unwrap_or(Manifest {
        tiers: HashMap::new(),
    })
}

/// The tier's mod slugs, including those it extends, in order and without duplicates.
fn tier_slugs(manifest: &Manifest, tier: &str) -> Vec<String> {
    let mut chain: Vec<&Tier> = Vec::new();
    let mut current = Some(tier.to_string());
    while let Some(name) = current {
        if chain.len() > 8 {
            break;
        }
        match manifest.tiers.get(&name) {
            Some(t) => {
                chain.push(t);
                current = t.extends.clone();
            }
            None => break,
        }
    }

    let mut seen = HashSet::new();
    let mut slugs = Vec::new();
    for t in chain.into_iter().rev() {
        for slug in &t.mods {
            if seen.insert(slug.clone()) {
                slugs.push(slug.clone());
            }
        }
    }
    slugs
}

// ---------------------------------------------------------------------------
// Modrinth
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone)]
pub(super) struct MrVersion {
    #[serde(default)]
    pub id: String,
    pub project_id: String,
    #[serde(default)]
    pub version_number: String,
    pub version_type: String,
    #[serde(default)]
    pub files: Vec<MrFile>,
    #[serde(default)]
    pub dependencies: Vec<MrDependency>,
}

#[derive(Deserialize, Clone)]
pub(super) struct MrFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: Option<u64>,
    pub hashes: MrHashes,
}

#[derive(Deserialize, Clone)]
pub(super) struct MrHashes {
    #[serde(default)]
    pub sha1: Option<String>,
}

#[derive(Deserialize, Clone)]
pub(super) struct MrDependency {
    #[serde(default)]
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Deserialize)]
struct MrProject {
    title: String,
}

/// A friendly name for messages; falls back to the slug or id.
pub(super) async fn project_title(project: &str) -> String {
    match http::get_json::<MrProject>(&format!("{MODRINTH}/project/{project}")).await {
        Ok(p) => p.title,
        Err(_) => project.to_string(),
    }
}

/// Newest compatible build of a project (a slug or an id), preferring releases.
pub(super) async fn best_version(project: &str, mc: &str, loader: &[&str]) -> Result<Option<MrVersion>> {
    let loaders = serde_json::to_string(loader)?;
    let game_versions = format!("[\"{mc}\"]");
    let url = reqwest::Url::parse_with_params(
        &format!("{MODRINTH}/project/{project}/version"),
        &[
            ("loaders", loaders.as_str()),
            ("game_versions", game_versions.as_str()),
        ],
    )?;
    let versions: Vec<MrVersion> = http::get_json(url.as_str()).await?;

    let pick = versions
        .iter()
        .find(|v| v.version_type == "release")
        .or_else(|| versions.iter().find(|v| v.version_type == "beta"))
        .or_else(|| versions.first());
    Ok(pick.cloned())
}

// ---------------------------------------------------------------------------
// Installed state
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Default)]
struct State {
    #[serde(default)]
    tier: String,
    #[serde(default)]
    mc: String,
    #[serde(default)]
    checked: u64,
    #[serde(default)]
    files: Vec<String>,
}

/// Files the XT tier installed in this profile's mods folder (jar names, enabled or not).
pub fn managed_files(mods_dir: &Path) -> Vec<String> {
    std::fs::read(mods_dir.join(STATE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<State>(&bytes).ok())
        .map(|s| s.files)
        .unwrap_or_default()
}

/// Make the next sync re-resolve and re-verify every XT mod (used by "Repair profile").
pub async fn force_recheck(mods_dir: &Path) {
    let mut state = read_state(mods_dir).await;
    if state.files.is_empty() {
        return;
    }
    state.checked = 0;
    write_state(mods_dir, &state).await;
}

/// The jar exists, either enabled or disabled by the user (`name.jar.disabled`).
fn present(mods_dir: &Path, name: &str) -> bool {
    mods_dir.join(name).exists() || mods_dir.join(format!("{name}.disabled")).exists()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn read_state(mods_dir: &Path) -> State {
    match tokio::fs::read(mods_dir.join(STATE_FILE)).await {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => State::default(),
    }
}

async fn write_state(mods_dir: &Path, state: &State) {
    if let Ok(text) = serde_json::to_string_pretty(state) {
        let _ = tokio::fs::write(mods_dir.join(STATE_FILE), text).await;
    }
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

pub struct Report {
    /// Number of mod jars managed for this profile now
    pub count: usize,
    /// Human-readable reasons for mods that could not be installed
    pub skipped: Vec<String>,
    /// Modrinth was unreachable, existing mods were left untouched
    pub offline: bool,
    /// Nothing to do: checked recently with the same tier and version
    pub up_to_date: bool,
}

/// Make the profile's mods folder match the tier for `mc`.
pub async fn sync(
    app: &AppHandle,
    paths: &Paths,
    profile_id: &str,
    tier: &str,
    mc: &str,
    loader: &str,
) -> Result<Report> {
    let mods_dir = paths.instance(profile_id).join("mods");
    tokio::fs::create_dir_all(&mods_dir).await?;

    let old = read_state(&mods_dir).await;
    let fresh = old.tier == tier
        && old.mc == mc
        && now_secs().saturating_sub(old.checked) < RECHECK_SECS
        && !old.files.is_empty()
        && old.files.iter().all(|f| present(&mods_dir, f));
    if fresh {
        return Ok(Report {
            count: old.files.len(),
            skipped: Vec::new(),
            offline: false,
            up_to_date: true,
        });
    }

    let manifest = load_manifest().await;
    let roots = tier_slugs(&manifest, tier);
    if roots.is_empty() {
        return Err(anyhow!("the mod list for '{tier}' is empty or unknown"));
    }

    // Resolve mods and their required dependencies, a batch at a time
    let mut chosen: HashMap<String, MrVersion> = HashMap::new();
    let mut seen: HashSet<String> = roots.iter().cloned().collect();
    let mut queue: Vec<String> = roots;
    let mut skipped: Vec<String> = Vec::new();
    let mut attempts = 0usize;
    let mut failures = 0usize;

    while !queue.is_empty() {
        let batch: Vec<String> = std::mem::take(&mut queue);
        let results = futures_util::stream::iter(batch.into_iter().map(|project| async move {
            let result = best_version(&project, mc, &[loader]).await;
            (project, result)
        }))
        .buffer_unordered(6)
        .collect::<Vec<_>>()
        .await;

        for (project, result) in results {
            attempts += 1;
            match result {
                Ok(Some(version)) => {
                    if !chosen.contains_key(&version.project_id) {
                        for dep in &version.dependencies {
                            if dep.dependency_type != "required" {
                                continue;
                            }
                            if let Some(id) = &dep.project_id {
                                if seen.insert(id.clone()) {
                                    queue.push(id.clone());
                                }
                            }
                        }
                        chosen.insert(version.project_id.clone(), version);
                    }
                }
                Ok(None) => {
                    let title = project_title(&project).await;
                    skipped.push(format!("{title} has no build for Minecraft {mc} yet"));
                }
                Err(e) => {
                    failures += 1;
                    skipped.push(format!("{project}: {e}"));
                }
            }
        }
    }

    // Offline (or Modrinth down): leave whatever is installed alone
    if attempts > 0 && failures == attempts {
        return Ok(Report {
            count: old.files.len(),
            skipped: Vec::new(),
            offline: true,
            up_to_date: false,
        });
    }

    // Files to install
    let mut tasks = Vec::new();
    let mut files: Vec<String> = Vec::new();
    for version in chosen.values() {
        let file = version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| version.files.first());
        let Some(file) = file else {
            continue;
        };
        let Some(name) = Path::new(&file.filename).file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        files.push(name.to_string());
        // A mod the user switched off stays off (and in place)
        if mods_dir.join(format!("{name}.disabled")).exists() {
            continue;
        }
        tasks.push(Task {
            url: file.url.clone(),
            dest: mods_dir.join(name),
            sha1: file.hashes.sha1.clone(),
            size: file.size,
        });
    }
    files.sort();

    // Remove builds we installed earlier that are no longer wanted
    for stale in old.files.iter().filter(|f| !files.contains(f)) {
        let _ = tokio::fs::remove_file(mods_dir.join(stale)).await;
        let _ = tokio::fs::remove_file(mods_dir.join(format!("{stale}.disabled"))).await;
    }

    run_tasks(app, "mods", "Updating mods", tasks).await?;

    write_state(
        &mods_dir,
        &State {
            tier: tier.to_string(),
            mc: mc.to_string(),
            checked: now_secs(),
            files: files.clone(),
        },
    )
    .await;

    Ok(Report {
        count: files.len(),
        skipped,
        offline: false,
        up_to_date: false,
    })
}
