mod backup;
mod error;
mod icons;
mod prefs;
mod platform;
mod detect;
mod config;
mod deploy;
mod dict;
mod download;
mod settings;
mod system;
mod tray;
mod update;
mod fonts;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 状态图标选了「自动」的话，开机跟着任务栏深浅换好
            icons::sync_on_startup(app.handle().clone());
            tray::setup(app.handle())?;
            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            detect::detect_rime,
            detect::uninstall_rime,
            detect::backup_leftover_config,
            config::save_theme_config,
            config::read_theme_config,
            config::open_config_dir,
            deploy::deploy_rime,
            dict::list_available_dicts,
            dict::list_installed_dicts,
            dict::install_dict,
            dict::toggle_dict,
            dict::remove_dict,
            dict::check_dict_updates,
            update::check_rime_update,
            download::download_rime,
            settings::read_input_settings,
            settings::save_input_settings,
            settings::read_schema_options,
            settings::save_schema_switch,
            settings::save_fuzzy,
            fonts::get_system_fonts,
            prefs::list_user_presets,
            prefs::save_user_preset,
            prefs::delete_user_preset,
            icons::read_schema_icons,
            icons::set_schema_icon,
            backup::export_settings,
            backup::inspect_backup,
            backup::import_settings,
            deploy::stop_rime_service,
            deploy::start_rime_service,
            settings::read_active_schema,
            settings::switch_active_schema,
            settings::switch_schema_and_restart,
            icons::clear_schema_icon,
            icons::clear_all_schema_icons,
            icons::list_builtin_icon_sets,
            icons::apply_builtin_icon_set,
            icons::sync_status_icons,
            prefs::read_icon_pref,
            system::get_autostart,
            system::set_autostart,
            system::open_system_setting,
            tray::show_main_window,
            tray::hide_tray_menu,
            tray::anchor_tray_menu,
            tray::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
