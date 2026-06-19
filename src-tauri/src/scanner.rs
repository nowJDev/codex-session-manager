// Codex 세션 JSONL 파일을 찾아 앱 표시용 메타데이터로 변환한다.
use crate::config::load_config;
use crate::types::Session;
use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub fn codex_home() -> PathBuf {
    if let Ok(p) = std::env::var("CODEX_HOME") {
        PathBuf::from(p)
    } else if let Ok(p) = std::env::var("CODEX_SESSION_HOME") {
        PathBuf::from(p).join(".codex")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".codex")
    }
}

pub fn sessions_dir() -> PathBuf {
    codex_home().join("sessions")
}

pub fn archived_sessions_dir() -> PathBuf {
    codex_home().join("archived_sessions")
}

pub fn projects_dir() -> PathBuf {
    sessions_dir()
}

/// 기본 sessions_dir + archived_sessions_dir + 사용자가 추가한 extra_project_dirs.
/// 존재하지 않는 경로는 자동으로 제외.
pub fn projects_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for primary in [sessions_dir(), archived_sessions_dir()] {
        if primary.exists() && !roots.iter().any(|r| r == &primary) {
            roots.push(primary);
        }
    }

    let cfg = crate::config::load_config();
    if let Some(extra) = &cfg.settings.extra_project_dirs {
        for p in extra {
            let pb = PathBuf::from(p);
            if pb.exists() && !roots.iter().any(|r| r == &pb) {
                roots.push(pb);
            }
        }
    }

    roots
}

struct JsonlMeta {
    session_id: Option<String>,
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
    cwd: Option<String>,
    version: Option<String>,
    first_user_message: Option<String>,
    total_lines: usize,
}

fn read_jsonl_meta(path: &Path) -> Result<JsonlMeta> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut first_ts = None;
    let mut last_ts = None;
    let mut cwd = None;
    let mut version = None;
    let mut first_user = None;
    let mut total = 0usize;
    let mut head_seen = 0usize;

    for line_res in reader.lines() {
        let Ok(line) = line_res else { continue };
        total += 1;
        let val: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if first_ts.is_none() {
            first_ts = val.get("timestamp").and_then(|v| v.as_str()).map(String::from);
        }
        if let Some(ts) = val.get("timestamp").and_then(|v| v.as_str()) {
            last_ts = Some(ts.to_string());
        }
        if head_seen >= 200 {
            continue;
        }
        head_seen += 1;

        let item_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let payload = val.get("payload").unwrap_or(&Value::Null);
        match item_type {
            "session_meta" => {
                if session_id.is_none() {
                    session_id = payload.get("id").and_then(|v| v.as_str()).map(String::from);
                }
                if cwd.is_none() {
                    cwd = payload.get("cwd").and_then(|v| v.as_str()).map(String::from);
                }
                if version.is_none() {
                    version = payload
                        .get("cli_version")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                }
            }
            "turn_context" => {
                if let Some(latest_cwd) = payload.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(latest_cwd.to_string());
                }
            }
            "event_msg" => {
                if first_user.is_none()
                    && payload.get("type").and_then(|v| v.as_str()) == Some("user_message")
                {
                    if let Some(message) = payload.get("message") {
                        first_user = extract_text(message).map(|s| truncate(&s, 200));
                    }
                }
            }
            "response_item" => {
                if first_user.is_none()
                    && payload.get("role").and_then(|v| v.as_str()) == Some("user")
                {
                    if let Some(content) = payload.get("content") {
                        first_user = extract_text(content).map(|s| truncate(&s, 200));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(JsonlMeta {
        session_id,
        first_timestamp: first_ts,
        last_timestamp: last_ts,
        cwd,
        version,
        first_user_message: first_user,
        total_lines: total,
    })
}

fn extract_text(content: &Value) -> Option<String> {
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        for item in arr {
            if matches!(
                item.get("type").and_then(|v| v.as_str()),
                Some("text") | Some("output_text") | Some("input_text")
            ) {
                if let Some(t) = item
                    .get("text")
                    .or_else(|| item.get("content"))
                    .and_then(|v| v.as_str())
                {
                    return Some(t.to_string());
                }
            }
        }
    }
    None
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn project_name_from_cwd(cwd: Option<&str>, fallback: &str) -> String {
    cwd.and_then(|p| {
        Path::new(p)
            .file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
    })
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| fallback.to_string())
}

fn session_id_from_filename(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if stem.len() >= 36 {
        Some(stem[stem.len() - 36..].to_string())
    } else {
        Some(stem.to_string())
    }
}

/// cwd 경로를 느슨한 비교용 안전 문자열로 인코딩한다.
pub fn encode_cwd_to_project_dir(cwd: &str) -> String {
    cwd.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// jsonl 본문 앞부분에서 최신 `payload.cwd`를 추출한다.
pub fn read_cwd_from_jsonl(path: &PathBuf) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut cwd = None;
    for (i, line) in reader.lines().enumerate() {
        if i >= 200 {
            break;
        }
        let Ok(line) = line else { continue };
        let Ok(val) = serde_json::from_str::<Value>(&line) else { continue };
        if let Some(next_cwd) = val.pointer("/payload/cwd").and_then(|v| v.as_str()) {
            cwd = Some(next_cwd.to_string());
        }
    }
    cwd
}

fn collect_jsonl_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                out.push(path);
            }
        }
    }
    out
}

