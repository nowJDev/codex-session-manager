// 현재 Codex CLI와 로컬 설정에서 표시 가능한 상태 정보를 수집한다.
use crate::environment;
use crate::scanner;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLimitStatus {
    pub key: String,
    pub label: String,
    pub available: bool,
    pub value: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus {
    pub checked_at: String,
    pub cli_found: bool,
    pub cli_path: Option<String>,
    pub cli_version: Option<String>,
    pub model: Option<String>,
    pub model_reasoning_effort: Option<String>,
    pub status_line: Vec<String>,
    pub limits: Vec<CodexLimitStatus>,
    pub note: String,
}

#[derive(Default)]
struct CodexConfigSnapshot {
    model: Option<String>,
    model_reasoning_effort: Option<String>,
    status_line: Vec<String>,
}

fn unquote_toml_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        Some(trimmed[1..trimmed.len() - 1].to_string())
    } else {
        None
    }
}

fn parse_string_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Vec::new();
    }
    trimmed[1..trimmed.len() - 1]
        .split(',')
        .filter_map(unquote_toml_string)
        .collect()
}

fn read_config_snapshot() -> CodexConfigSnapshot {
    let path = scanner::codex_home().join("config.toml");
    let Ok(body) = fs::read_to_string(path) else {
        return CodexConfigSnapshot::default();
    };
    let mut snapshot = CodexConfigSnapshot::default();
    let mut section = String::new();

    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(&['[', ']'][..]).to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if section.is_empty() {
            match key {
                "model" => snapshot.model = unquote_toml_string(value),
                "model_reasoning_effort" => {
                    snapshot.model_reasoning_effort = unquote_toml_string(value);
                }
                _ => {}
            }
        } else if section == "tui" && key == "status_line" {
            snapshot.status_line = parse_string_array(value);
        }
    }

    snapshot
}

fn run_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(status) if status.success() => {
                let output = child.wait_with_output().ok()?;
                return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
            Some(_) => return None,
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn unavailable_limit(key: &str, label: &str) -> CodexLimitStatus {
    CodexLimitStatus {
        key: key.to_string(),
        label: label.to_string(),
        available: false,
        value: None,
        detail: "Codex CLI는 이 /status 값을 비대화형 JSON 명령으로 제공하지 않습니다.".to_string(),
    }
}

pub fn get_codex_status() -> CodexStatus {
    let snapshot = read_config_snapshot();
    let cli_path = environment::locate_codex();
    let cli_version = cli_path
        .as_deref()
        .and_then(|path| run_with_timeout(path, &["--version"], Duration::from_secs(5)));

    CodexStatus {
        checked_at: chrono::Utc::now().to_rfc3339(),
        cli_found: cli_path.is_some(),
        cli_path,
        cli_version,
        model: snapshot.model,
        model_reasoning_effort: snapshot.model_reasoning_effort,
        status_line: snapshot.status_line,
        limits: vec![
            unavailable_limit("contextUsed", "Context"),
            unavailable_limit("fiveHourLimit", "5h limit"),
            unavailable_limit("weeklyLimit", "Weekly limit"),
        ],
        note: "남은 사용량과 한도는 공식 Codex usage 페이지에서 확인하세요.".to_string(),
    }
}
