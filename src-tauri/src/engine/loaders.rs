//! Loader installs. Fabric and Quilt: fetch the profile for a Minecraft version from the
//! loader's meta server and store it as a local version that inherits the vanilla one.
//! Forge and NeoForge are handled by the installer module.

use super::http;
use super::installer;
use super::mojang::VersionJson;
use super::paths::Paths;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use tauri::AppHandle;

#[derive(Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: Option<bool>,
}

/// Install (or refresh) the loader and return the version id to launch.
/// `build` pins a specific loader build; otherwise the recommended (newest stable) one.
pub async fn install(
    app: &AppHandle,
    paths: &Paths,
    kind: &str,
    mc: &str,
    build: Option<&str>,
) -> Result<String> {
    match kind {
        "fabric" | "quilt" => install_meta(paths, kind, mc, build).await,
        "forge" | "neoforge" => installer::install(app, paths, kind, mc, build).await,
        other => Err(anyhow!("{other} is not supported")),
    }
}

async fn install_meta(paths: &Paths, kind: &str, mc: &str, build: Option<&str>) -> Result<String> {
    let base = match kind {
        "fabric" => "https://meta.fabricmc.net/v2/versions/loader",
        "quilt" => "https://meta.quiltmc.org/v3/versions/loader",
        other => return Err(anyhow!("{other} is not supported yet")),
    };

    match fetch_profile(paths, base, kind, mc, build).await {
        Ok(id) => Ok(id),
        // Offline: reuse a profile installed earlier for this Minecraft version
        Err(e) => local_fallback(paths, kind, mc).ok_or(e),
    }
}

async fn fetch_profile(
    paths: &Paths,
    base: &str,
    kind: &str,
    mc: &str,
    build: Option<&str>,
) -> Result<String> {
    let version = match build {
        Some(v) => v.to_string(),
        None => {
            let list: Vec<LoaderEntry> = http::get_json(&format!("{base}/{mc}")).await?;
            list.iter()
                .find(|e| e.loader.stable.unwrap_or(!e.loader.version.contains('-')))
                .or_else(|| list.first())
                .ok_or_else(|| anyhow!("no {kind} loader is available for Minecraft {mc}"))?
                .loader
                .version
                .clone()
        }
    };

    let url = format!("{base}/{mc}/{version}/profile/json");
    let bytes = http::get_bytes(&url).await?;
    let profile: VersionJson = serde_json::from_slice(&bytes)?;
    if profile.id.is_empty() {
        return Err(anyhow!("the {kind} profile has no id"));
    }

    let path = paths.version_json(&profile.id);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, &bytes).await?;
    Ok(profile.id)
}

fn local_fallback(paths: &Paths, kind: &str, mc: &str) -> Option<String> {
    let prefix = format!("{kind}-loader-");
    let suffix = format!("-{mc}");
    let mut found: Vec<String> = std::fs::read_dir(paths.versions())
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(&prefix) && name.ends_with(&suffix))
        .collect();
    found.sort();
    found.pop()
}
