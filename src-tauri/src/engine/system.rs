//! What this PC actually has: installed memory, graphics cards and Java installations.
//! Used by Settings so the choices reflect the machine instead of generic presets.

use super::paths::Paths;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    /// Built into the CPU (power saving) rather than a separate card (high performance)
    pub integrated: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstall {
    /// Full path to java.exe
    pub path: String,
    pub version: String,
    pub major: u32,
    pub vendor: String,
    /// Downloaded and managed by XT Launcher
    pub managed: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub total_memory_mb: u64,
    pub gpus: Vec<Gpu>,
    pub javas: Vec<JavaInstall>,
}

// ---------------------------------------------------------------------------
// Memory and graphics (one PowerShell CIM query, run without a console window)
// ---------------------------------------------------------------------------

fn is_integrated(name: &str) -> bool {
    let n = name.to_lowercase();
    let intel_discrete = n.contains("arc(tm) a") || n.contains("arc a") || n.contains("arc(tm) b") || n.contains("arc b");
    let amd_integrated = n.contains("radeon(tm) graphics")
        || n == "amd radeon graphics"
        || n.contains("radeon vega")
        || n.contains("radeon(tm) vega")
        || n.contains("radeon 780m")
        || n.contains("radeon 760m")
        || n.contains("radeon 880m")
        || n.contains("radeon 890m");
    (n.contains("intel") && !intel_discrete) || amd_integrated
}

fn is_virtual_adapter(name: &str) -> bool {
    let n = name.to_lowercase();
    ["basic display", "basic render", "remote", "virtual", "parsec", "indirect display", "dummy"]
        .iter()
        .any(|k| n.contains(k))
}

#[cfg(windows)]
async fn query_hardware() -> (u64, Vec<Gpu>) {
    use std::process::Stdio;

    let script = "$g = @(Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name }); \
                  $m = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory; \
                  @{ gpus = $g; mem = $m } | ConvertTo-Json -Compress";
    let mut cmd = tokio::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    cmd.creation_flags(0x0800_0000);

    let Ok(output) = cmd.output().await else {
        return (0, Vec::new());
    };
    let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) else {
        return (0, Vec::new());
    };

    let memory_mb = json.get("mem").and_then(|m| m.as_u64()).unwrap_or(0) / (1024 * 1024);
    let names: Vec<String> = match json.get("gpus") {
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(one)) => vec![one.clone()],
        _ => Vec::new(),
    };

    let mut seen = HashSet::new();
    let gpus = names
        .into_iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty() && !is_virtual_adapter(n) && seen.insert(n.clone()))
        .map(|name| Gpu {
            integrated: is_integrated(&name),
            name,
        })
        .collect();
    (memory_mb, gpus)
}

#[cfg(not(windows))]
async fn query_hardware() -> (u64, Vec<Gpu>) {
    (0, Vec::new())
}

// ---------------------------------------------------------------------------
// Java installations
// ---------------------------------------------------------------------------

/// Read `<home>/release` (JAVA_VERSION="25.0.3", IMPLEMENTOR="Eclipse Adoptium").
fn read_release(home: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .map(|v| v.trim().trim_matches('"').to_string())
    };
    let version = field("JAVA_VERSION")?;
    let vendor = field("IMPLEMENTOR").unwrap_or_else(|| "Java".to_string());
    Some((version, vendor))
}

fn major_of(version: &str) -> u32 {
    let mut parts = version.split(|c: char| !c.is_ascii_digit());
    let first: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    if first == 1 {
        parts.next().and_then(|p| p.parse().ok()).unwrap_or(first)
    } else {
        first
    }
}

/// Every directory under `dir` (one level) that looks like a Java home.
fn homes_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("bin").join("java.exe").exists())
        .collect()
}

fn find_javas(paths: &Paths) -> Vec<JavaInstall> {
    let mut candidates: Vec<(PathBuf, bool)> = Vec::new();

    // Runtimes XT Launcher downloaded
    for home in homes_in(&paths.runtimes()) {
        candidates.push((home, true));
    }

    // JAVA_HOME
    if let Ok(home) = std::env::var("JAVA_HOME") {
        candidates.push((PathBuf::from(home), false));
    }

    // Common install locations
    let mut bases: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Ok(root) = std::env::var(var) {
            bases.push(PathBuf::from(root));
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        bases.push(PathBuf::from(local).join("Programs"));
    }
    let vendors = [
        "Java",
        "Eclipse Adoptium",
        "Eclipse Foundation",
        "Temurin",
        "Microsoft",
        "Zulu",
        "BellSoft",
        "Amazon Corretto",
        "Semeru",
        "OpenJDK",
        "GraalVM",
    ];
    for base in &bases {
        for vendor in vendors {
            for home in homes_in(&base.join(vendor)) {
                candidates.push((home, false));
            }
        }
    }

    // The official Minecraft launcher's runtimes
    if let Ok(appdata) = std::env::var("APPDATA") {
        let runtime = PathBuf::from(appdata).join(".minecraft").join("runtime");
        for component in homes_in_two_levels(&runtime) {
            candidates.push((component, false));
        }
    }

    let mut seen = HashSet::new();
    let mut found: Vec<JavaInstall> = Vec::new();
    for (home, managed) in candidates {
        let exe = home.join("bin").join("java.exe");
        if !exe.exists() {
            continue;
        }
        let key = std::fs::canonicalize(&exe)
            .unwrap_or_else(|_| exe.clone())
            .to_string_lossy()
            .to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        let Some((version, vendor)) = read_release(&home) else {
            continue;
        };
        found.push(JavaInstall {
            path: exe.to_string_lossy().into_owned(),
            major: major_of(&version),
            version,
            vendor,
            managed,
        });
    }
    found.sort_by(|a, b| b.major.cmp(&a.major).then(a.path.cmp(&b.path)));
    found
}

/// `.minecraft/runtime/<component>/<platform>/<component>` style layouts.
fn homes_in_two_levels(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(components) = std::fs::read_dir(root) else {
        return out;
    };
    for component in components.filter_map(|e| e.ok()).map(|e| e.path()) {
        let Ok(platforms) = std::fs::read_dir(&component) else {
            continue;
        };
        for platform in platforms.filter_map(|e| e.ok()).map(|e| e.path()) {
            out.extend(homes_in(&platform));
        }
    }
    out
}

#[tauri::command]
pub async fn system_info(app: AppHandle) -> Result<SystemInfo, String> {
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    let javas_task = tokio::task::spawn_blocking(move || find_javas(&paths));
    let (total_memory_mb, gpus) = query_hardware().await;
    let javas = javas_task.await.unwrap_or_default();
    Ok(SystemInfo {
        total_memory_mb,
        gpus,
        javas,
    })
}

// ---------------------------------------------------------------------------
// Live memory (for the memory slider's hint)
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatus {
    pub total_mb: u64,
    /// Available right now without swapping anything out
    pub free_mb: u64,
}

#[cfg(windows)]
fn read_memory() -> Option<(u64, u64)> {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    }

    let mut status = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    // SAFETY: `status` is a correctly sized, initialised MEMORYSTATUSEX with its length set
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        return None;
    }
    Some((status.total_phys / (1024 * 1024), status.avail_phys / (1024 * 1024)))
}

#[cfg(not(windows))]
fn read_memory() -> Option<(u64, u64)> {
    None
}

/// Installed and currently free memory, in MB.
#[tauri::command]
pub fn memory_status() -> Result<MemoryStatus, String> {
    read_memory()
        .map(|(total_mb, free_mb)| MemoryStatus { total_mb, free_mb })
        .ok_or_else(|| "memory status is not available".to_string())
}
