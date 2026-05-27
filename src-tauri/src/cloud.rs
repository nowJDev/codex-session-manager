use crate::config::{load_config, upsert_session_meta};
use crate::scanner::projects_dir;
use crate::types::{Session, SessionMeta};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CLOUD_FOLDER: &str = "Claude Sessions";

/// Google Drive 데스크탑 클라이언트의 로컬 마운트 폴더를 탐지한다.
/// 검사 우선순위:
/// 1. 환경변수 GOOGLE_DRIVE_PATH
/// 2. `%USERPROFILE%\Google Drive\My Drive` (구버전 Backup&Sync)
/// 3. Drive for desktop의 가상 드라이브 (G:\My Drive, H:\My Drive ...)
///    - Windows가 OS 언어로 localize하므로 한국어/일본어/중국어 폴더명도 시도
///    - 그래도 못 찾으면 `.shortcut-targets-by-id` (Google Drive 시그니처) 존재 시
///      해당 드라이브에서 가장 큰 사용자 폴더를 root로 채택
/// 4. macOS: ~/Library/CloudStorage/GoogleDrive-*/My Drive
/// Google Drive의 "My Drive" 폴더 — OS 언어에 따라 localize된 이름들.
/// Windows Drive for desktop은 OS 표시 언어를 따라간다 (영어 OS면 영어, 한국어 OS면 한국어 등).
const MY_DRIVE_NAMES: &[&str] = &[
    "My Drive",      // English
    "내 드라이브",    // Korean
    "マイドライブ",   // Japanese
    "Mein Drive",    // German
    "Meu Drive",     // Portuguese
    "Mi unidad",     // Spanish
    "Mon Drive",     // French
    "Il mio Drive",  // Italian
    "我的云端硬盘",   // Chinese (Simplified)
    "我的雲端硬碟",   // Chinese (Traditional)
];

/// 알려진 이름으로 못 찾았을 때 — 드라이브 루트에서 사용자 폴더로 보이는 것
/// (숨김/시스템 제외, 디렉토리)을 하나 골라낸다. 시그니처 검증을 통과한
/// 드라이브에 한해서만 호출되므로 false positive 위험은 낮다.
#[cfg(target_os = "windows")]
fn find_my_drive_fallback(drive_root: &PathBuf) -> Option<PathBuf> {
    let entries = fs::read_dir(drive_root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        // 시스템/숨김 폴더 스킵
        if name.starts_with('.') || name.starts_with('$') {
            continue;
        }
        // "다른 컴퓨터" / "Other computers" 류는 우리가 원하는 게 아님
        // 하지만 일반화하기 어려우니, 알려진 이름 외엔 다 후보로 인정
        // (자동 연결 안 되면 어차피 사용자가 수동 선택)
        return Some(path);
    }
    None
}

