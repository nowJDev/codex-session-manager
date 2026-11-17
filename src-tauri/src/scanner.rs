// Codex 세션 JSONL 파일을 찾아 앱 표시용 메타데이터로 변환한다.
use crate::application::refresh_budget::RefreshBudget;
use crate::config::load_config;
use crate::domain::session::is_internal_summary_run;
use crate::domain::transcript::{
    compact_whitespace, extract_user_message_text, first_user_message_name, truncate,
    TranscriptContent, TranscriptPart, TranscriptPartKind,
};
#[cfg(test)]
use crate::types::DeleteSessionStatus;
use crate::types::{Session, SessionSnapshot};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

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

pub(crate) struct JsonlMeta {
    pub(crate) session_id: Option<String>,
    pub(crate) parent_id: Option<String>,
    pub(crate) is_subagent: bool,
    pub(crate) agent_nickname: Option<String>,
    pub(crate) session_kind: crate::types::SessionKind,
    pub(crate) is_summary_run: bool,
    pub(crate) history_mode: String,
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
    cwd: Option<String>,
    version: Option<String>,
    first_user_message: Option<String>,
    total_lines: usize,
}

fn transcript_part_kind(item: &Value) -> Option<TranscriptPartKind> {
    match item.get("type").and_then(|v| v.as_str()) {
        Some("text") => Some(TranscriptPartKind::Text),
        Some("output_text") => Some(TranscriptPartKind::OutputText),
        Some("input_text") => Some(TranscriptPartKind::InputText),
        _ => None,
    }
}

fn transcript_content_from_json(value: &Value) -> Option<TranscriptContent> {
    if let Some(text) = value.as_str() {
        return Some(TranscriptContent::Text(text.to_string()));
    }
    let parts = value.as_array()?.iter().filter_map(|item| {
        let kind = transcript_part_kind(item)?;
        let text = item
            .get("text")
            .or_else(|| item.get("content"))
            .and_then(|v| v.as_str())?;
        Some(TranscriptPart {
            kind,
            text: text.to_string(),
        })
    });
    Some(TranscriptContent::Parts(parts.collect()))
}

