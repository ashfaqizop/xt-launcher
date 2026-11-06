use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// On-disk layout. Libraries, assets and Java runtimes are shared between profiles;
/// every profile gets its own game directory under `instances/`.
#[derive(Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(app: &AppHandle) -> anyhow::Result<Self> {
        let root = app
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("cannot resolve app data dir: {e}"))?;
        Ok(Self { root })
    }

    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }
    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }
    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }
    pub fn instances(&self) -> PathBuf {
        self.root.join("instances")
    }
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
    pub fn natives(&self, version_id: &str) -> PathBuf {
        self.root.join("natives").join(version_id)
    }
    pub fn instance(&self, profile_id: &str) -> PathBuf {
        self.instances().join(profile_id)
    }
    pub fn version_json(&self, id: &str) -> PathBuf {
        self.versions().join(id).join(format!("{id}.json"))
    }
    pub fn version_jar(&self, id: &str) -> PathBuf {
        self.versions().join(id).join(format!("{id}.jar"))
    }
}
