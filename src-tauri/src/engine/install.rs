//! Downloading a version: client jar, libraries and assets, with parallel verified downloads.

use super::http;
use super::mojang::{
    features, merge, rules_allow, Manifest, VersionJson, MANIFEST_URL, RESOURCES_URL,
};
use super::paths::Paths;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

const PARALLEL_DOWNLOADS: usize = 16;

#[derive(Clone, Serialize)]
pub struct Progress {
    pub stage: String,
    pub done: usize,
    pub total: usize,
    pub label: String,
}

pub fn emit_progress(app: &AppHandle, stage: &str, done: usize, total: usize, label: &str) {
    let _ = app.emit(
        "engine-progress",
        Progress {
            stage: stage.to_string(),
            done,
            total,
            label: label.to_string(),
        },
    );
}

pub struct Task {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// Run downloads concurrently, reporting progress under `stage`.
pub async fn run_tasks(app: &AppHandle, stage: &str, label: &str, tasks: Vec<Task>) -> Result<()> {
    let total = tasks.len();
    if total == 0 {
        return Ok(());
    }
    emit_progress(app, stage, 0, total, label);

    let done = Arc::new(AtomicUsize::new(0));
    let step = (total / 100).max(1);

    let mut stream = futures_util::stream::iter(tasks)
        .map(|task| {
            let done = Arc::clone(&done);
            let app = app.clone();
            let stage = stage.to_string();
            let label = label.to_string();
            async move {
                http::download(&task.url, &task.dest, task.sha1.as_deref(), task.size).await?;
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n % step == 0 || n == total {
                    emit_progress(&app, &stage, n, total, &label);
                }
                Ok::<(), anyhow::Error>(())
            }
        })
        .buffer_unordered(PARALLEL_DOWNLOADS);

    while let Some(result) = stream.next().await {
        result?;
    }
    Ok(())
}

/// Latest manifest from Mojang, falling back to the last cached copy when offline.
pub async fn load_manifest(paths: &Paths) -> Result<Manifest> {
    let cache = paths.cache().join("version_manifest_v2.json");
    match http::get_bytes(MANIFEST_URL).await {
        Ok(bytes) => {
            let manifest: Manifest =
                serde_json::from_slice(&bytes).context("parsing the version manifest")?;
            let _ = tokio::fs::create_dir_all(paths.cache()).await;
            let _ = tokio::fs::write(&cache, &bytes).await;
            Ok(manifest)
        }
        Err(e) => {
            let bytes = tokio::fs::read(&cache).await.map_err(|_| e)?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

/// Read a version JSON from disk, downloading it from the manifest if missing.
async fn ensure_version_json(paths: &Paths, id: &str) -> Result<VersionJson> {
    let local = paths.version_json(id);
    if let Ok(bytes) = tokio::fs::read(&local).await {
        if let Ok(v) = serde_json::from_slice::<VersionJson>(&bytes) {
            return Ok(v);
        }
    }

    let manifest = load_manifest(paths).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| anyhow!("Minecraft version {id} was not found"))?;
    http::download(&entry.url, &local, entry.sha1.as_deref(), None).await?;
    let bytes = tokio::fs::read(&local).await?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {id}.json"))
}

/// Resolve `id` (following `inheritsFrom`) into one merged description.
/// Returns it together with the id of the vanilla version that provides the client jar.
pub async fn resolve(paths: &Paths, id: &str) -> Result<(VersionJson, String)> {
    let mut chain: Vec<VersionJson> = Vec::new();
    let mut current = id.to_string();
    loop {
        if chain.len() > 8 {
            return Err(anyhow!("version {id} has a circular or too deep inheritance"));
        }
        let v = ensure_version_json(paths, &current).await?;
        let parent = v.inherits_from.clone();
        chain.push(v);
        match parent {
            Some(p) => current = p,
            None => break,
        }
    }

    let root_id = chain.last().map(|v| v.id.clone()).unwrap_or_default();
    let mut ordered = chain.into_iter().rev();
    let mut merged = ordered
        .next()
        .ok_or_else(|| anyhow!("empty version chain"))?;
    for child in ordered {
        merged = merge(merged, child);
    }
    Ok((merged, root_id))
}

#[derive(Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

/// Download everything the version needs to run: client jar, libraries, assets.
pub async fn prepare(app: &AppHandle, paths: &Paths, v: &VersionJson, root_id: &str) -> Result<()> {
    let feats = features(true);

    // Client jar
    let client = v
        .downloads
        .as_ref()
        .and_then(|d| d.client.as_ref())
        .ok_or_else(|| anyhow!("version {} has no client download", v.id))?;
    run_tasks(
        app,
        "client",
        "Minecraft",
        vec![Task {
            url: client.url.clone(),
            dest: paths.version_jar(root_id),
            sha1: client.sha1.clone(),
            size: client.size,
        }],
    )
    .await?;

    // Libraries (plus old-style native jars)
    let mut lib_tasks = Vec::new();
    for lib in &v.libraries {
        if !rules_allow(&lib.rules, &feats) {
            continue;
        }
        for dl in lib.artifact().into_iter().chain(lib.native()) {
            lib_tasks.push(Task {
                url: dl.url,
                dest: paths.libraries().join(&dl.path),
                sha1: dl.sha1,
                size: dl.size,
            });
        }
    }
    run_tasks(app, "libraries", "Libraries", lib_tasks).await?;

    // Assets
    let index_ref = v
        .asset_index
        .as_ref()
        .ok_or_else(|| anyhow!("version {} has no asset index", v.id))?;
    let index_path = paths
        .assets()
        .join("indexes")
        .join(format!("{}.json", index_ref.id));
    http::download(
        &index_ref.url,
        &index_path,
        index_ref.sha1.as_deref(),
        index_ref.size,
    )
    .await?;
    let index: AssetIndex = serde_json::from_slice(&tokio::fs::read(&index_path).await?)
        .context("parsing the asset index")?;

    let mut seen: HashSet<&str> = HashSet::new();
    let mut asset_tasks = Vec::new();
    for object in index.objects.values() {
        if object.hash.len() < 2 || !seen.insert(object.hash.as_str()) {
            continue;
        }
        let prefix = &object.hash[..2];
        asset_tasks.push(Task {
            url: format!("{RESOURCES_URL}/{prefix}/{}", object.hash),
            dest: paths.assets().join("objects").join(prefix).join(&object.hash),
            sha1: Some(object.hash.clone()),
            size: Some(object.size),
        });
    }
    run_tasks(app, "assets", "Game assets", asset_tasks).await?;
    Ok(())
}
