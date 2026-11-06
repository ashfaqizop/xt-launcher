//! Java runtime management using Mojang's own runtime distribution (the same
//! `java-runtime-*` components the vanilla launcher uses), so no archives to unpack.

use super::http;
use super::install::{run_tasks, Task};
use super::paths::Paths;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::AppHandle;

const ALL_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Deserialize)]
struct RuntimeEntry {
    manifest: RemoteFile,
}

#[derive(Deserialize)]
struct RemoteFile {
    url: String,
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Deserialize)]
struct RuntimeFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    downloads: Option<RuntimeDownloads>,
}

#[derive(Deserialize)]
struct RuntimeDownloads {
    #[serde(default)]
    raw: Option<RemoteFile>,
}

fn platform() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "windows-arm64"
    } else if cfg!(target_arch = "x86") {
        "windows-x86"
    } else {
        "windows-x64"
    }
}

/// Make sure the Java runtime `component` is installed and return its `java.exe`.
/// An already installed runtime is used without touching the network.
pub async fn ensure(app: &AppHandle, paths: &Paths, component: &str) -> Result<PathBuf> {
    let dir = paths.runtimes().join(component);
    let java = dir.join("bin").join("java.exe");
    let marker = dir.join(".xt-complete");
    if marker.exists() && java.exists() {
        return Ok(java);
    }

    let all: HashMap<String, HashMap<String, Vec<RuntimeEntry>>> = http::get_json(ALL_URL).await?;
    let entry = all
        .get(platform())
        .and_then(|components| components.get(component))
        .and_then(|list| list.first())
        .ok_or_else(|| anyhow!("Java runtime '{component}' is not available for this platform"))?;

    let manifest: RuntimeManifest = http::get_json(&entry.manifest.url).await?;
    let mut tasks = Vec::new();
    for (relative, file) in manifest.files {
        if file.kind != "file" {
            continue;
        }
        if let Some(raw) = file.downloads.and_then(|d| d.raw) {
            tasks.push(Task {
                url: raw.url,
                dest: dir.join(&relative),
                sha1: raw.sha1,
                size: raw.size,
            });
        }
    }
    run_tasks(app, "java", "Java runtime", tasks).await?;

    if !java.exists() {
        return Err(anyhow!("the Java runtime download is incomplete"));
    }
    tokio::fs::write(&marker, b"ok").await?;
    Ok(java)
}
