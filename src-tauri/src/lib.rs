pub mod auth;
pub mod commands;
pub mod diff;
pub mod github;
pub mod model;
pub mod sync;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::auth_with_gh,
            commands::auth_with_pat,
            commands::auth_load_saved,
            commands::auth_device_start,
            commands::auth_device_poll,
            commands::logout,
            commands::list_repos,
            commands::list_owners,
            commands::list_repos_for_owner,
            commands::refresh_repos_for_owner,
            commands::list_org_teams,
            commands::list_team_repos,
            commands::audit,
            commands::apply_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
