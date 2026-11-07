//! A profile's screenshots: list, read (for thumbnails and copying), open and delete.
//! Everything is confined to `<instance>/screenshots`.

use super::paths::Paths;
use super::win;
use serde::Serialize;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use tauri::{ipc::Response, AppHandle};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub name: String,
    /// Last modified, milliseconds since the Unix epoch
    pub modified: u64,
    pub size: u64,
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
}

fn folder(app: &AppHandle, profile_id: &str) -> Result<PathBuf, String> {
    if !safe_id(profile_id) {
        return Err("invalid profile".to_string());
    }
    let paths = Paths::new(app).map_err(|e| e.to_string())?;
    Ok(paths.instance(profile_id).join("screenshots"))
}

/// The full path of one screenshot, refusing anything that isn't a plain image file name.
fn file_path(app: &AppHandle, profile_id: &str, name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.contains(|c: char| c == '/' || c == '\\') || name == ".." || !is_image(name) {
        return Err("invalid file name".to_string());
    }
    Ok(folder(app, profile_id)?.join(name))
}

/// Newest first. `limit` caps how many are returned.
#[tauri::command]
pub async fn list_screenshots(
    app: AppHandle,
    profile_id: String,
    limit: Option<usize>,
) -> Result<Vec<Screenshot>, String> {
    let dir = folder(&app, &profile_id)?;
    tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut shots: Vec<Screenshot> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                if !is_image(&name) {
                    return None;
                }
                let meta = entry.metadata().ok()?;
                if !meta.is_file() {
                    return None;
                }
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                Some(Screenshot {
                    name,
                    modified,
                    size: meta.len(),
                })
            })
            .collect();
        shots.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| b.name.cmp(&a.name)));
        if let Some(n) = limit {
            shots.truncate(n);
        }
        shots
    })
    .await
    .map_err(|e| e.to_string())
}

/// The raw bytes of one screenshot (sent to the UI as a binary response).
#[tauri::command]
pub async fn read_screenshot(app: AppHandle, profile_id: String, name: String) -> Result<Response, String> {
    let path = file_path(&app, &profile_id, &name)?;
    let bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

/// Open a screenshot in the default image viewer.
#[tauri::command]
pub fn open_screenshot(app: AppHandle, profile_id: String, name: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = file_path(&app, &profile_id, &name)?;
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Move a screenshot to the Recycle Bin (it can be restored from there). If that isn't
/// possible the file is deleted outright.
#[tauri::command]
pub async fn delete_screenshot(app: AppHandle, profile_id: String, name: String) -> Result<(), String> {
    let path = file_path(&app, &profile_id, &name)?;
    if !path.is_file() {
        return Ok(());
    }

    let target = path.clone();
    let recycled = tokio::task::spawn_blocking(move || win::recycle(&target))
        .await
        .unwrap_or(false);
    if recycled && !path.exists() {
        return Ok(());
    }

    tokio::fs::remove_file(&path).await.map_err(|e| e.to_string())
}
