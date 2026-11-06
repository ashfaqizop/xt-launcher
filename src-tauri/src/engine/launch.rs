//! Building the Java command line for a resolved version.

use super::mojang::{features, rules_allow, Arg, ArgValue, VersionJson};
use super::paths::Paths;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Account {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
}

pub struct Options {
    pub ram_mb: u32,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    /// Extra JVM arguments (e.g. authlib-injector), placed before everything else
    pub extra_jvm: Vec<String>,
    /// Use ZGC instead of G1
    pub zgc: bool,
    /// Major version of the Java runtime that will run the game
    pub java_major: u32,
}

/// Garbage collector flags. ZGC (generational on Java 21+) keeps frame times steady;
/// old Java versions without a usable ZGC, and everything else, use tuned G1.
fn gc_flags(zgc: bool, java_major: u32) -> Vec<String> {
    let mut flags: Vec<&str> = if zgc && java_major >= 17 {
        vec!["-XX:+UseZGC", "-XX:+PerfDisableSharedMem"]
    } else {
        vec![
            "-XX:+UseG1GC",
            "-XX:+ParallelRefProcEnabled",
            "-XX:MaxGCPauseMillis=200",
        ]
    };
    // Generational ZGC is opt-in on Java 21-23 and the only mode from Java 24
    if zgc && (21..24).contains(&java_major) {
        flags.push("-XX:+ZGenerational");
    }
    flags.into_iter().map(String::from).collect()
}

/// The offline-mode UUID the game itself derives: MD5("OfflinePlayer:<name>") as a v3 UUID.
pub fn offline_uuid(name: &str) -> String {
    let mut bytes = md5::compute(format!("OfflinePlayer:{name}")).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    hex::encode(bytes)
}

fn substitute(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("${{{key}}}"), value);
    }
    out
}

fn flatten(args: &[Arg], feats: &HashMap<String, bool>, vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            Arg::Plain(s) => out.push(substitute(s, vars)),
            Arg::Ruled { rules, value } => {
                if !rules_allow(rules, feats) {
                    continue;
                }
                match value {
                    ArgValue::One(s) => out.push(substitute(s, vars)),
                    ArgValue::Many(list) => out.extend(list.iter().map(|s| substitute(s, vars))),
                }
            }
        }
    }
    out
}

fn extract_zip(jar: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(jar).with_context(|| format!("opening {}", jar.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if name.starts_with("META-INF") {
            continue;
        }
        let out = dest.join(&name);
        if out.exists() {
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut target = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut target)?;
    }
    Ok(())
}

/// Unpack old-style native libraries (pre 1.19) into this version's natives directory.
pub async fn extract_natives(paths: &Paths, v: &VersionJson) -> Result<PathBuf> {
    let dir = paths.natives(&v.id);
    tokio::fs::create_dir_all(&dir).await?;

    let feats = features(true);
    let jars: Vec<PathBuf> = v
        .libraries
        .iter()
        .filter(|lib| rules_allow(&lib.rules, &feats))
        .filter_map(|lib| lib.native())
        .map(|dl| paths.libraries().join(dl.path))
        .collect();

    let target = dir.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        for jar in jars {
            extract_zip(&jar, &target)?;
        }
        Ok(())
    })
    .await??;
    Ok(dir)
}

/// Everything after `java.exe`: JVM arguments, main class, game arguments.
pub fn build_args(
    paths: &Paths,
    v: &VersionJson,
    root_id: &str,
    natives: &Path,
    game_dir: &Path,
    account: &Account,
    opts: &Options,
) -> Result<Vec<String>> {
    let feats = features(true);

    let mut classpath_entries: Vec<String> = Vec::new();
    for lib in &v.libraries {
        if !rules_allow(&lib.rules, &feats) {
            continue;
        }
        if let Some(dl) = lib.artifact() {
            classpath_entries.push(paths.libraries().join(&dl.path).to_string_lossy().into_owned());
        }
    }
    classpath_entries.push(paths.version_jar(root_id).to_string_lossy().into_owned());
    let classpath = classpath_entries.join(";");

    let assets_name = v
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .or_else(|| v.assets.clone())
        .unwrap_or_else(|| "legacy".to_string());

    let path_str = |p: &Path| p.to_string_lossy().into_owned();
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", account.name.clone());
    vars.insert("auth_uuid", account.uuid.clone());
    vars.insert("auth_access_token", account.access_token.clone());
    vars.insert("auth_session", format!("token:{}:{}", account.access_token, account.uuid));
    vars.insert("user_type", account.user_type.clone());
    vars.insert("user_properties", "{}".to_string());
    vars.insert("clientid", String::new());
    vars.insert("auth_xuid", String::new());
    vars.insert("version_name", v.id.clone());
    vars.insert("version_type", v.kind.clone().unwrap_or_else(|| "release".to_string()));
    vars.insert("game_directory", path_str(game_dir));
    vars.insert("assets_root", path_str(&paths.assets()));
    vars.insert("assets_index_name", assets_name.clone());
    vars.insert("game_assets", path_str(&paths.assets().join("virtual").join("legacy")));
    vars.insert("natives_directory", path_str(natives));
    vars.insert("library_directory", path_str(&paths.libraries()));
    vars.insert("classpath", classpath.clone());
    vars.insert("classpath_separator", ";".to_string());
    vars.insert("launcher_name", "XT Launcher".to_string());
    vars.insert("launcher_version", env!("CARGO_PKG_VERSION").to_string());
    vars.insert("resolution_width", opts.width.to_string());
    vars.insert("resolution_height", opts.height.to_string());

    let main_class = v
        .main_class
        .clone()
        .ok_or_else(|| anyhow!("version {} has no main class", v.id))?;

    // JVM arguments
    let mut jvm: Vec<String> = opts.extra_jvm.clone();
    jvm.push(format!("-Xmx{}M", opts.ram_mb));
    jvm.extend(gc_flags(opts.zgc, opts.java_major));
    if let Some(arguments) = &v.arguments {
        jvm.extend(flatten(&arguments.jvm, &feats, &vars));
    }
    if !jvm.iter().any(|a| a.starts_with("-Djava.library.path")) {
        jvm.push(format!("-Djava.library.path={}", path_str(natives)));
    }
    if !jvm.iter().any(|a| a == "-cp" || a == "-classpath") {
        jvm.push("-cp".to_string());
        jvm.push(classpath);
    }

    // Game arguments
    let mut game: Vec<String> = match (&v.arguments, &v.minecraft_arguments) {
        (Some(arguments), _) => flatten(&arguments.game, &feats, &vars),
        (None, Some(legacy)) => legacy
            .split_whitespace()
            .map(|s| substitute(s, &vars))
            .collect(),
        (None, None) => Vec::new(),
    };
    if v.arguments.is_none() && !game.iter().any(|a| a == "--width") {
        game.push("--width".to_string());
        game.push(opts.width.to_string());
        game.push("--height".to_string());
        game.push(opts.height.to_string());
    }
    if opts.fullscreen {
        game.push("--fullscreen".to_string());
    }

    let mut args = jvm;
    args.push(main_class);
    args.extend(game);
    Ok(args)
}
