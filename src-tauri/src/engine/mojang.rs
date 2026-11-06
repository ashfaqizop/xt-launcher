//! Mojang / loader version metadata: types, rule evaluation and inheritance merging.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const RESOURCES_URL: &str = "https://resources.download.minecraft.net";
const DEFAULT_MAVEN: &str = "https://libraries.minecraft.net/";

// ---------------------------------------------------------------------------
// Version manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(rename = "releaseTime", default)]
    pub release_time: String,
}

// ---------------------------------------------------------------------------
// Version JSON
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone, Default)]
pub struct VersionJson {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "inheritsFrom", default)]
    pub inherits_from: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(rename = "mainClass", default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments", default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(rename = "assetIndex", default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<VersionDownloads>,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Arg>,
    #[serde(default)]
    pub jvm: Vec<Arg>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Arg {
    Plain(String),
    Ruled { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VersionDownloads {
    #[serde(default)]
    pub client: Option<Artifact>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion", default)]
    pub major_version: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    /// Maven base url (Fabric / Quilt style entries without a `downloads` block)
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

/// A resolved file to fetch: relative path under `libraries/`, url, and optional checks.
#[derive(Debug, Clone)]
pub struct Dl {
    pub path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// `group:artifact:version[:classifier]` -> `group/as/path/artifact/version/artifact-version[-classifier].jar`
pub fn maven_path(name: &str) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next();
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!(
        "{}/{artifact}/{version}/{file}",
        group.replace('.', "/")
    ))
}

impl Library {
    /// The jar that belongs on the classpath, if this library has one.
    pub fn artifact(&self) -> Option<Dl> {
        match &self.downloads {
            Some(d) => {
                let a = d.artifact.as_ref()?;
                let path = a.path.clone().or_else(|| maven_path(&self.name))?;
                Some(Dl {
                    path,
                    url: a.url.clone(),
                    sha1: a.sha1.clone(),
                    size: a.size,
                })
            }
            None => {
                let path = maven_path(&self.name)?;
                let mut base = self
                    .url
                    .clone()
                    .unwrap_or_else(|| DEFAULT_MAVEN.to_string());
                if !base.ends_with('/') {
                    base.push('/');
                }
                Some(Dl {
                    url: format!("{base}{path}"),
                    path,
                    sha1: self.sha1.clone(),
                    size: self.size,
                })
            }
        }
    }

    /// Old-style native classifier jar (extracted into the natives directory).
    pub fn native(&self) -> Option<Dl> {
        let key = self.natives.as_ref()?.get("windows")?.replace("${arch}", "64");
        let a = self.downloads.as_ref()?.classifiers.as_ref()?.get(&key)?;
        Some(Dl {
            path: a.path.clone()?,
            url: a.url.clone(),
            sha1: a.sha1.clone(),
            size: a.size,
        })
    }

    /// Identity without the version, used to let child libraries override parent ones.
    fn key(&self) -> String {
        let mut parts = self.name.split(':');
        let group = parts.next().unwrap_or("");
        let artifact = parts.next().unwrap_or("");
        let _version = parts.next();
        let classifier = parts.next().unwrap_or("");
        format!("{group}:{artifact}:{classifier}")
    }
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

/// Features the launcher reports to version rules.
pub fn features(custom_resolution: bool) -> HashMap<String, bool> {
    let mut m = HashMap::new();
    m.insert("has_custom_resolution".to_string(), custom_resolution);
    m
}

fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86"
    }
}

fn rule_matches(rule: &Rule, feats: &HashMap<String, bool>) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != "windows" {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            if arch != current_arch() {
                return false;
            }
        }
    }
    if let Some(required) = &rule.features {
        for (k, v) in required {
            if feats.get(k).copied().unwrap_or(false) != *v {
                return false;
            }
        }
    }
    true
}

pub fn rules_allow(rules: &[Rule], feats: &HashMap<String, bool>) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule_matches(rule, feats) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

// ---------------------------------------------------------------------------
// Inheritance (Fabric / Quilt profiles inherit a vanilla version)
// ---------------------------------------------------------------------------

/// Merge `child` on top of `parent`: child wins for scalars, arguments are appended,
/// and child libraries come first so they override same-named parent libraries.
pub fn merge(parent: VersionJson, child: VersionJson) -> VersionJson {
    let mut seen: HashSet<String> = HashSet::new();
    let mut libraries = Vec::new();
    for lib in child.libraries.into_iter().chain(parent.libraries) {
        if seen.insert(lib.key()) {
            libraries.push(lib);
        }
    }

    let arguments = match (parent.arguments, child.arguments) {
        (Some(p), Some(c)) => {
            let mut game = p.game;
            game.extend(c.game);
            let mut jvm = p.jvm;
            jvm.extend(c.jvm);
            Some(Arguments { game, jvm })
        }
        (p, c) => c.or(p),
    };

    VersionJson {
        id: child.id,
        inherits_from: None,
        kind: child.kind.or(parent.kind),
        main_class: child.main_class.or(parent.main_class),
        arguments,
        minecraft_arguments: child.minecraft_arguments.or(parent.minecraft_arguments),
        libraries,
        asset_index: child.asset_index.or(parent.asset_index),
        assets: child.assets.or(parent.assets),
        downloads: child.downloads.or(parent.downloads),
        java_version: child.java_version.or(parent.java_version),
    }
}
