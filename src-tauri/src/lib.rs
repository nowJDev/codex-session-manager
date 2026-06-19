pub mod cloud;
pub mod config;
pub mod debuglog;
pub mod environment;
pub mod resume;
pub mod scanner;
pub mod summary;
pub mod terminal;
pub mod types;
pub mod update;

use crate::config::{
    delete_session_meta as cfg_delete_meta, load_config, update_settings, upsert_session_meta,
};
use crate::types::{Config, Session, SessionMeta, Settings};

fn to_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

use std::sync::atomic::{AtomicBool, Ordering};
static AUTO_SUMMARY_RUNNING: AtomicBool = AtomicBool::new(false);

/// 빈 description 세션을 1개씩 순차 자동 요약하는 백그라운드 워커.
/// 이미 실행 중이면 no-op.
#[tauri::command]
fn start_auto_summary(app: tauri::AppHandle) -> Result<bool, String> {
    if AUTO_SUMMARY_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(false);
    }
    const BATCH_SIZE: usize = 5;
    std::thread::spawn(move || {
        use tauri::Emitter;
        loop {
            let sessions = match scanner::scan_local_sessions() {
                Ok(s) => s,
                Err(_) => break,
            };
            let pending: Vec<(String, String)> = sessions
                .into_iter()
                .filter(|s| {
                    s.description.as_deref().unwrap_or("").is_empty()
                        && s.auto_summary.as_deref().unwrap_or("").is_empty()
                })
                .take(BATCH_SIZE)
                .map(|s| (s.session_id, s.file_path))
                .collect();
            if pending.is_empty() {
                break;
            }

            match summary::auto_summarize_batch(&pending) {
                Ok(result) => {
                    for (id, _path) in &pending {
                        if let Some((name, desc)) = result.get(id) {
                            let _ = upsert_session_meta(
                                id,
                                SessionMeta {
                                    name: Some(name.clone()),
                                    auto_summary: Some(desc.clone()),
                                    ..Default::default()
                                },
                            );
                            let _ = app.emit("auto-summary-progress", id);
                        } else {
                            // 이 배치에서 누락 — 다음 루프에서 또 시도되니 그냥 두지만,
                            // 무한 재시도 방지 위해 1회 미스 마커 저장
                            let _ = upsert_session_meta(
                                id,
                                SessionMeta {
                                    auto_summary: Some("(요약 누락 — 재시도 예정)".into()),
                                    ..Default::default()
                                },
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[auto-summary batch] 실패: {}", e);
                    // 배치 전체 실패 시 첫 세션에만 실패 마커 (무한 재시도 방지)
                    if let Some((id, _)) = pending.first() {
                        let _ = upsert_session_meta(
                            id,
                            SessionMeta {
                                auto_summary: Some(format!("(자동 요약 실패: {})", e)),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        AUTO_SUMMARY_RUNNING.store(false, Ordering::SeqCst);
    });
    Ok(true)
}

#[tauri::command]
fn list_sessions() -> Result<Vec<Session>, String> {
    let mut local = scanner::scan_local_sessions().map_err(to_str)?;
    let cloud_all = cloud::list_cloud_sessions().unwrap_or_default();

    // 로컬에도 있고 클라우드에도 있으면 storage_type = "synced"
    use std::collections::HashSet;
    let cloud_ids: HashSet<String> = cloud_all.iter().map(|c| c.session_id.clone()).collect();
    for l in local.iter_mut() {
        if cloud_ids.contains(&l.session_id) {
            l.storage_type = "synced".into();
        } else {
            l.storage_type = "local-only".into();
        }
    }

    // 클라우드에만 있는 세션 → "cloud-only"
    let local_ids: HashSet<String> = local.iter().map(|l| l.session_id.clone()).collect();
    let cloud_only: Vec<Session> = cloud_all
        .into_iter()
        .filter(|c| !local_ids.contains(&c.session_id))
        .map(|mut c| {
            c.storage_type = "cloud-only".into();
            c
        })
        .collect();
    local.extend(cloud_only);
    Ok(local)
}

#[tauri::command]
fn get_config_cmd() -> Config {
    load_config()
}

#[tauri::command]
fn save_session_meta(session_id: String, patch: SessionMeta) -> Result<(), String> {
    upsert_session_meta(&session_id, patch).map_err(to_str)
}

#[tauri::command]
fn delete_session(session_id: String, file_path: String) -> Result<(), String> {
    scanner::delete_session(&session_id, &file_path).map_err(to_str)?;
    cfg_delete_meta(&session_id).map_err(to_str)
}

#[tauri::command]
fn archive_session(session_id: String) -> Result<(), String> {
    scanner::archive_session(&session_id).map_err(to_str)
}

#[tauri::command]
fn unarchive_session(session_id: String) -> Result<(), String> {
    scanner::unarchive_session(&session_id).map_err(to_str)
}

#[tauri::command]
fn save_settings(patch: Settings) -> Result<(), String> {
    update_settings(patch).map_err(to_str)
}

#[tauri::command]
fn set_cloud_folder(root: String) -> Result<String, String> {
    cloud::set_cloud_root(&root)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(to_str)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DebugLogInfo {
    path: String,
    exists: bool,
    size: u64,
    tail: String,
}

#[tauri::command]
fn get_debug_log_cmd() -> DebugLogInfo {
    let path = debuglog::log_path();
    let exists = path.exists();
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let tail = if exists {
        std::fs::read_to_string(&path)
            .map(|s| {
                // 마지막 4000자만
                let chars: Vec<char> = s.chars().collect();
                let start = chars.len().saturating_sub(4000);
                chars[start..].iter().collect::<String>()
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    DebugLogInfo {
        path: path.to_string_lossy().to_string(),
        exists,
        size,
        tail,
    }
}

#[tauri::command]
fn open_debug_log_folder_cmd() -> Result<(), String> {
    let path = debuglog::log_path();
    let folder = path.parent().ok_or("no parent dir")?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(folder)
            .spawn()
            .map_err(to_str)?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(folder)
            .spawn()
            .map_err(to_str)?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(to_str)?;
    }
    Ok(())
}

#[tauri::command]
fn detect_google_drive_cmd() -> cloud::CloudDetectResult {
    cloud::detect_google_drive_result()
}

#[tauri::command]
fn connect_google_drive_cmd() -> Result<String, String> {
    let p = cloud::detect_google_drive()
        .ok_or_else(|| "Google Drive 폴더를 찾지 못했어요. 데스크탑 클라이언트가 설치돼 있나요?".to_string())?;
    cloud::set_cloud_root(&p.to_string_lossy())
        .map(|p| p.to_string_lossy().to_string())
        .map_err(to_str)
}

#[tauri::command]
fn upload_to_cloud(session: Session) -> Result<(), String> {
    cloud::upload_session(&session).map_err(to_str)
}

#[tauri::command]
fn checkout_session(session: Session) -> Result<String, String> {
    cloud::checkout(&session).map_err(to_str)
}

#[tauri::command]
fn checkin_session(session: Session) -> Result<(), String> {
    cloud::checkin(&session).map_err(to_str)
}

#[tauri::command]
fn resume_session(session_id: String, cwd: Option<String>) -> Result<(), String> {
    resume::resume_in_new_terminal(&session_id, cwd.as_deref()).map_err(to_str)
}

#[tauri::command]
fn check_environment_cmd() -> environment::EnvironmentReport {
    environment::check_environment()
}

#[tauri::command]
async fn check_update_cmd() -> Result<update::UpdateInfo, String> {
    update::check_latest_release().await.map_err(to_str)
}

#[tauri::command]
async fn generate_summary_cmd(
    session_id: String,
    file_path: String,
) -> Result<String, String> {
    let cfg = load_config();
    let prev = cfg
        .sessions
        .get(&session_id)
        .and_then(|m| m.description.clone().or(m.auto_summary.clone()));

    let (name, desc) = tokio::task::spawn_blocking(move || {
        summary::auto_summarize_session(&file_path, prev.as_deref())
    })
    .await
    .map_err(to_str)?
    .map_err(to_str)?;

    upsert_session_meta(
        &session_id,
        SessionMeta {
            name: Some(name.clone()),
            auto_summary: Some(desc.clone()),
            ..Default::default()
        },
    )
    .map_err(to_str)?;

    Ok(desc)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_config_cmd,
            save_session_meta,
            delete_session,
            archive_session,
            unarchive_session,
            save_settings,
            set_cloud_folder,
            upload_to_cloud,
            checkout_session,
            checkin_session,
            resume_session,
            check_environment_cmd,
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