pub fn scan_local_sessions() -> Result<Vec<Session>> {
    use std::io::Write;
    let mut out = Vec::new();
    let roots = projects_roots();

    let log_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex-sessions")
        .join("scan-debug.log");
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut log = fs::File::create(&log_path).ok();
    let ts = chrono::Utc::now().to_rfc3339();
    if let Some(f) = log.as_mut() {
        let _ = writeln!(f, "=== scan_local_sessions @ {} ===", ts);
        let _ = writeln!(f, "roots = {}", roots.len());
        for r in &roots {
            let _ = writeln!(f, "  - {}", r.display());
        }
    }

    if roots.is_empty() {
        return Ok(out);
    }
    let cfg_full = load_config();
    let saved = cfg_full.sessions;

    let excluded_raw: Vec<String> = cfg_full
        .settings
        .excluded_scan_paths
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect();
    let excluded_encoded: Vec<String> = excluded_raw
        .iter()
        .map(|p| encode_cwd_to_project_dir(p))
        .filter(|s| !s.is_empty())
        .collect();

    let mut total_found = 0usize;
    let mut total_pushed = 0usize;
    let mut total_excluded = 0usize;
    let mut meta_fail = 0usize;
    let mut stem_fail = 0usize;
    let mut stat_fail = 0usize;
    let mut seen_session_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for root in &roots {
        for path in collect_jsonl_files(root) {
            total_found += 1;
            let stat = match fs::metadata(&path) {
                Ok(s) => s,
                Err(e) => {
                    stat_fail += 1;
                    if let Some(f) = log.as_mut() {
                        let _ = writeln!(f, "[SKIP stat] {}: {}", path.display(), e);
                    }
                    continue;
                }
            };
            let Some(fallback_session_id) = session_id_from_filename(&path) else {
                stem_fail += 1;
                if let Some(f) = log.as_mut() {
                    let _ = writeln!(f, "[SKIP stem] {}", path.display());
                }
                continue;
            };

            let meta = match read_jsonl_meta(&path) {
                Ok(m) => Some(m),
                Err(e) => {
                    meta_fail += 1;
                    if let Some(f) = log.as_mut() {
                        let _ = writeln!(f, "[meta-fail] {}: {}", path.display(), e);
                    }
                    None
                }
            };
            let stem = meta
                .as_ref()
                .and_then(|m| m.session_id.clone())
                .unwrap_or(fallback_session_id);
            let cwd = meta.as_ref().and_then(|m| m.cwd.clone());
            let file_path_text = path.to_string_lossy().to_string();
            let excluded_match = excluded_raw.iter().any(|p| {
                file_path_text.contains(p) || cwd.as_deref().is_some_and(|c| c.contains(p))
            }) || excluded_encoded.iter().any(|p| {
                file_path_text.contains(p) || cwd.as_deref().is_some_and(|c| c.contains(p))
            });
            if excluded_match {
                total_excluded += 1;
                if let Some(f) = log.as_mut() {
                    let _ = writeln!(f, "[excluded] {}", path.display());
                }
                continue;
            }

            if !seen_session_ids.insert(stem.clone()) {
                if let Some(f) = log.as_mut() {
                    let _ = writeln!(f, "[dup-skip] {} ({})", stem, path.display());
                }
                continue;
            }

            let saved_meta = saved.get(&stem).cloned().unwrap_or_default();
            let favorite = saved_meta.favorite.unwrap_or(false);
            let project_dir = cwd.clone().unwrap_or_else(|| root.to_string_lossy().to_string());

            total_pushed += 1;
            out.push(Session {
                session_id: stem,
                name: saved_meta.name,
                description: saved_meta.description,
                auto_summary: saved_meta.auto_summary,
                favorite,
                project: project_name_from_cwd(cwd.as_deref(), "Codex"),
                project_dir,
                file_path: path.to_string_lossy().to_string(),
                size: stat.len(),
                total_lines: meta.as_ref().map(|m| m.total_lines).unwrap_or(0),
                first_timestamp: meta.as_ref().and_then(|m| m.first_timestamp.clone()),
                last_timestamp: meta.as_ref().and_then(|m| m.last_timestamp.clone()),
                cwd,
                version: meta.as_ref().and_then(|m| m.version.clone()),
                first_user_message: meta.as_ref().and_then(|m| m.first_user_message.clone()),
                storage_type: saved_meta.storage_type.unwrap_or_else(|| "local".into()),
                locked_by: None,
            });
        }
    }

    if let Some(f) = log.as_mut() {
        let _ = writeln!(f, "---");
        let _ = writeln!(f, "TOTAL found    = {}", total_found);
        let _ = writeln!(f, "TOTAL pushed   = {}", total_pushed);
        let _ = writeln!(f, "TOTAL excluded = {}", total_excluded);
        let _ = writeln!(f, "stat_fail      = {}", stat_fail);
        let _ = writeln!(f, "stem_fail      = {}", stem_fail);
        let _ = writeln!(f, "meta_fail      = {}", meta_fail);
        let _ = writeln!(f, "out.len()      = {} (returned to frontend)", out.len());
    }

    out.sort_by(|a, b| match b.favorite.cmp(&a.favorite) {
        std::cmp::Ordering::Equal => {
            let ta = a.last_timestamp.as_deref().unwrap_or("");
            let tb = b.last_timestamp.as_deref().unwrap_or("");
            tb.cmp(ta)
        }
        other => other,
    });

    Ok(out)
}

pub fn get_session_messages(file_path: &str, max_messages: usize) -> Result<Vec<String>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line_res in reader.lines() {
        if out.len() >= max_messages {
            break;
        }
        let Ok(line) = line_res else { continue };
        let Ok(val) = serde_json::from_str::<Value>(&line) else { continue };
        if val.get("type").and_then(|v| v.as_str()) != Some("event_msg") {
            continue;
        }
        let Some(payload) = val.get("payload") else { continue };
        if payload.get("type").and_then(|v| v.as_str()) != Some("user_message") {
            continue;
        }
        if let Some(message) = payload.get("message") {
            if let Some(text) = extract_text(message) {
                out.push(truncate(&text, 300));
            }
        }
    }
    Ok(out)
}

pub fn delete_session_file(file_path: &str) -> Result<()> {
    let path = PathBuf::from(file_path);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