pub fn detect_google_drive() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("GOOGLE_DRIVE_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }

    let home = dirs::home_dir()?;

    // Windows Backup & Sync (영어/한국어/일본어/중국어)
    for name in MY_DRIVE_NAMES {
        let p = home.join("Google Drive").join(name);
        if p.exists() {
            return Some(p);
        }
    }
    let legacy2 = home.join("Google Drive");
    if legacy2.exists() && legacy2.is_dir() {
        return Some(legacy2);
    }

    // Windows Drive for desktop 가상 드라이브 (G:, H:, ...)
    #[cfg(target_os = "windows")]
    {
        for letter in ['G', 'H', 'I', 'J', 'K', 'L'] {
            let drive = PathBuf::from(format!("{}:\\", letter));
            // 알려진 이름들 시도 (영어/한국어/일본어/중국어 간체/번체)
            for name in MY_DRIVE_NAMES {
                let p = drive.join(name);
                if p.exists() {
                    return Some(p);
                }
            }
            // Fallback: Google Drive 시그니처(.shortcut-targets-by-id)가 있으면
            // 이 드라이브의 첫 정상 디렉토리를 My Drive로 추정
            if drive.join(".shortcut-targets-by-id").exists() {
                if let Some(found) = find_my_drive_fallback(&drive) {
                    return Some(found);
                }
            }
        }
    }

    // macOS Drive for desktop
    #[cfg(target_os = "macos")]
    {
        let cloud_storage = home.join("Library").join("CloudStorage");
        if cloud_storage.exists() {
            if let Ok(entries) = fs::read_dir(&cloud_storage) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("GoogleDrive-") {
                        let mydrive = entry.path().join("My Drive");
                        if mydrive.exists() {
                            return Some(mydrive);
                        }
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudDetectResult {
    pub found: bool,
    pub path: Option<String>,
}

pub fn detect_google_drive_result() -> CloudDetectResult {
    match detect_google_drive() {
        Some(p) => CloudDetectResult {
            found: true,
            path: Some(p.to_string_lossy().to_string()),
        },
        None => CloudDetectResult { found: false, path: None },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudMeta {
    session_id: String,
    name: Option<String>,
    description: Option<String>,
    auto_summary: Option<String>,
    project: String,
    project_dir: String,
    uploaded_at: String,
}

pub fn cloud_path() -> Option<PathBuf> {
    load_config().settings.cloud_path.map(PathBuf::from)
}

pub fn set_cloud_root(root: &str) -> Result<PathBuf> {
    let root = PathBuf::from(root);
    if !root.exists() {
        return Err(anyhow!("folder not found: {}", root.display()));
    }
    let folder = root.join(CLOUD_FOLDER);
    fs::create_dir_all(&folder)?;
    crate::config::update_settings(crate::types::Settings {
        cloud_path: Some(folder.to_string_lossy().to_string()),
        ..Default::default()
    })?;
    Ok(folder)
}

pub fn upload_session(s: &Session) -> Result<()> {
    let cloud = cloud_path().ok_or_else(|| anyhow!("cloud not configured"))?;
    fs::create_dir_all(&cloud)?;

    let dest = cloud.join(format!("{}.jsonl", s.session_id));
    fs::copy(&s.file_path, &dest)?;

    let meta = CloudMeta {
        session_id: s.session_id.clone(),
        name: s.name.clone(),
        description: s.description.clone(),
        auto_summary: s.auto_summary.clone(),
        project: s.project.clone(),
        project_dir: s.project_dir.clone(),
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    };
    fs::write(
        cloud.join(format!("{}.meta.json", s.session_id)),
        serde_json::to_string_pretty(&meta)?,
    )?;

    // 로컬은 보존 — 활성 세션이 계속 jsonl을 갱신할 수 있으므로 삭제하면
    // 새 jsonl이 생기면서 데이터가 분리되는 문제가 생긴다. 사용자가 명시적으로
    // "Sync to cloud"를 다시 누를 때 클라우드를 로컬 최신본으로 덮어쓰면 됨.

    upsert_session_meta(
        &s.session_id,
        SessionMeta {
            storage_type: Some("cloud".into()),
            ..Default::default()
        },
    )?;
    Ok(())
}

// === 락 파일 메커니즘 ===
// 클라우드에 `<id>.lock` 파일을 두어 동시 편집을 방지.
// 락 본문: { hostname, acquired_at } JSON

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockInfo {
    pub hostname: String,
    pub acquired_at: String,
}

fn lock_path(cloud: &PathBuf, session_id: &str) -> PathBuf {
    cloud.join(format!("{}.lock", session_id))
}

fn machine_id() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn read_lock(session_id: &str) -> Option<LockInfo> {
    let cloud = cloud_path()?;
    let path = lock_path(&cloud, session_id);
    if !path.exists() {
        return None;
    }
    let body = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&body).ok()
}

pub fn acquire_lock(session_id: &str) -> Result<()> {
    let cloud = cloud_path().ok_or_else(|| anyhow!("cloud not configured"))?;
    fs::create_dir_all(&cloud)?;
    let path = lock_path(&cloud, session_id);
    let me = machine_id();
    if let Some(existing) = read_lock(session_id) {
        if existing.hostname != me {
            return Err(anyhow!(
                "이 세션은 다른 PC '{}'에서 사용 중 (락 시각: {})",
                existing.hostname,
                existing.acquired_at
            ));
        }
        // 같은 PC면 락 재획득 허용
    }
    let info = LockInfo {
        hostname: me,
        acquired_at: chrono::Utc::now().to_rfc3339(),
    };
    fs::write(&path, serde_json::to_string_pretty(&info)?)?;
    Ok(())
}

pub fn release_lock(session_id: &str) -> Result<()> {
    let Some(cloud) = cloud_path() else { return Ok(()) };
    let path = lock_path(&cloud, session_id);
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    Ok(())
}

pub fn list_cloud_sessions() -> Result<Vec<Session>> {
    let Some(cloud) = cloud_path() else { return Ok(vec![]) };
    if !cloud.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&cloud)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else { continue };
        if !name.ends_with(".meta.json") {
            continue;
        }
        let Ok(body) = fs::read_to_string(&path) else { continue };
        let Ok(meta) = serde_json::from_str::<CloudMeta>(&body) else { continue };
        let jsonl = cloud.join(format!("{}.jsonl", meta.session_id));
        let stat = fs::metadata(&jsonl).ok();
        let lock = read_lock(&meta.session_id);
        let locked_by = lock.map(|l| l.hostname);
        out.push(Session {
            session_id: meta.session_id.clone(),
            name: meta.name,
            description: meta.description,
            auto_summary: meta.auto_summary,
            project: meta.project,
            project_dir: meta.project_dir,
            file_path: jsonl.to_string_lossy().to_string(),
            size: stat.as_ref().map(|s| s.len()).unwrap_or(0),
            total_lines: 0,
            first_timestamp: None,
            last_timestamp: Some(meta.uploaded_at),
            cwd: None,
            version: None,
            first_user_message: None,
            storage_type: "cloud".into(),
            favorite: false,
            locked_by,
        });
    }
    Ok(out)
}

pub fn checkout(session: &Session) -> Result<String> {
    let cloud = cloud_path().ok_or_else(|| anyhow!("cloud not configured"))?;
    let src = cloud.join(format!("{}.jsonl", session.session_id));
    if !src.exists() {
        return Err(anyhow!("session not found in cloud"));
    }
    // 락 획득 (다른 PC에서 사용 중이면 실패)
    acquire_lock(&session.session_id)?;

    // jsonl 본문의 cwd를 신뢰해서 폴더를 결정. 메타의 project_dir이 과거에 잘못
    // 박혔어도 여기서 교정된다. cwd가 없는 비정상 jsonl만 fallback으로 메타 사용.
    let project_dir = crate::scanner::read_cwd_from_jsonl(&src)
        .map(|c| crate::scanner::encode_cwd_to_project_dir(&c))
        .unwrap_or_else(|| session.project_dir.clone());

    let local_dir = projects_dir().join(&project_dir);
    fs::create_dir_all(&local_dir)?;
    let dest = local_dir.join(format!("{}.jsonl", session.session_id));
    fs::copy(&src, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}

pub fn checkin(session: &Session) -> Result<()> {
    let Some(cloud) = cloud_path() else { return Ok(()) };

    // 로컬 jsonl 위치 — file_path가 실제 로컬 파일이면 그걸 우선 사용.
    // (클라우드 폴더 안의 경로일 수도 있으니 그건 제외.) 못 찾으면 project_dir 기반으로 폴백.
    let cloud_root = cloud.to_string_lossy().to_string();
    let file_path_pb = PathBuf::from(&session.file_path);
    let local_path = if !session.file_path.is_empty()
        && !session.file_path.starts_with(&cloud_root)
        && file_path_pb.exists()
    {
        file_path_pb
    } else {
        projects_dir()
            .join(&session.project_dir)
            .join(format!("{}.jsonl", session.session_id))
    };

    if local_path.exists() {
        let dest = cloud.join(format!("{}.jsonl", session.session_id));
        fs::copy(&local_path, &dest)?;
        // 로컬 보존 — single source of truth 폐기 (활성 세션 데이터 분리 방지)
    }

    let meta_path = cloud.join(format!("{}.meta.json", session.session_id));
    if meta_path.exists() {
        let body = fs::read_to_string(&meta_path).unwrap_or_default();
        if let Ok(mut meta) = serde_json::from_str::<CloudMeta>(&body) {
            meta.uploaded_at = chrono::Utc::now().to_rfc3339();
            if session.name.is_some() {
                meta.name = session.name.clone();
            }
            if session.description.is_some() {
                meta.description = session.description.clone();
            }
            fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
        }
    }

    // 락 해제
    let _ = release_lock(&session.session_id);
    Ok(())
}
