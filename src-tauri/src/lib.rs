// Tauri 앱의 composition root와 공개 모듈 경계를 정의한다.
pub mod adapters;
pub mod application;
pub(crate) mod cloud;
pub(crate) mod codex_status;
pub(crate) mod config;
pub(crate) mod debuglog;
pub mod domain;
pub(crate) mod environment;
pub(crate) mod resume;
pub(crate) mod scanner;
pub(crate) mod session_state;
pub(crate) mod summary;
pub(crate) mod terminal;
pub mod types;
pub(crate) mod update;

#[cfg(test)]
mod integration_tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use crate::adapters::inbound::tauri_commands::{
        archive_session, check_environment_cmd, check_update_cmd, checkin_session,
        checkout_session, connect_google_drive_cmd, delete_session, delete_sessions,
        detect_google_drive_cmd, generate_summary_cmd, get_codex_status_cmd, get_config_cmd,
        get_debug_log_cmd, list_sessions, open_debug_log_folder_cmd, resume_session,
        save_session_meta, save_settings, set_cloud_folder, start_auto_summary, unarchive_session,
        upload_to_cloud,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_config_cmd,
            save_session_meta,
            delete_session,
            delete_sessions,
            archive_session,
            unarchive_session,
            save_settings,
            set_cloud_folder,
            upload_to_cloud,
            checkout_session,
            checkin_session,
            resume_session,
            check_environment_cmd,
            get_codex_status_cmd,
            check_update_cmd,
            generate_summary_cmd,
            start_auto_summary,
            detect_google_drive_cmd,
            connect_google_drive_cmd,
            get_debug_log_cmd,
            open_debug_log_folder_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
