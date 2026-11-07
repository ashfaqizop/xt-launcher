//! What each mod loader actually offers: the Minecraft versions it supports and its
//! own builds for a given Minecraft version. Fetched from the loaders' servers and
//! cached for the session.

use super::http;
use super::installer::{neoforge_mc, version_key};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

const FABRIC: &str = "https://meta.fabricmc.net/v2/versions";
const QUILT: &str = "https://meta.quiltmc.org/v3/versions";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const FORGE_PROMOS: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoaderBuild {
    pub version: String,
    pub stable: bool,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LoaderCatalog {
    /// Minecraft versions this loader supports, newest first
    pub games: Vec<String>,
    /// Builds of the loader for the requested Minecraft version, newest first
    pub builds: Vec<LoaderBuild>,
    /// The build used when the profile doesn't pin one
    pub recommended: Option<String>,
}

// ---------------------------------------------------------------------------
// Session cache of fetched documents
// ---------------------------------------------------------------------------

fn cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn fetch_text(url: &str) -> Result<String> {
    if let Some(hit) = cache().lock().unwrap().get(url).cloned() {
        return Ok(hit);
    }
    let bytes = http::get_bytes(url).await?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    cache().lock().unwrap().insert(url.to_string(), text.clone());
    Ok(text)
}

fn xml_versions(xml: &str) -> Vec<String> {
    xml.split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</version>").next())
        .map(str::to_string)
        .collect()
}

fn newest_first(mut list: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    list.retain(|v| seen.insert(v.clone()));
    list.sort_by(|a, b| version_key(b).cmp(&version_key(a)).then_with(|| b.cmp(a)));
    list
}

// ---------------------------------------------------------------------------
// Fabric / Quilt (meta servers)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GameEntry {
    version: String,
}

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

async fn meta_catalog(base: &str, mc: Option<&str>) -> Result<LoaderCatalog> {
    let games: Vec<GameEntry> = serde_json::from_str(&fetch_text(&format!("{base}/game")).await?)?;
    let mut catalog = LoaderCatalog {
        games: games.into_iter().map(|g| g.version).collect(),
        ..Default::default()
    };

    if let Some(mc) = mc {
        if catalog.games.iter().any(|g| g == mc) {
            let entries: Vec<LoaderEntry> =
                serde_json::from_str(&fetch_text(&format!("{base}/loader/{mc}")).await?)?;
            catalog.builds = entries
                .into_iter()
                .map(|e| LoaderBuild {
                    stable: e.loader.stable.unwrap_or(!e.loader.version.contains('-')),
                    version: e.loader.version,
                })
                .collect();
            catalog.recommended = catalog
                .builds
                .iter()
                .find(|b| b.stable)
                .or_else(|| catalog.builds.first())
                .map(|b| b.version.clone());
        }
    }
    Ok(catalog)
}

// ---------------------------------------------------------------------------
// Forge (maven metadata: "<mc>-<forge>")
// ---------------------------------------------------------------------------

async fn forge_catalog(mc: Option<&str>) -> Result<LoaderCatalog> {
    let mut by_mc: HashMap<String, Vec<String>> = HashMap::new();
    for entry in xml_versions(&fetch_text(FORGE_MAVEN).await?) {
        let Some((game, rest)) = entry.split_once('-') else {
            continue;
        };
        // Very old builds append "-<mc>" again; drop it
        let build = rest
            .strip_suffix(&format!("-{game}"))
            .unwrap_or(rest)
            .to_string();
        by_mc.entry(game.to_string()).or_default().push(build);
    }

    let mut catalog = LoaderCatalog {
        games: newest_first(by_mc.keys().cloned().collect()),
        ..Default::default()
    };

    if let Some(mc) = mc {
        if let Some(builds) = by_mc.remove(mc) {
            let promos: Value = fetch_text(FORGE_PROMOS)
                .await
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or(Value::Null);
            let recommended = promos["promos"][format!("{mc}-recommended")]
                .as_str()
                .map(str::to_string);
            catalog.builds = newest_first(builds)
                .into_iter()
                .map(|version| LoaderBuild {
                    stable: recommended.as_deref() == Some(version.as_str()),
                    version,
                })
                .collect();
            catalog.recommended = recommended
                .filter(|r| catalog.builds.iter().any(|b| &b.version == r))
                .or_else(|| catalog.builds.first().map(|b| b.version.clone()));
        }
    }
    Ok(catalog)
}

// ---------------------------------------------------------------------------
// NeoForge (maven metadata; the Minecraft version is encoded in the build number)
// ---------------------------------------------------------------------------

async fn neoforge_catalog(mc: Option<&str>) -> Result<LoaderCatalog> {
    let mut by_mc: HashMap<String, Vec<String>> = HashMap::new();
    for build in xml_versions(&fetch_text(NEOFORGE_MAVEN).await?) {
        if let Some(game) = neoforge_mc(&build) {
            by_mc.entry(game).or_default().push(build);
        }
    }

    let mut catalog = LoaderCatalog {
        games: newest_first(by_mc.keys().cloned().collect()),
        ..Default::default()
    };

    if let Some(mc) = mc {
        if let Some(builds) = by_mc.remove(mc) {
            catalog.builds = newest_first(builds)
                .into_iter()
                .map(|version| LoaderBuild {
                    stable: !version.contains('-'),
                    version,
                })
                .collect();
            catalog.recommended = catalog
                .builds
                .iter()
                .find(|b| b.stable)
                .or_else(|| catalog.builds.first())
                .map(|b| b.version.clone());
        }
    }
    Ok(catalog)
}

pub async fn catalog(loader: &str, mc: Option<&str>) -> Result<LoaderCatalog> {
    match loader {
        "fabric" => meta_catalog(FABRIC, mc).await,
        "quilt" => meta_catalog(QUILT, mc).await,
        "forge" => forge_catalog(mc).await,
        "neoforge" => neoforge_catalog(mc).await,
        other => Err(anyhow!("{other} has no loader versions")),
    }
}

/// Minecraft versions a loader supports and, for `mc`, the loader's own builds.
#[tauri::command]
pub async fn loader_catalog(loader: String, mc: Option<String>) -> Result<LoaderCatalog, String> {
    catalog(&loader, mc.as_deref().filter(|m| !m.is_empty()))
        .await
        .map_err(|e| format!("{e:#}"))
}
