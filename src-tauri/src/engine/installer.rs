//! Forge and NeoForge: run the official installer headlessly into our own folder layout
//! (libraries/ and versions/ follow the standard structure, so the result is a normal
//! local version that inherits the vanilla one).

use super::install as downloads;
use super::paths::Paths;
use super::{http, java};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::process::Stdio;
use tauri::AppHandle;

struct Plan {
    /// The version id the installer will create under versions/
    id: String,
    url: String,
    label: &'static str,
}

/// Install (or reuse) a Forge / NeoForge build for `mc` and return its version id.
/// `build` pins a specific loader build; otherwise the recommended one is used.
pub async fn install(
    app: &AppHandle,
    paths: &Paths,
    kind: &str,
    mc: &str,
    build: Option<&str>,
) -> Result<String> {
    match plan(kind, mc, build).await {
        Ok(plan) => run_plan(app, paths, mc, plan).await,
        // Offline: reuse a build installed earlier for this Minecraft version
        Err(e) => local_fallback(paths, kind, mc).ok_or(e),
    }
}

/// `1.21.1` -> `21.1.`, `1.21` -> `21.0.`, `26.3` -> `26.3.0.`, `26.1.2` -> `26.1.2.`
fn neoforge_prefix(mc: &str) -> String {
    let mut parts: Vec<&str> = mc.split('.').collect();
    if parts.first() == Some(&"1") {
        parts.remove(0);
        while parts.len() < 2 {
            parts.push("0");
        }
    } else {
        while parts.len() < 3 {
            parts.push("0");
        }
    }
    format!("{}.", parts.join("."))
}

/// The Minecraft version a NeoForge build is for: `21.1.172` -> `1.21.1`,
/// `21.0.5` -> `1.21`, `26.3.0.101` -> `26.3`, `26.1.2.126` -> `26.1.2`.
pub(super) fn neoforge_mc(build: &str) -> Option<String> {
    let core = build.split('-').next()?;
    let p: Vec<&str> = core.split('.').collect();
    if p.len() < 3 {
        return None;
    }
    let major: u32 = p[0].parse().ok()?;
    if major < 25 {
        Some(if p[1] == "0" {
            format!("1.{}", p[0])
        } else {
            format!("1.{}.{}", p[0], p[1])
        })
    } else {
        if p.len() < 4 {
            return None;
        }
        Some(if p[2] == "0" {
            format!("{}.{}", p[0], p[1])
        } else {
            format!("{}.{}.{}", p[0], p[1], p[2])
        })
    }
}

