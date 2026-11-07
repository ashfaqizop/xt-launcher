mod engine;

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
        .plugin(tauri_plugin_opener::init())
        .manage(engine::session::Session::default())
        .manage(engine::auth::AuthState::default())
        .invoke_handler(tauri::generate_handler![
            finish_splash,
            engine::session::launch_game,
            engine::session::kill_game,
            engine::session::cancel_launch,
            engine::session::get_logs,
            engine::session::engine_info,
            engine::session::list_versions,
            engine::session::open_dir,
            engine::auth::ms_login,
            engine::auth::auth_cancel,
            engine::auth::ms_has_builtin_client,
            engine::auth::ely_login,
            engine::auth::forget_account,
            engine::system::system_info,
            engine::catalog::loader_catalog,
            engine::system::memory_status,
            engine::screenshots::list_screenshots,
            engine::screenshots::read_screenshot,
            engine::screenshots::open_screenshot,
            engine::screenshots::delete_screenshot,
            engine::storage::storage_report,
            engine::storage::storage_clean,
            engine::storage::storage_delete_instance,
            engine::storage::delete_profile_files,
            engine::modman::list_mods,
            engine::modman::set_mod_enabled,
            engine::modman::remove_mod,
            engine::modman::add_mod_files,
            engine::modman::modrinth_search,
            engine::modman::modrinth_identify,
            engine::modman::modrinth_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running XT Launcher");
}
