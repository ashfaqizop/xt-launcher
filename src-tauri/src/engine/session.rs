//! A game session: prepare, launch, stream logs, supervise, and the Tauri commands
//! the UI talks to.

use super::crash::{self, CrashInfo};
use super::launch::{self, Account, Options};
use super::paths::Paths;
use super::{install, java, loaders};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

const MAX_LOG_LINES: usize = 5000;

// ---------------------------------------------------------------------------
// State and payloads
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Session {
    busy: AtomicBool,
    killed: AtomicBool,
    pid: Mutex<Option<u32>>,
    logs: Mutex<VecDeque<LogLine>>,
}

#[derive(Clone, Serialize)]
pub struct LogLine {
    pub stream: String,
    pub line: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatePayload {
    state: String,
    message: Option<String>,
    code: Option<i32>,
    crash: Option<CrashInfo>,
}

fn emit_state(
    app: &AppHandle,
    state: &str,
    message: Option<String>,
    code: Option<i32>,
    crash: Option<CrashInfo>,
) {
    let _ = app.emit(
        "game-state",
        StatePayload {
            state: state.to_string(),
            message,
            code,
            crash,
        },
    );
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub name: String,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub user_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub profile_id: String,
    /// vanilla | fabric | quilt
    pub loader: String,
    /// A Minecraft version id, or "latest" for the newest release
    pub mc: String,
    pub account: AccountInfo,
    #[serde(default)]
    pub ram_mb: Option<u32>,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    #[serde(default)]
    pub java_path: Option<String>,
    /// default | high | low
    #[serde(default)]
    pub gpu: Option<String>,
}

// ---------------------------------------------------------------------------
// Launch flow
// ---------------------------------------------------------------------------

async fn pump<R>(app: AppHandle, reader: R, stream: &'static str)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let mut buf: Vec<u8> = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&buf).trim_end().to_string();
                let entry = LogLine {
                    stream: stream.to_string(),
                    line,
                };
                {
                    let session = app.state::<Session>();
                    let mut logs = session.logs.lock().unwrap();
                    if logs.len() >= MAX_LOG_LINES {
                        logs.pop_front();
                    }
                    logs.push_back(entry.clone());
                }
                let _ = app.emit("game-log", entry);
            }
        }
    }
}

#[cfg(windows)]
fn apply_gpu_preference(java: &Path, preference: Option<&str>) {
    use std::os::windows::process::CommandExt;

    let key = r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences";
    let exe = java.to_string_lossy().to_string();
    let value = match preference {
        Some("high") => Some("GpuPreference=2;"),
        Some("low") => Some("GpuPreference=1;"),
        _ => None,
    };

    let mut cmd = std::process::Command::new("reg");
    match value {
        Some(v) => {
            cmd.args(["add", key, "/v", exe.as_str(), "/t", "REG_SZ", "/d", v, "/f"]);
        }
        None => {
            cmd.args(["delete", key, "/v", exe.as_str(), "/f"]);
        }
    }
    cmd.creation_flags(0x0800_0000);
    let _ = cmd.output();
}

#[cfg(not(windows))]
fn apply_gpu_preference(_java: &Path, _preference: Option<&str>) {}

