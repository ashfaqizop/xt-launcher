use tauri::Manager;

/// Called by the splash screen once the update check is finished:
/// shows the main window and closes the splash.
#[tauri::command]
fn finish_splash(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        main.show().map_err(|e| e.to_string())?;
        let _ = main.set_focus();
    }
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![finish_splash])
        .run(tauri::generate_context!())
        .expect("error while running XT Launcher");
}
