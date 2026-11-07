//! The mod manager: what is in a profile's mods/ folder, switching mods on and off, and
//! finding and installing mods from Modrinth (with their required dependencies).

use super::http;
use super::install::{self, run_tasks, Task};
use super::mods::{self, best_version, project_title, MrVersion, MODRINTH};
use super::paths::Paths;
use anyhow::{anyhow, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::AppHandle;

const REGISTRY: &str = ".xt-installed.json";
const MAX_ICON_BYTES: u64 = 48 * 1024;

// ---------------------------------------------------------------------------
// Paths and names
// ---------------------------------------------------------------------------

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn mods_dir(app: &AppHandle, profile_id: &str) -> Result<PathBuf, String> {
    if !safe_id(profile_id) {
        return Err("invalid profile".to_string());
    }
    let paths = Paths::new(app).map_err(|e| e.to_string())?;
    Ok(paths.instance(profile_id).join("mods"))
}

/// A plain jar file name (optionally ending in `.disabled`), never a path.
fn safe_file(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(|c: char| c == '/' || c == '\\')
        && name != ".."
        && (name.ends_with(".jar") || name.ends_with(".jar.disabled"))
}

fn base_name(file: &str) -> &str {
    file.strip_suffix(".disabled").unwrap_or(file)
}

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

// ---------------------------------------------------------------------------
// Mods installed through the launcher (the XT tiers keep their own list)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct Installed {
    file: String,
    project_id: String,
    title: String,
    version: String,
    #[serde(default)]
    dependency: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct Registry {
    #[serde(default)]
    mods: Vec<Installed>,
}

fn read_registry(dir: &Path) -> Registry {
    std::fs::read(dir.join(REGISTRY))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_registry(dir: &Path, registry: &Registry) {
    if let Ok(text) = serde_json::to_string_pretty(registry) {
        let _ = std::fs::write(dir.join(REGISTRY), text);
    }
}

// ---------------------------------------------------------------------------
// Reading a jar
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Meta {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    loader: &'static str,
    icon_path: Option<String>,
}

type Jar = zip::ZipArchive<std::fs::File>;

fn read_entry(zip: &mut Jar, name: &str, limit: u64) -> Option<Vec<u8>> {
    let entry = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    entry.take(limit).read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or("").trim().to_string()
}

fn short(text: &str, max: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let cut: String = one_line.chars().take(max).collect();
        format!("{}…", cut.trim_end())
    }
}

/// `icon` in fabric/quilt metadata is a path, or a map of size -> path.
fn pick_icon(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Object(map) => {
            let mut best: Option<(u32, String)> = None;
            for (key, value) in map {
                let (Ok(size), Some(path)) = (key.parse::<u32>(), value.as_str()) else {
                    continue;
                };
                let better = match &best {
                    None => true,
                    Some((current, _)) => {
                        if size <= 128 {
                            *current > 128 || size > *current
                        } else {
                            *current > 128 && size < *current
                        }
                    }
                };
                if better {
                    best = Some((size, path.to_string()));
                }
            }
            best.map(|b| b.1)
        }
        _ => None,
    }
}

fn names_of(v: &Value) -> String {
    let list: Vec<String> = match v {
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::String(s) => s.clone(),
                Value::Object(o) => o.get("name").map(text).unwrap_or_default(),
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .collect(),
        Value::Object(map) => map.keys().cloned().collect(),
        _ => Vec::new(),
    };
    list.into_iter().take(3).collect::<Vec<_>>().join(", ")
}

fn parse_fabric(v: &Value) -> Meta {
    Meta {
        id: text(&v["id"]),
        name: text(&v["name"]),
        version: text(&v["version"]),
        description: short(&text(&v["description"]), 240),
        author: names_of(&v["authors"]),
        loader: "fabric",
        icon_path: pick_icon(&v["icon"]),
    }
}

fn parse_quilt(v: &Value) -> Meta {
    let q = &v["quilt_loader"];
    let md = &q["metadata"];
    Meta {
        id: text(&q["id"]),
        name: text(&md["name"]),
        version: text(&q["version"]),
        description: short(&text(&md["description"]), 240),
        author: names_of(&md["contributors"]),
        loader: "quilt",
        icon_path: pick_icon(&md["icon"]),
    }
}