/// Numeric ordering key: `26.3.0.101` -> [26, 3, 0, 101]
pub(super) fn version_key(version: &str) -> Vec<u64> {
    version
        .split('-')
        .next()
        .unwrap_or("")
        .split('.')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

async fn plan(kind: &str, mc: &str, build: Option<&str>) -> Result<Plan> {
    if let Some(version) = build {
        return Ok(pinned_plan(kind, mc, version)?);
    }
    match kind {
        "forge" => {
            let bytes = http::get_bytes(
                "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
            )
            .await?;
            let json: Value = serde_json::from_slice(&bytes)?;
            let promos = json
                .get("promos")
                .ok_or_else(|| anyhow!("unexpected Forge promotions format"))?;
            let version = promos
                .get(format!("{mc}-recommended"))
                .or_else(|| promos.get(format!("{mc}-latest")))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Forge isn't available for Minecraft {mc}"))?;
            Ok(Plan {
                id: format!("{mc}-forge-{version}"),
                url: format!(
                    "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{version}/forge-{mc}-{version}-installer.jar"
                ),
                label: "Forge",
            })
        }
        "neoforge" => {
            let bytes = http::get_bytes(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
            )
            .await?;
            let xml = String::from_utf8_lossy(&bytes).into_owned();
            let prefix = neoforge_prefix(mc);

            let mut candidates: Vec<&str> = xml
                .split("<version>")
                .skip(1)
                .filter_map(|chunk| chunk.split("</version>").next())
                .filter(|v| v.starts_with(prefix.as_str()))
                .collect();
            let stable: Vec<&str> = candidates
                .iter()
                .copied()
                .filter(|v| !v.contains('-'))
                .collect();
            if !stable.is_empty() {
                candidates = stable;
            }
            let version = candidates
                .into_iter()
                .max_by_key(|v| version_key(v))
                .ok_or_else(|| anyhow!("NeoForge isn't available for Minecraft {mc}"))?;
            Ok(Plan {
                id: format!("neoforge-{version}"),
                url: format!(
                    "https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"
                ),
                label: "NeoForge",
            })
        }
        other => Err(anyhow!("{other} is not an installer-based loader")),
    }
}

fn pinned_plan(kind: &str, mc: &str, version: &str) -> Result<Plan> {
    match kind {
        "forge" => Ok(Plan {
            id: format!("{mc}-forge-{version}"),
            url: format!(
                "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{version}/forge-{mc}-{version}-installer.jar"
            ),
            label: "Forge",
        }),
        "neoforge" => Ok(Plan {
            id: format!("neoforge-{version}"),
            url: format!(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"
            ),
            label: "NeoForge",
        }),
        other => Err(anyhow!("{other} is not an installer-based loader")),
    }
}

async fn run_plan(app: &AppHandle, paths: &Paths, mc: &str, plan: Plan) -> Result<String> {
    if !http::verify_all() && tokio::fs::metadata(paths.version_json(&plan.id)).await.is_ok() {
        return Ok(plan.id);
    }
    downloads::emit_progress(app, "loader", 0, 0, &format!("Installing {}", plan.label));

    // The installer runs on the Java this Minecraft version needs
    let (vanilla, _) = downloads::resolve(paths, mc).await?;
    let component = vanilla
        .java_version
        .as_ref()
        .map(|j| j.component.clone())
        .unwrap_or_else(|| "jre-legacy".to_string());
    let java_exe = java::ensure(app, paths, &component).await?;

    let dir = paths.cache().join("installers");
    let jar = dir.join(format!("{}-installer.jar", plan.id));
    if http::verify_all() {
        let _ = tokio::fs::remove_file(&jar).await;
    }
    http::download(&plan.url, &jar, None, None).await?;

    // The installers expect a launcher_profiles.json next to libraries/ and versions/
    let profiles = paths.root.join("launcher_profiles.json");
    if tokio::fs::metadata(&profiles).await.is_err() {
        tokio::fs::write(&profiles, br#"{"profiles":{}}"#).await?;
    }

    let mut cmd = tokio::process::Command::new(&java_exe);
    cmd.arg("-jar")
        .arg(&jar)
        .arg("--installClient")
        .arg(&paths.root)
        .current_dir(&dir)
        .stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);

    let output = cmd
        .output()
        .await
        .with_context(|| format!("running the {} installer", plan.label))?;

    if !output.status.success() || tokio::fs::metadata(paths.version_json(&plan.id)).await.is_err() {
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        let lines: Vec<&str> = text.lines().collect();
        let tail = lines[lines.len().saturating_sub(12)..].join("\n");
        return Err(anyhow!("The {} installer failed:\n{tail}", plan.label));
    }
    Ok(plan.id)
}

fn local_fallback(paths: &Paths, kind: &str, mc: &str) -> Option<String> {
    let names: Vec<String> = std::fs::read_dir(paths.versions())
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    let mut found: Vec<String> = if kind == "forge" {
        let prefix = format!("{mc}-forge-");
        names.into_iter().filter(|n| n.starts_with(&prefix)).collect()
    } else {
        let prefix = neoforge_prefix(mc);
        names
            .into_iter()
            .filter(|n| {
                n.strip_prefix("neoforge-")
                    .is_some_and(|v| v.starts_with(prefix.as_str()))
            })
            .collect()
    };
    found.sort_by_key(|n| version_key(n.rsplit('-').next().unwrap_or(n.as_str())));
    found.pop()
}