async fn run_launch(app: &AppHandle, req: LaunchRequest) -> Result<()> {
    let paths = Paths::new(app)?;
    let game_dir = paths.instance(&req.profile_id);
    for dir in [
        paths.libraries(),
        paths.assets(),
        paths.versions(),
        paths.runtimes(),
        paths.cache(),
        game_dir.clone(),
    ] {
        tokio::fs::create_dir_all(dir).await?;
    }

    emit_state(app, "installing", Some("Preparing Minecraft".to_string()), None, None);

    // Which version to run
    let mc_id = if req.mc == "latest" {
        install::load_manifest(&paths).await?.latest.release
    } else {
        req.mc.clone()
    };
    let version_id = match req.loader.as_str() {
        "vanilla" => mc_id.clone(),
        kind => loaders::install(&paths, kind, &mc_id).await?,
    };

    let (version, root_id) = install::resolve(&paths, &version_id).await?;
    install::prepare(app, &paths, &version, &root_id).await?;

    // Java
    let component = version
        .java_version
        .as_ref()
        .map(|j| j.component.clone())
        .unwrap_or_else(|| "jre-legacy".to_string());
    let custom_java = req
        .java_path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());
    let java_exe: PathBuf = match custom_java {
        Some(p) => PathBuf::from(p),
        None => java::ensure(app, &paths, &component).await?,
    };

    let natives = launch::extract_natives(&paths, &version).await?;

    let account = Account {
        uuid: req
            .account
            .uuid
            .clone()
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| launch::offline_uuid(&req.account.name)),
        access_token: req
            .account
            .access_token
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "0".to_string()),
        user_type: req
            .account
            .user_type
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "legacy".to_string()),
        name: req.account.name.clone(),
    };
    let options = Options {
        ram_mb: req.ram_mb.unwrap_or(4096).clamp(512, 65536),
        width: req.width.max(320),
        height: req.height.max(240),
        fullscreen: req.fullscreen,
    };
    let args = launch::build_args(&paths, &version, &root_id, &natives, &game_dir, &account, &options)?;

    apply_gpu_preference(&java_exe, req.gpu.as_deref());

    // Start the game
    let started = SystemTime::now();
    let mut cmd = tokio::process::Command::new(&java_exe);
    cmd.args(&args)
        .current_dir(&game_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // no console window
    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting Java at {}", java_exe.display()))?;

    {
        let session = app.state::<Session>();
        *session.pid.lock().unwrap() = child.id();
        session.logs.lock().unwrap().clear();
        session.killed.store(false, Ordering::SeqCst);
    }
    emit_state(app, "running", None, None, None);

    let out_task = child
        .stdout
        .take()
        .map(|r| tokio::spawn(pump(app.clone(), r, "out")));
    let err_task = child
        .stderr
        .take()
        .map(|r| tokio::spawn(pump(app.clone(), r, "err")));

    let status = child.wait().await?;
    if let Some(task) = out_task {
        let _ = task.await;
    }
    if let Some(task) = err_task {
        let _ = task.await;
    }

    // Wrap up
    let session = app.state::<Session>();
    *session.pid.lock().unwrap() = None;
    let killed = session.killed.load(Ordering::SeqCst);
    let code = status.code();

    let crash = if !killed && code != Some(0) {
        let tail = {
            let logs = session.logs.lock().unwrap();
            let lines: Vec<&str> = logs.iter().map(|l| l.line.as_str()).collect();
            let start = lines.len().saturating_sub(400);
            lines[start..].join("\n")
        };
        crash::analyze(&game_dir, started, &tail).or_else(|| {
            Some(CrashInfo {
                title: "Minecraft closed unexpectedly".to_string(),
                hint: "Open the game log to see what happened.".to_string(),
                report: None,
            })
        })
    } else {
        None
    };

    emit_state(app, "exited", None, code, crash);
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn launch_game(app: AppHandle, req: LaunchRequest) -> Result<(), String> {
    {
        let session = app.state::<Session>();
        if session.busy.swap(true, Ordering::SeqCst) {
            return Err("The game is already starting or running".to_string());
        }
    }

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_launch(&app, req).await {
            emit_state(&app, "error", Some(format!("{e:#}")), None, None);
        }
        app.state::<Session>().busy.store(false, Ordering::SeqCst);
    });
    Ok(())
}

#[tauri::command]
pub fn kill_game(session: State<'_, Session>) -> Result<(), String> {
    let pid = *session.pid.lock().unwrap();
    let Some(pid) = pid else {
        return Ok(());
    };
    session.killed.store(true, Ordering::SeqCst);
    kill_process_tree(pid)
}

#[cfg(windows)]
fn kill_process_tree(pid: u32) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let pid_text = pid.to_string();
    std::process::Command::new("taskkill")
        .args(["/PID", pid_text.as_str(), "/T", "/F"])
        .creation_flags(0x0800_0000)
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn kill_process_tree(_pid: u32) -> Result<(), String> {
    Err("Stopping the game is only supported on Windows".to_string())
}

#[tauri::command]
pub fn get_logs(session: State<'_, Session>) -> Vec<LogLine> {
    session.logs.lock().unwrap().iter().cloned().collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub data_dir: String,
}

#[tauri::command]
pub fn engine_info(app: AppHandle) -> Result<EngineInfo, String> {
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    Ok(EngineInfo {
        data_dir: paths.root.to_string_lossy().into_owned(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionItem {
    pub id: String,
    pub kind: String,
    pub release_time: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<VersionItem>,
}

#[tauri::command]
pub async fn list_versions(app: AppHandle, snapshots: bool, old: bool) -> Result<VersionList, String> {
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    let manifest = install::load_manifest(&paths)
        .await
        .map_err(|e| format!("{e:#}"))?;

    let versions = manifest
        .versions
        .into_iter()
        .filter(|v| match v.kind.as_str() {
            "release" => true,
            "snapshot" => snapshots,
            "old_beta" | "old_alpha" => old,
            _ => false,
        })
        .map(|v| VersionItem {
            id: v.id,
            kind: v.kind,
            release_time: v.release_time,
        })
        .collect();

    Ok(VersionList {
        latest_release: manifest.latest.release,
        latest_snapshot: manifest.latest.snapshot,
        versions,
    })
}

/// Open a launcher folder in Explorer. `kind` is "root", "game" or "mods".
#[tauri::command]
pub fn open_dir(app: AppHandle, kind: String, profile_id: Option<String>) -> Result<(), String> {
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    let profile = profile_id.unwrap_or_else(|| "default".to_string());
    let dir = match kind.as_str() {
        "root" => paths.root.clone(),
        "game" => paths.instance(&profile),
        "mods" => paths.instance(&profile).join("mods"),
        other => return Err(format!("unknown folder '{other}'")),
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
