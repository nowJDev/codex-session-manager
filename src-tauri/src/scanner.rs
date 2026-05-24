use crate::config::load_config;
use crate::types::Session;
use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub fn claude_dir() -> PathBuf {
    let home = if let Ok(p) = std::env::var("CLAUDE_SESSION_HOME") {
        PathBuf::from(p)
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    };
    home.join(".claude")
}

pub fn projects_dir() -> PathBuf {
    claude_dir().join("projects")
}

/// 기본 projects_dir + 사용자가 추가한 extra_project_dirs + (Windows에서) WSL 자동 탐지.
/// 존재하지 않는 경로는 자동으로 제외.
pub fn projects_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    let primary = projects_dir();
    if primary.exists() {
        roots.push(primary);
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

    // WSL 자동 탐지 (Windows 전용, 기본 활성)
    #[cfg(target_os = "windows")]
    {
        let wsl_on = cfg.settings.wsl_auto_detect.unwrap_or(true);
        if wsl_on {
            for p in detect_wsl_projects_dirs() {
                if !roots.iter().any(|r| r == &p) {
                    roots.push(p);
                }
            }
        }
    }

    roots
}

/// `wsl.exe -l -q` 로 배포판 목록을 얻고, 각 배포판의 `\\wsl.localhost\<distro>\home\*\.claude\projects`
/// 중 존재하는 경로를 반환.
#[cfg(target_os = "windows")]
fn detect_wsl_projects_dirs() -> Vec<PathBuf> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = match Command::new("wsl.exe")
        .args(["-l", "-q"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if !output.status.success() {
        return Vec::new();
    }

    // wsl.exe -l -q 는 UTF-16 LE BOM 출력
    let raw = &output.stdout;
    let text = if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
        let u16s: Vec<u16> = raw[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        String::from_utf8_lossy(raw).into_owned()
    };

    let mut result = Vec::new();
    for line in text.lines() {
        let distro = line.trim().trim_matches('\0');
        if distro.is_empty() {
            continue;
        }
        let home_root = PathBuf::from(format!(r"\\wsl.localhost\{}\home", distro));
        if !home_root.exists() {
            continue;
        }
        let Ok(users) = fs::read_dir(&home_root) else { continue };
        for user_entry in users.flatten() {
            let projects = user_entry.path().join(".claude").join("projects");
            if projects.exists() && projects.is_dir() {
                result.push(projects);
            }
        }
    }
    result
}

struct JsonlMeta {
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
    cwd: Option<String>,
    version: Option<String>,
    first_user_message: Option<String>,
    total_lines: usize,
}

fn read_jsonl_meta(path: &PathBuf) -> Result<JsonlMeta> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

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
        if head_seen < 20 {
            head_seen += 1;
            if first_ts.is_none() {
                first_ts = val.get("timestamp").and_then(|v| v.as_str()).map(String::from);
            }
            if cwd.is_none() {
                cwd = val.get("cwd").and_then(|v| v.as_str()).map(String::from);
            }
            if version.is_none() {
                version = val.get("version").and_then(|v| v.as_str()).map(String::from);
            }
            if first_user.is_none()
                && val.get("type").and_then(|v| v.as_str()) == Some("user")
            {
                if let Some(content) = val.pointer("/message/content") {
                    first_user = extract_text(content).map(|s| truncate(&s, 200));
                }
            }
        }
        if let Some(ts) = val.get("timestamp").and_then(|v| v.as_str()) {
            last_ts = Some(ts.to_string());
        }
    }

    Ok(JsonlMeta {
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
            if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
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

fn decode_project_name(dir: &str) -> String {
    let mut s = dir.replace("--", "/");
    if let Some(first) = s.chars().next() {
        if first.is_ascii_uppercase() {
            s = format!("{}:{}", first, &s[1..]);
        }
    }
    s
}

pub fn scan_local_sessions() -> Result<Vec<Session>> {
    use std::io::Write;
    let mut out = Vec::new();
    let roots = projects_roots();

    let log_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude-sessions")
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
    let saved = load_config().sessions;

    let mut total_found = 0usize;
    let mut total_pushed = 0usize;
    let mut meta_fail = 0usize;
    let mut stem_fail = 0usize;
    let mut stat_fail = 0usize;
    let mut seen_session_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for root in &roots {
    for entry in fs::read_dir(root)? {
        let Ok(entry) = entry else {
            if let Some(f) = log.as_mut() {
                let _ = writeln!(f, "[ERR] read_dir entry failed in projects_dir");
            }
            continue;
        };
        let project_path = entry.path();
        if !project_path.is_dir() {
            continue;
        }
        let project_dir = entry.file_name().to_string_lossy().to_string();
        // 자동 요약용 격리 cwd에서 만들어진 jsonl은 목록에서 제외 (무한루프 방지)
        if project_dir.contains(crate::summary::ISOLATION_MARKER) {
            if let Some(f) = log.as_mut() {
                let _ = writeln!(f, "[skip-isolation] {}", project_dir);
            }
            continue;
        }

        let files = match fs::read_dir(&project_path) {
            Ok(f) => f,
            Err(e) => {
                if let Some(f) = log.as_mut() {
                    let _ = writeln!(f, "[ERR] read_dir({}) failed: {}", project_dir, e);
                }
                continue;
            }
        };

        let mut per_proj_found = 0usize;
        let mut per_proj_pushed = 0usize;
        for file in files {
            let Ok(file) = file else { continue };
            let path = file.path();
            if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }
            per_proj_found += 1;
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
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else {
                stem_fail += 1;
                if let Some(f) = log.as_mut() {
                    let _ = writeln!(f, "[SKIP stem] {}", path.display());
                }
                continue;
            };

            // 같은 session_id가 여러 루트에 있으면 먼저 본 것(primary 우선)만 사용
            if !seen_session_ids.insert(stem.clone()) {
                if let Some(f) = log.as_mut() {
                    let _ = writeln!(f, "[dup-skip] {} ({})", stem, path.display());
                }
                continue;
            }

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
            let saved_meta = saved.get(&stem).cloned().unwrap_or_default();
            let favorite = saved_meta.favorite.unwrap_or(false);

            per_proj_pushed += 1;
            total_pushed += 1;
            out.push(Session {
                session_id: stem,
                name: saved_meta.name,
                description: saved_meta.description,
                auto_summary: saved_meta.auto_summary,
                favorite,
                project: decode_project_name(&project_dir),
                project_dir: project_dir.clone(),
                file_path: path.to_string_lossy().to_string(),
                size: stat.len(),
                total_lines: meta.as_ref().map(|m| m.total_lines).unwrap_or(0),
                first_timestamp: meta.as_ref().and_then(|m| m.first_timestamp.clone()),
                last_timestamp: meta.as_ref().and_then(|m| m.last_timestamp.clone()),
                cwd: meta.as_ref().and_then(|m| m.cwd.clone()),
                version: meta.as_ref().and_then(|m| m.version.clone()),
                first_user_message: meta.as_ref().and_then(|m| m.first_user_message.clone()),
                storage_type: saved_meta.storage_type.unwrap_or_else(|| "local".into()),
                locked_by: None,
            });
        }

        if let Some(f) = log.as_mut() {
            let _ = writeln!(
                f,
                "[proj] {:>4} found / {:>4} pushed  -- {}",
                per_proj_found, per_proj_pushed, project_dir
            );
        }
    }
    } // end for root in &roots

    if let Some(f) = log.as_mut() {
        let _ = writeln!(f, "---");
        let _ = writeln!(f, "TOTAL found  = {}", total_found);
        let _ = writeln!(f, "TOTAL pushed = {}", total_pushed);
        let _ = writeln!(f, "stat_fail    = {}", stat_fail);
        let _ = writeln!(f, "stem_fail    = {}", stem_fail);
        let _ = writeln!(f, "meta_fail    = {}", meta_fail);
        let _ = writeln!(f, "out.len()    = {} (returned to frontend)", out.len());
    }

    out.sort_by(|a, b| {
        match b.favorite.cmp(&a.favorite) {
            std::cmp::Ordering::Equal => {
                let ta = a.last_timestamp.as_deref().unwrap_or("");
                let tb = b.last_timestamp.as_deref().unwrap_or("");
                tb.cmp(ta)
            }
            other => other,
        }
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
        if val.get("type").and_then(|v| v.as_str()) != Some("user") {
            continue;
        }
        if let Some(content) = val.pointer("/message/content") {
            if let Some(text) = extract_text(content) {
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
