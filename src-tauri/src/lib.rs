pub mod models;
pub mod store;
pub mod commands;


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::get_project,
            commands::projects::get_all_projects,
            commands::projects::delete_project,
            commands::system::list_installed_apps,
            commands::system::delete_directory,
            commands::projects::open_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
