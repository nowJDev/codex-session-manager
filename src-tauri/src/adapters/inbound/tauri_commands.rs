// Tauri IPC command를 애플리케이션 use case로 위임한다.
use crate::adapters::inbound::auto_summary_worker;
use crate::adapters::outbound::session_ports::DefaultSessionPorts;
use crate::adapters::outbound::system_ports::DefaultSystemPorts;
use crate::application::ports::{
    CodexStatus, DebugLogInfo, DriveDetectResult, EnvironmentReport, UpdateInfo,
};
use crate::application::session_service::SessionService;
use crate::application::system_service::SystemService;
use crate::types::{Config, DeleteSessionTarget, Session, SessionMeta, Settings};

fn to_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn session_service() -> SessionService<DefaultSessionPorts> {
    SessionService::new(DefaultSessionPorts)
}

fn system_service() -> SystemService<DefaultSystemPorts> {
    SystemService::new(DefaultSystemPorts)
}

/// 빈 description 세션을 1개씩 순차 자동 요약하는 백그라운드 워커.
/// 이미 실행 중이면 no-op.
#[tauri::command]
pub fn start_auto_summary(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri::Emitter;
    Ok(auto_summary_worker::start_auto_summary(
        DefaultSessionPorts,
        move |id| {
            let _ = app.emit("auto-summary-progress", id);
        },
    ))
}

#[tauri::command]
pub fn list_sessions() -> Result<Vec<Session>, String> {
    session_service().list_sessions().map_err(to_str)
}

#[tauri::command]
pub fn get_config_cmd() -> Config {
    session_service().get_config()
}

#[tauri::command]
pub fn save_session_meta(session_id: String, patch: SessionMeta) -> Result<(), String> {
    session_service()
        .save_session_meta(&session_id, patch)
        .map_err(to_str)
}

#[tauri::command]
pub async fn delete_session(session_id: String, file_path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || session_service().delete_session(&session_id, &file_path))
        .await
        .map_err(to_str)?
        .map_err(to_str)
}

#[tauri::command]
pub async fn delete_sessions(targets: Vec<DeleteSessionTarget>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || session_service().delete_sessions(targets))
        .await
        .map_err(to_str)?
        .map_err(to_str)
}

#[tauri::command]
pub fn archive_session(session_id: String) -> Result<(), String> {
    session_service()
        .archive_session(&session_id)
        .map_err(to_str)
}

#[tauri::command]
pub fn unarchive_session(session_id: String) -> Result<(), String> {
    session_service()
        .unarchive_session(&session_id)
        .map_err(to_str)
}

#[tauri::command]
pub fn save_settings(patch: Settings) -> Result<(), String> {
    session_service().save_settings(patch).map_err(to_str)
}

#[tauri::command]
pub fn set_cloud_folder(root: String) -> Result<String, String> {
    session_service()
        .set_cloud_folder(&root)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(to_str)
}

#[tauri::command]
pub fn get_debug_log_cmd() -> DebugLogInfo {
    system_service().debug_log_info()
}

#[tauri::command]
pub fn open_debug_log_folder_cmd() -> Result<(), String> {
    system_service().open_debug_log_folder().map_err(to_str)
}

#[tauri::command]
pub fn detect_google_drive_cmd() -> DriveDetectResult {
    system_service().detect_google_drive()
}

#[tauri::command]
pub fn connect_google_drive_cmd() -> Result<String, String> {
    system_service().connect_google_drive().map_err(to_str)
}

#[tauri::command]
pub fn upload_to_cloud(session: Session) -> Result<(), String> {
    session_service().upload_to_cloud(&session).map_err(to_str)
}

#[tauri::command]
pub fn checkout_session(session: Session) -> Result<String, String> {
    session_service().checkout_session(&session).map_err(to_str)
}

#[tauri::command]
pub fn checkin_session(session: Session) -> Result<(), String> {
    session_service().checkin_session(&session).map_err(to_str)
}

#[tauri::command]
pub fn resume_session(session_id: String, cwd: Option<String>) -> Result<(), String> {
    session_service()
        .resume_session(&session_id, cwd.as_deref())
        .map_err(to_str)
}

#[tauri::command]
pub fn check_environment_cmd() -> EnvironmentReport {
    system_service().check_environment()
}

#[tauri::command]
pub fn get_codex_status_cmd() -> CodexStatus {
    system_service().get_codex_status()
}

#[tauri::command]
pub async fn check_update_cmd() -> Result<UpdateInfo, String> {
    system_service().check_update().await.map_err(to_str)
}

#[tauri::command]
pub async fn generate_summary_cmd(session_id: String, file_path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || session_service().generate_summary(&session_id, &file_path))
        .await
        .map_err(to_str)?
        .map_err(to_str)
}