pub(crate) fn read_jsonl_meta(path: &Path) -> Result<JsonlMeta> {
    read_jsonl_meta_using(path, None)
}
pub(crate) fn read_jsonl_meta_with_budget(path: &Path, budget: RefreshBudget) -> Result<JsonlMeta> {
    read_jsonl_meta_using(path, Some(budget))
}
fn read_jsonl_meta_using(path: &Path, budget: Option<RefreshBudget>) -> Result<JsonlMeta> {
    if let Some(budget) = budget {
        budget.check()?;
    }
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut parent_id = None;
    let mut is_subagent = false;
    let mut agent_nickname = None;
    let mut session_kind = crate::types::SessionKind::Unknown;
    let mut history_mode = "unknown".to_string();
    let mut first_ts = None;
    let mut last_ts = None;
    let mut cwd = None;
    let mut version = None;
    let mut first_user = None;
    let mut total = 0usize;
    let mut head_seen = 0usize;

    let mut lines = reader.lines();
    loop {
        if let Some(budget) = budget {
            budget.check()?;
        }
        let Some(line_res) = lines.next() else { break };
        if let Some(budget) = budget {
            budget.check()?;
        }
        let Ok(line) = line_res else { continue };
        total += 1;
        let val: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if first_ts.is_none() {
            first_ts = val
                .get("timestamp")
                .and_then(|v| v.as_str())
                .map(String::from);
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
                history_mode = payload
                    .get("history_mode")
                    .and_then(Value::as_str)
                    .unwrap_or("legacy")
                    .to_string();
                if session_id.is_none() {
                    session_id = payload.get("id").and_then(|v| v.as_str()).map(String::from);
                    let subagent = payload.pointer("/source/subagent").filter(|v| !v.is_null());
                    session_kind = crate::types::SessionKind::from_source(
                        payload.get("source").unwrap_or(&Value::Null),
                    );
                    is_subagent = session_kind == crate::types::SessionKind::Worker;
                    parent_id = payload
                        .get("parent_thread_id")
                        .and_then(Value::as_str)
                        .filter(|s| !s.trim().is_empty())
                        .map(str::to_owned);
                    if let Some(subagent) = subagent {
                        let spawned = subagent.get("thread_spawn").unwrap_or(&Value::Null);
                        let other = subagent.get("other").and_then(|v| v.as_str());

                        parent_id = parent_id.or_else(|| {
                            spawned
                                .get("parent_thread_id")
                                .and_then(Value::as_str)
                                .filter(|s| !s.trim().is_empty())
                                .map(str::to_owned)
                        });
                        agent_nickname = payload
                            .get("agent_nickname")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.trim().is_empty())
                            .or_else(|| spawned.get("agent_nickname").and_then(|v| v.as_str()))
                            .or(other)
                            .filter(|s| !s.trim().is_empty())
                            .map(String::from);
                    }
                }
                if cwd.is_none() {
                    cwd = payload
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .map(String::from);
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
                        first_user = transcript_content_from_json(message)
                            .and_then(|content| extract_user_message_text(&content))
                            .map(|s| truncate(&s, 200));
                    }
                }
            }
            "response_item" => {
                if first_user.is_none()
                    && payload.get("role").and_then(|v| v.as_str()) == Some("user")
                {
                    if let Some(content) = payload.get("content") {
                        first_user = transcript_content_from_json(content)
                            .and_then(|content| extract_user_message_text(&content))
                            .map(|s| truncate(&s, 200));
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(budget) = budget {
        budget.check()?;
    }
    Ok(JsonlMeta {
        session_id,
        parent_id,
        is_subagent,
        agent_nickname,
        session_kind,
        is_summary_run: is_internal_summary_run(cwd.as_deref()),
        history_mode,
        first_timestamp: first_ts,
        last_timestamp: last_ts,
        cwd,
        version,
        first_user_message: first_user,
        total_lines: total,
    })
}

fn load_session_index_names(budget: RefreshBudget) -> HashMap<String, String> {
    if budget.check().is_err() {
        return HashMap::new();
    }
    let path = codex_home().join("session_index.jsonl");
    let Ok(file) = fs::File::open(path) else {
        return HashMap::new();
    };
    let reader = BufReader::new(file);
    let mut names = HashMap::new();
    for line in reader.lines().map_while(Result::ok) {
        if budget.check().is_err() {
            break;
        }
        let Ok(val) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(id) = val.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let name = val
            .get("thread_name")
            .or_else(|| val.get("name"))
            .or_else(|| val.get("title"))
            .and_then(|v| v.as_str())
            .map(compact_whitespace)
            .filter(|s| !s.is_empty());
        if let Some(name) = name {
            names.insert(id.to_string(), name);
        }
    }
    names
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

fn collect_jsonl_files(root: &Path, budget: RefreshBudget) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    'walk: while let Some(dir) = stack.pop() {
        if budget.check().is_err() {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if budget.check().is_err() {
                break 'walk;
            }
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

#[cfg(test)]
pub fn scan_local_sessions() -> Result<Vec<Session>> {
    scan_sessions_in_roots(
        projects_roots(),
        RefreshBudget::new(std::time::Instant::now),
    )
    .map(|s| s.sessions)
}

pub fn scan_extra_sessions(budget: RefreshBudget) -> Result<SessionSnapshot> {
    let roots = load_config()
        .settings
        .extra_project_dirs
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect();
    scan_sessions_in_roots(roots, budget)
}

fn scan_sessions_in_roots(roots: Vec<PathBuf>, budget: RefreshBudget) -> Result<SessionSnapshot> {
    use std::io::Write;
    let mut out = Vec::new();
    let mut snapshot = SessionSnapshot {
        sessions: vec![],
        complete: true,
        missing_count: 0,
        errors: vec![],
    };
    if !budget.record_timeout(&mut snapshot) {
        return Ok(snapshot);
    }
    let archived_root = archived_sessions_dir();

    let log_path = crate::config::config_dir().join("scan-debug.log");
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
        budget.record_timeout(&mut snapshot);
        return Ok(snapshot);
    }
    if !budget.record_timeout(&mut snapshot) {
        return Ok(snapshot);
    }
    let cfg_full = load_config();
    let saved = cfg_full.sessions;
    let index_names = load_session_index_names(budget);

    let mut total_found = 0usize;
    let mut total_pushed = 0usize;
    let mut meta_fail = 0usize;
    let mut stem_fail = 0usize;
    let mut stat_fail = 0usize;
    let mut seen_session_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    'roots: for root in &roots {
        for path in collect_jsonl_files(root, budget) {
            if !budget.record_timeout(&mut snapshot) {
                break 'roots;
            }
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

            let meta = match read_jsonl_meta_with_budget(&path, budget) {
                Ok(m) => Some(m),
                Err(e) => {
                    if !budget.record_timeout(&mut snapshot) {
                        break 'roots;
                    }
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
            if !seen_session_ids.insert(stem.clone()) {
                if let Some(f) = log.as_mut() {
                    let _ = writeln!(f, "[dup-skip] {} ({})", stem, path.display());
                }
                continue;
            }

            let saved_meta = saved.get(&stem).cloned().unwrap_or_default();
            let inferred_name = index_names.get(&stem).cloned().or_else(|| {
                meta.as_ref()
                    .and_then(|m| m.first_user_message.as_deref())
                    .and_then(first_user_message_name)
            });
            let favorite = saved_meta.favorite.unwrap_or(false);
            let project_dir = cwd
                .clone()
                .unwrap_or_else(|| root.to_string_lossy().to_string());
            let archived = path.starts_with(&archived_root);

            if !budget.record_timeout(&mut snapshot) {
                break 'roots;
            }
            total_pushed += 1;
            out.push(Session {
                session_id: stem.clone(),
                parent_id: meta.as_ref().and_then(|m| m.parent_id.clone()),
                is_subagent: meta.as_ref().is_some_and(|m| m.is_subagent),
                agent_nickname: meta.as_ref().and_then(|m| m.agent_nickname.clone()),
                name: saved_meta
                    .name
                    .or_else(|| meta.as_ref().and_then(|m| m.agent_nickname.clone()))
                    .or(inferred_name)
                    .or_else(|| {
                        Some(
                            meta.as_ref()
                                .map(|m| m.session_kind)
                                .unwrap_or_default()
                                .fallback_name(&stem),
                        )
                    }),
                description: saved_meta.description,
                auto_summary: saved_meta.auto_summary,
                favorite,
                project: project_name_from_cwd(cwd.as_deref(), "Codex"),
                project_dir,
                file_path: Some(path.to_string_lossy().to_string()),
                size: Some(stat.len()),
                preview: None,
                history_mode: meta
                    .as_ref()
                    .map(|m| m.history_mode.clone())
                    .unwrap_or_else(|| "unknown".into()),
                registered: false,
                session_kind: meta.as_ref().map(|m| m.session_kind).unwrap_or_default(),
                is_summary_run: meta.as_ref().is_some_and(|m| m.is_summary_run),
                total_lines: meta.as_ref().map(|m| m.total_lines).unwrap_or(0),
                first_timestamp: meta.as_ref().and_then(|m| m.first_timestamp.clone()),
                last_timestamp: meta.as_ref().and_then(|m| m.last_timestamp.clone()),
                cwd,
                version: meta.as_ref().and_then(|m| m.version.clone()),
                first_user_message: meta.as_ref().and_then(|m| m.first_user_message.clone()),
                storage_type: saved_meta.storage_type.unwrap_or_else(|| "local".into()),
                archived,
                locked_by: None,
            });
        }
    }

    if let Some(f) = log.as_mut() {
        let _ = writeln!(f, "---");
        let _ = writeln!(f, "TOTAL found    = {}", total_found);
        let _ = writeln!(f, "TOTAL pushed   = {}", total_pushed);
        let _ = writeln!(f, "stat_fail      = {}", stat_fail);
        let _ = writeln!(f, "stem_fail      = {}", stem_fail);
        let _ = writeln!(f, "meta_fail      = {}", meta_fail);
        let _ = writeln!(f, "out.len()      = {} (returned to frontend)", out.len());
    }

    budget.record_timeout(&mut snapshot);
    out.sort_by(|a, b| match b.favorite.cmp(&a.favorite) {
        std::cmp::Ordering::Equal => {
            let ta = a.last_timestamp.as_deref().unwrap_or("");
            let tb = b.last_timestamp.as_deref().unwrap_or("");
            tb.cmp(ta)
        }
        other => other,
    });

    snapshot.sessions = out;
    budget.record_timeout(&mut snapshot);
    Ok(snapshot)
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
        let Ok(val) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if val.get("type").and_then(|v| v.as_str()) != Some("event_msg") {
            continue;
        }
        let Some(payload) = val.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(|v| v.as_str()) != Some("user_message") {
            continue;
        }
        if let Some(message) = payload.get("message") {
            if let Some(text) = transcript_content_from_json(message)
                .and_then(|content| extract_user_message_text(&content))
            {
                out.push(truncate(&text, 300));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
pub fn delete_session_file(file_path: &str) -> Result<()> {
    let path = PathBuf::from(file_path);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn validate_session_id(session_id: &str) -> Result<()> {
    if session_id.is_empty()
        || session_id.starts_with('-')
        || !session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(anyhow!("올바르지 않은 세션 ID"));
    }
    Ok(())
}

fn codex_command(codex: &str, home: &Path) -> Command {
    let mut cmd = Command::new(codex);
    cmd.env("CODEX_HOME", home);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn run_codex_session_action_using(
    codex: &str,
    home: &Path,
    action: &str,
    session_id: &str,
) -> Result<()> {
    validate_session_id(session_id)?;
    let mut cmd = codex_command(codex, home);
    cmd.arg(action);
    if action == "delete" {
        cmd.arg("--force");
    }
    cmd.arg(session_id);
    let output = cmd.output().map_err(|e| {
        crate::debuglog::log(
            "session-action",
            &format!(
                "action={action} id={session_id} cli={codex} home={} launch_error={e}",
                home.display()
            ),
        );
        anyhow!("codex {} 실행 실패: {}", action, e)
    })?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let version = codex_command(codex, home)
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into());
        crate::debuglog::log("session-action", &format!(
            "action={action} id={session_id} cli={codex} version={version} home={} exit={} stderr={stderr}",
            home.display(), output.status
        ));
        Err(anyhow!(
            "codex {} {} 실패({}): {}",
            action,
            session_id,
            output.status,
            stderr
        ))
    }
}

fn run_codex_session_action(action: &str, session_id: &str) -> Result<()> {
    let codex =
        crate::environment::locate_codex().ok_or_else(|| anyhow!("codex CLI를 찾을 수 없음"))?;
    run_codex_session_action_using(&codex, &codex_home(), action, session_id)
}

#[cfg(test)]
fn delete_session_using(
    codex: Option<&str>,
    home: &Path,
    roots: &[PathBuf],
    session_id: &str,
    file_path: &Path,
) -> Result<DeleteSessionStatus> {
    validate_session_id(session_id)?;
    let result = codex
        .ok_or_else(|| anyhow!("codex CLI를 찾을 수 없음"))
        .and_then(|codex| run_codex_session_action_using(codex, home, "delete", session_id));
    match result {
        Ok(()) => Ok(DeleteSessionStatus::Deleted),
        Err(error) => {
            match crate::session_state::is_session_absent(home, roots, session_id, file_path) {
                Ok(true) => {
                    crate::debuglog::log(
                        "session-delete",
                        &format!(
                            "id={session_id} already_missing=true home={}",
                            home.display()
                        ),
                    );
                    Ok(DeleteSessionStatus::AlreadyMissing)
                }
                Ok(false) => Err(error),
                Err(inspection_error) => {
                    crate::debuglog::log(
                        "session-delete",
                        &format!("id={session_id} state_check_error={inspection_error:#}"),
                    );
                    Err(anyhow!(
                        "{error}; 세션 부재 확인 실패: {inspection_error:#}"
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
pub fn delete_session(session_id: &str, file_path: &str) -> Result<DeleteSessionStatus> {
    let home = codex_home();
    // projects_roots는 접근 불가 경로를 제외하므로 안전한 부재 확인에는 필터 전 경로를 사용한다.
    let mut roots = vec![home.join("sessions"), home.join("archived_sessions")];
    roots.extend(
        load_config()
            .settings
            .extra_project_dirs
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from),
    );
    delete_session_using(
        crate::environment::locate_codex().as_deref(),
        &home,
        &roots,
        session_id,
        Path::new(file_path),
    )
}

pub fn archive_session(session_id: &str) -> Result<()> {
    run_codex_session_action("archive", session_id)
}

pub fn unarchive_session(session_id: &str) -> Result<()> {
    run_codex_session_action("unarchive", session_id)
}

#[cfg(test)]
#[path = "scanner_delete_tests.rs"]
mod delete_tests;

#[cfg(test)]
mod budget_tests {
    use super::*;
    use crate::application::refresh_budget::RefreshBudget;
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    thread_local! {static CLOCK: Cell<Option<(Instant,u64)>> = const {Cell::new(None)};}
    fn ticking_now() -> Instant {
        CLOCK.with(|c| {
            let (base, tick) = c.get().unwrap();
            c.set(Some((base, tick + 1)));
            base + Duration::from_secs(tick)
        })
    }
    #[test]
    fn refresh_budget_stops_large_extra_jsonl_inside_line_loop() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("large.jsonl");
        fs::write(&file, "{}\n".repeat(200)).unwrap();
        CLOCK.with(|c| c.set(Some((Instant::now(), 0))));
        let result = read_jsonl_meta_with_budget(&file, RefreshBudget::new(ticking_now));
        assert!(
            result.is_err(),
            "JSONL 반복 처리 중 초과한 갱신 예산을 검사해야 한다"
        );
        assert!(result.err().unwrap().to_string().contains("예산 초과"));
    }
    #[test]
    fn refresh_budget_stops_directory_walk_and_keeps_discovered_paths() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..100 {
            fs::write(dir.path().join(format!("{i}.jsonl")), "{}").unwrap();
        }
        CLOCK.with(|c| c.set(Some((Instant::now(), 0))));
        let budget = RefreshBudget::new(ticking_now);
        let files = collect_jsonl_files(dir.path(), budget);
        assert!(!files.is_empty());
        assert!(files.len() < 100);
        assert!(budget.check().is_err());
    }
}