/// The value of `key = ...` in a (Neo)Forge mods.toml; handles one-line strings and
/// multi-line triple-quoted strings, which is all the launcher needs.
fn toml_value(lines: &[&str], key: &str) -> Option<String> {
    let triple_single = "'''";
    let triple_double = "\"\"\"";
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix('=') {
                let value = value.trim();
                for quote in [triple_single, triple_double] {
                    if let Some(after) = value.strip_prefix(quote) {
                        if let Some(end) = after.find(quote) {
                            return Some(after[..end].to_string());
                        }
                        let mut out = after.to_string();
                        let mut j = i + 1;
                        while j < lines.len() {
                            if let Some(end) = lines[j].find(quote) {
                                out.push(' ');
                                out.push_str(&lines[j][..end]);
                                break;
                            }
                            out.push(' ');
                            out.push_str(lines[j].trim());
                            j += 1;
                        }
                        return Some(out);
                    }
                }
                for quote in ['"', '\''] {
                    if let Some(after) = value.strip_prefix(quote) {
                        if let Some(end) = after.find(quote) {
                            return Some(after[..end].to_string());
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_forge(toml: &str, manifest_version: &str, loader: &'static str) -> Meta {
    let marker = "[[mods]]";
    let start = toml.find(marker).map(|i| i + marker.len()).unwrap_or(0);
    let rest = &toml[start..];
    let end = rest.find("[[").unwrap_or(rest.len());
    let section: Vec<&str> = rest[..end].lines().collect();
    let top: Vec<&str> = toml.lines().collect();

    let mut version = toml_value(&section, "version").unwrap_or_default();
    if version.contains("${") {
        version = manifest_version.to_string();
    }
    Meta {
        id: toml_value(&section, "modId").unwrap_or_default(),
        name: toml_value(&section, "displayName").unwrap_or_default(),
        version,
        description: short(&toml_value(&section, "description").unwrap_or_default(), 240),
        author: toml_value(&section, "authors")
            .or_else(|| toml_value(&top, "authors"))
            .unwrap_or_default(),
        loader,
        icon_path: toml_value(&section, "logoFile").or_else(|| toml_value(&top, "logoFile")),
    }
}

fn manifest_version(zip: &mut Jar) -> String {
    let Some(bytes) = read_entry(zip, "META-INF/MANIFEST.MF", 64 * 1024) else {
        return String::new();
    };
    String::from_utf8_lossy(&bytes)
        .lines()
        .find_map(|l| l.strip_prefix("Implementation-Version:"))
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn read_meta(zip: &mut Jar) -> Meta {
    if let Some(bytes) = read_entry(zip, "fabric.mod.json", 256 * 1024) {
        if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
            return parse_fabric(&v);
        }
    }
    if let Some(bytes) = read_entry(zip, "quilt.mod.json", 256 * 1024) {
        if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
            return parse_quilt(&v);
        }
    }
    for (entry, loader) in [("META-INF/neoforge.mods.toml", "neoforge"), ("META-INF/mods.toml", "forge")] {
        if let Some(bytes) = read_entry(zip, entry, 256 * 1024) {
            let toml = String::from_utf8_lossy(&bytes).into_owned();
            let version = manifest_version(zip);
            return parse_forge(&toml, &version, loader);
        }
    }
    Meta {
        loader: "unknown",
        ..Meta::default()
    }
}

fn icon_data_url(zip: &mut Jar, path: &str) -> Option<String> {
    let path = path.trim_start_matches('/');
    let lower = path.to_lowercase();
    let mime = if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        return None;
    };
    let bytes = read_entry(zip, path, MAX_ICON_BYTES + 1)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_ICON_BYTES {
        return None;
    }
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn sha1_file(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hex::encode(hasher.finalize()))
}

// ---------------------------------------------------------------------------
// Listing
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    /// The file as it is on disk (ends in `.disabled` when switched off)
    pub file: String,
    pub enabled: bool,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub mod_id: String,
    pub loader: String,
    pub icon: Option<String>,
    pub size: u64,
    pub modified: u64,
    /// xt (kept by the XT tier) | modrinth (installed from the browser) | manual
    pub source: String,
    pub project_id: Option<String>,
    pub sha1: String,
}

fn scan(dir: &Path) -> Vec<ModEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let managed: HashSet<String> = mods::managed_files(dir).into_iter().collect();
    let registry = read_registry(dir);

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file) = path.file_name().and_then(|n| n.to_str()).map(String::from) else {
            continue;
        };
        if !safe_file(&file) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let enabled = file.ends_with(".jar");
        let base = base_name(&file).to_string();

        let mut parsed = Meta::default();
        let mut icon = None;
        if let Ok(f) = std::fs::File::open(&path) {
            if let Ok(mut zip) = zip::ZipArchive::new(f) {
                parsed = read_meta(&mut zip);
                if let Some(p) = parsed.icon_path.clone() {
                    icon = icon_data_url(&mut zip, &p);
                }
            }
        }

        let stem = base.strip_suffix(".jar").unwrap_or(&base).to_string();
        let registered = registry.mods.iter().find(|m| m.file == base);
        let source = if managed.contains(&base) {
            "xt"
        } else if registered.is_some() {
            "modrinth"
        } else {
            "manual"
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        out.push(ModEntry {
            file,
            enabled,
            name: if parsed.name.is_empty() { stem } else { parsed.name },
            version: parsed.version,
            description: parsed.description,
            author: parsed.author,
            mod_id: parsed.id,
            loader: if parsed.loader.is_empty() {
                "unknown".to_string()
            } else {
                parsed.loader.to_string()
            },
            icon,
            size: meta.len(),
            modified,
            source: source.to_string(),
            project_id: registered.map(|m| m.project_id.clone()),
            sha1: sha1_file(&path).unwrap_or_default(),
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

#[tauri::command]
pub async fn list_mods(app: AppHandle, profile_id: String) -> Result<Vec<ModEntry>, String> {
    let dir = mods_dir(&app, &profile_id)?;
    tokio::task::spawn_blocking(move || scan(&dir))
        .await
        .map_err(|e| e.to_string())
}

/// Switch a mod on or off by renaming `x.jar` <-> `x.jar.disabled`. Returns the new file name.
#[tauri::command]
pub async fn set_mod_enabled(
    app: AppHandle,
    profile_id: String,
    file: String,
    enabled: bool,
) -> Result<String, String> {
    let dir = mods_dir(&app, &profile_id)?;
    if !safe_file(&file) {
        return Err("invalid file name".to_string());
    }
    let base = base_name(&file).to_string();
    let on = dir.join(&base);
    let off = dir.join(format!("{base}.disabled"));
    let (from, to, result) = if enabled {
        (off, on, base.clone())
    } else {
        (on, off, format!("{base}.disabled"))
    };
    if from.exists() {
        if to.exists() {
            let _ = tokio::fs::remove_file(&to).await;
        }
        tokio::fs::rename(&from, &to).await.map_err(|e| e.to_string())?;
    }
    Ok(result)
}

/// Delete a mod you added yourself. Mods the XT tier keeps can only be switched off.
#[tauri::command]
pub async fn remove_mod(app: AppHandle, profile_id: String, file: String) -> Result<(), String> {
    let dir = mods_dir(&app, &profile_id)?;
    if !safe_file(&file) {
        return Err("invalid file name".to_string());
    }
    let base = base_name(&file).to_string();
    if mods::managed_files(&dir).contains(&base) {
        return Err("XT keeps this mod up to date. Switch it off instead of deleting it.".to_string());
    }
    tokio::fs::remove_file(dir.join(&file))
        .await
        .map_err(|e| e.to_string())?;

    let mut registry = read_registry(&dir);
    let before = registry.mods.len();
    registry.mods.retain(|m| m.file != base);
    if registry.mods.len() != before {
        write_registry(&dir, &registry);
    }
    Ok(())
}

/// Copy jar files the user dropped on the window into the profile's mods folder.
#[tauri::command]
pub async fn add_mod_files(app: AppHandle, profile_id: String, paths: Vec<String>) -> Result<usize, String> {
    let dir = mods_dir(&app, &profile_id)?;
    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    let mut added = 0;
    for source in paths {
        let source = PathBuf::from(source);
        let Some(name) = source.file_name().and_then(|n| n.to_str()).map(String::from) else {
            continue;
        };
        if !name.to_lowercase().ends_with(".jar") || !source.is_file() {
            continue;
        }
        let target = dir.join(&name);
        if target.exists() || dir.join(format!("{name}.disabled")).exists() {
            continue;
        }
        if tokio::fs::copy(&source, &target).await.is_ok() {
            added += 1;
        }
    }
    Ok(added)
}

// ---------------------------------------------------------------------------
// Modrinth: search, identify, install
// ---------------------------------------------------------------------------

fn loader_set(loader: &str) -> Result<Vec<&'static str>> {
    match loader {
        "fabric" => Ok(vec!["fabric"]),
        // Quilt runs Fabric mods too
        "quilt" => Ok(vec!["quilt", "fabric"]),
        "forge" => Ok(vec!["forge"]),
        "neoforge" => Ok(vec!["neoforge"]),
        _ => Err(anyhow!(
            "Vanilla profiles don't use mods. Pick a loader for this profile first."
        )),
    }
}

async fn resolve_mc(app: &AppHandle, mc: &str) -> Result<String> {
    if mc == "latest" || mc.is_empty() {
        let paths = Paths::new(app)?;
        Ok(install::load_manifest(&paths).await?.latest.release)
    } else {
        Ok(mc.to_string())
    }
}

#[derive(Deserialize)]
struct RawHit {
    project_id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    follows: u64,
    #[serde(default)]
    author: String,
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Deserialize)]
struct RawSearch {
    #[serde(default)]
    hits: Vec<RawHit>,
    #[serde(default)]
    total_hits: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub author: String,
    pub categories: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub hits: Vec<Hit>,
    pub total: u64,
    /// The Minecraft version the search was filtered to
    pub mc: String,
}

async fn search(
    app: &AppHandle,
    query: &str,
    mc: &str,
    loader: &str,
    sort: &str,
    offset: u32,
) -> Result<SearchPage> {
    let mc = resolve_mc(app, mc).await?;
    let loaders = loader_set(loader)?;
    let categories: Vec<String> = loaders.iter().map(|l| format!("categories:{l}")).collect();
    let facets = json!([["project_type:mod"], [format!("versions:{mc}")], categories]).to_string();
    let index = match sort {
        "downloads" | "follows" | "newest" | "updated" => sort,
        _ => "relevance",
    };
    let offset_text = offset.to_string();
    let url = reqwest::Url::parse_with_params(
        &format!("{MODRINTH}/search"),
        &[
            ("query", query.trim()),
            ("facets", facets.as_str()),
            ("limit", "20"),
            ("offset", offset_text.as_str()),
            ("index", index),
        ],
    )?;
    let raw: RawSearch = http::get_json(url.as_str()).await?;
    Ok(SearchPage {
        total: raw.total_hits,
        mc,
        hits: raw
            .hits
            .into_iter()
            .map(|h| Hit {
                project_id: h.project_id,
                slug: h.slug,
                title: h.title,
                description: h.description,
                icon_url: h.icon_url.filter(|u| !u.is_empty()),
                downloads: h.downloads,
                follows: h.follows,
                author: h.author,
                categories: h.categories,
            })
            .collect(),
    })
}

/// Search Modrinth for mods that work with this loader and Minecraft version.
#[tauri::command]
pub async fn modrinth_search(
    app: AppHandle,
    query: String,
    mc: String,
    loader: String,
    sort: String,
    offset: u32,
) -> Result<SearchPage, String> {
    search(&app, &query, &mc, &loader, &sort, offset)
        .await
        .map_err(err)
}

#[derive(Deserialize)]
struct RawProject {
    id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    icon_url: Option<String>,
}

async fn project_infos(ids: &[String]) -> HashMap<String, RawProject> {
    let mut out = HashMap::new();
    for chunk in ids.chunks(40) {
        let ids_json = serde_json::to_string(chunk).unwrap_or_default();
        let Ok(url) = reqwest::Url::parse_with_params(
            &format!("{MODRINTH}/projects"),
            &[("ids", ids_json.as_str())],
        ) else {
            continue;
        };
        if let Ok(list) = http::get_json::<Vec<RawProject>>(url.as_str()).await {
            for project in list {
                out.insert(project.id.clone(), project);
            }
        }
    }
    out
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identified {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub icon_url: Option<String>,
    pub version: String,
}

/// Which Modrinth project each jar (by SHA-1) is.
async fn identify(hashes: &[String]) -> Result<HashMap<String, Identified>> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }
    let body = json!({ "hashes": hashes, "algorithm": "sha1" });
    let found: HashMap<String, MrVersion> =
        http::post_json(&format!("{MODRINTH}/version_files"), &body).await?;

    let mut ids: Vec<String> = found.values().map(|v| v.project_id.clone()).collect();
    ids.sort();
    ids.dedup();
    let infos = project_infos(&ids).await;

    Ok(found
        .into_iter()
        .map(|(hash, version)| {
            let info = infos.get(&version.project_id);
            (
                hash,
                Identified {
                    slug: info.map(|p| p.slug.clone()).unwrap_or_default(),
                    title: info.map(|p| p.title.clone()).unwrap_or_default(),
                    icon_url: info
                        .and_then(|p| p.icon_url.clone())
                        .filter(|u| !u.is_empty()),
                    version: version.version_number.clone(),
                    project_id: version.project_id,
                },
            )
        })
        .collect())
}

#[tauri::command]
pub async fn modrinth_identify(hashes: Vec<String>) -> Result<HashMap<String, Identified>, String> {
    identify(&hashes).await.map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledInfo {
    pub title: String,
    pub file: String,
    pub dependency: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    pub installed: Vec<InstalledInfo>,
    pub skipped: Vec<String>,
}

/// Project ids of everything already in the folder (so dependencies aren't added twice).
async fn present_projects(dir: &Path) -> HashSet<String> {
    let mut ids: HashSet<String> = read_registry(dir)
        .mods
        .into_iter()
        .map(|m| m.project_id)
        .collect();
    let owned = dir.to_path_buf();
    let hashes: Vec<String> = tokio::task::spawn_blocking(move || {
        std::fs::read_dir(&owned)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.ends_with(".jar") || n.ends_with(".jar.disabled"))
                            .unwrap_or(false)
                    })
                    .filter_map(|p| sha1_file(&p))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default();
    if let Ok(found) = identify(&hashes).await {
        ids.extend(found.into_values().map(|i| i.project_id));
    }
    ids
}

async fn install_project(
    app: &AppHandle,
    profile_id: &str,
    project: &str,
    mc: &str,
    loader: &str,
) -> Result<InstallReport> {
    let mc = resolve_mc(app, mc).await?;
    let loaders = loader_set(loader)?;
    let paths = Paths::new(app)?;
    let dir = paths.instance(profile_id).join("mods");
    tokio::fs::create_dir_all(&dir).await?;
    install::reset_cancel();

    let present = present_projects(&dir).await;

    // The mod itself, then its required dependencies
    let mut chosen: Vec<(MrVersion, bool)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut queue: Vec<(String, bool)> = vec![(project.to_string(), false)];

    while let Some((id, is_dependency)) = queue.pop() {
        if chosen.len() > 40 {
            skipped.push("Stopped adding dependencies after 40 mods".to_string());
            break;
        }
        let version = match best_version(&id, &mc, &loaders).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                let title = project_title(&id).await;
                if is_dependency {
                    skipped.push(format!("Required mod {title} has no build for Minecraft {mc}"));
                    continue;
                }
                return Err(anyhow!(
                    "{title} has no build for Minecraft {mc} with this loader yet"
                ));
            }
            Err(e) => {
                if is_dependency {
                    skipped.push(format!("Couldn't look up a required mod ({id}): {e}"));
                    continue;
                }
                return Err(e);
            }
        };
        if !seen.insert(version.project_id.clone()) {
            continue;
        }
        if present.contains(&version.project_id) {
            if is_dependency {
                continue; // already installed
            }
            return Err(anyhow!("That mod is already installed in this profile"));
        }
        for dep in &version.dependencies {
            if dep.dependency_type != "required" {
                continue;
            }
            if let Some(dep_id) = &dep.project_id {
                if !seen.contains(dep_id) && !present.contains(dep_id) {
                    queue.push((dep_id.clone(), true));
                }
            }
        }
        chosen.push((version, is_dependency));
    }

    let mut tasks = Vec::new();
    let mut files: Vec<(usize, String)> = Vec::new();
    for (i, (version, _)) in chosen.iter().enumerate() {
        let file = version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| version.files.first());
        let Some(file) = file else {
            continue;
        };
        let Some(name) = Path::new(&file.filename)
            .file_name()
            .and_then(|n| n.to_str())
        else {
            continue;
        };
        files.push((i, name.to_string()));
        tasks.push(Task {
            url: file.url.clone(),
            dest: dir.join(name),
            sha1: file.hashes.sha1.clone(),
            size: file.size,
        });
    }
    run_tasks(app, "mods", "Installing mods", tasks).await?;

    let ids: Vec<String> = chosen.iter().map(|(v, _)| v.project_id.clone()).collect();
    let infos = project_infos(&ids).await;

    let mut registry = read_registry(&dir);
    let mut installed = Vec::new();
    for (i, name) in files {
        let (version, is_dependency) = &chosen[i];
        let title = infos
            .get(&version.project_id)
            .map(|p| p.title.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| name.clone());
        registry
            .mods
            .retain(|m| m.project_id != version.project_id && m.file != name);
        registry.mods.push(Installed {
            file: name.clone(),
            project_id: version.project_id.clone(),
            title: title.clone(),
            version: version.version_number.clone(),
            dependency: *is_dependency,
        });
        installed.push(InstalledInfo {
            title,
            file: name,
            dependency: *is_dependency,
        });
    }
    write_registry(&dir, &registry);

    Ok(InstallReport { installed, skipped })
}

/// Install a Modrinth project (and its required dependencies) into a profile.
#[tauri::command]
pub async fn modrinth_install(
    app: AppHandle,
    profile_id: String,
    project: String,
    mc: String,
    loader: String,
) -> Result<InstallReport, String> {
    if !safe_id(&profile_id) {
        return Err("invalid profile".to_string());
    }
    install_project(&app, &profile_id, &project, &mc, &loader)
        .await
        .map_err(err)
}
