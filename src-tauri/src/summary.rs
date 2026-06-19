use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const MODEL: &str = "gpt-5-codex";

/// 격리 cwd. 이 폴더에서 `codex exec`를 실행하면 별도 rollout이 만들어진다.
/// scanner는 이 폴더(이름에 ".summary-runs"가 포함된 project_dir)를 skip한다.
pub fn isolation_cwd() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(".codex-sessions").join(".summary-runs")
}

pub const ISOLATION_MARKER: &str = "summary-runs";

/// session 전체에서 헤드 N줄 + 테일 M줄만 추출해 본문으로 쓴다.
fn collect_excerpt(file_path: &str, head_n: usize, tail_n: usize) -> Result<String> {
    use std::io::{BufRead, BufReader};
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(|r| r.ok()).collect();
    let total = lines.len();

    let pick: Vec<&String> = if total <= head_n + tail_n {
        lines.iter().collect()
    } else {
        let head = &lines[..head_n];
        let tail = &lines[total - tail_n..];
        head.iter().chain(tail.iter()).collect()
    };

    let mut out = String::new();
    for line in pick {
        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ty = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let item = match ty {
            "user" | "assistant" => val
                .pointer("/message/content")
                .and_then(extract_text)
                .map(|text| (ty, text)),
            "event_msg" => {
                let payload = val.get("payload").unwrap_or(&serde_json::Value::Null);
                if payload.get("type").and_then(|v| v.as_str()) == Some("user_message") {
                    payload
                        .get("message")
                        .and_then(extract_text)
                        .map(|text| ("user", text))
                } else {
                    None
                }
            }
            "response_item" => {
                let payload = val.get("payload").unwrap_or(&serde_json::Value::Null);
                payload
                    .get("role")
                    .and_then(|v| v.as_str())
                    .filter(|role| *role == "user" || *role == "assistant")
                    .and_then(|role| {
                        payload
                            .get("content")
                            .and_then(extract_text)
                            .map(|text| (role, text))
                    })
            }
            _ => None,
        };
        let Some((role, text)) = item else { continue };
        if text.is_empty() {
            continue;
        }
        let snippet = text.chars().take(400).collect::<String>();
        out.push_str(&format!("[{}] {}\n", role, snippet));
        if out.len() > 20_000 {
            break;
        }
    }
    Ok(out)
}

fn extract_text(content: &serde_json::Value) -> Option<String> {
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        let mut buf = String::new();
        for item in arr {
            if matches!(
                item.get("type").and_then(|v| v.as_str()),
                Some("text") | Some("output_text") | Some("input_text")
            ) {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    buf.push_str(t);
                    buf.push('\n');
                }
            }
        }
        if !buf.is_empty() {
            return Some(buf);
        }
    }
    None
}

/// codex CLI를 격리 cwd에서 헤드리스(exec)로 호출한다.
/// 호출 직후 격리 폴더 내 모든 jsonl을 삭제하여 무한루프를 방지한다.
pub fn run_codex_headless(prompt: &str) -> Result<String> {
    let cwd = isolation_cwd();
    fs::create_dir_all(&cwd)?;

    // 호출 직전 스냅샷 (이후 새로 생긴 파일만 정리)
    let projects_root = crate::scanner::projects_dir();

    // codex CLI 절대 경로 (Tauri GUI 앱은 PATH가 불완전할 수 있음)
    let codex = crate::environment::locate_codex().ok_or_else(|| {
        crate::debuglog::log("summary", "ERROR: codex CLI not found anywhere");
        anyhow!("codex CLI를 찾을 수 없음. 설치 후 PATH 또는 npm global 위치에 있어야 함")
    })?;
    crate::debuglog::log("summary", &format!("codex path: {}", codex));

    let mut cmd = Command::new(&codex);
    cmd.arg("exec")
        .arg("--model")
        .arg(MODEL)
        .arg(prompt)
        .current_dir(&cwd);

    // Windows에서 콘솔 창 뜨는 거 방지
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().map_err(|e| {
        crate::debuglog::log("summary", &format!("ERROR spawn failed: {}", e));
        anyhow!("codex CLI 실행 실패 ({}): {}", codex, e)
    })?;

    // 격리 cwd로 만들어진 project 폴더 (이름에 .summary-runs 포함) 내부 jsonl 정리
    if projects_root.exists() {
        if let Ok(entries) = fs::read_dir(&projects_root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains(ISOLATION_MARKER) {
                    let p = entry.path();
                    if p.is_dir() {
                        if let Ok(files) = fs::read_dir(&p) {
                            for f in files.flatten() {
                                let fp = f.path();
                                if fp.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                                    let _ = fs::remove_file(&fp);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        crate::debuglog::log(
            "summary",
            &format!("ERROR codex exit {}: stderr={}", output.status, stderr),
        );
        return Err(anyhow!("codex exit {}: {}", output.status, stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    crate::debuglog::log("summary", &format!("OK ({} chars)", stdout.len()));
    Ok(stdout)
}

/// 여러 세션을 한 번의 codex 호출로 일괄 요약.
/// 입력: [(session_id, file_path)]
/// 반환: HashMap<session_id, (name, desc)>. 응답에 누락된 세션은 맵에서 빠진다.
pub fn auto_summarize_batch(
    items: &[(String, String)],
) -> Result<std::collections::HashMap<String, (String, String)>> {
    if items.is_empty() {
        return Ok(Default::default());
    }

    let mut body = String::new();
    body.push_str(
        "다음은 여러 Codex 세션의 발췌다. 각 세션마다 한국어로 NAME과 DESC를 정확한 형식으로 출력하라.\n\n",
    );
    body.push_str("출력 형식 (반드시 모든 세션에 대해 빠짐없이):\n");
    body.push_str("=== 1 ===\n");
    body.push_str("NAME: <12자 이내 짧은 제목, 따옴표 없이>\n");
    body.push_str("DESC: <100자 이내 한 문장 요약, 따옴표 없이>\n");
    body.push_str("=== 2 ===\n");
    body.push_str("NAME: ...\n");
    body.push_str("DESC: ...\n");
    body.push_str("...\n\n");
    body.push_str("세션 본문:\n\n");

    for (i, (_id, path)) in items.iter().enumerate() {
        let excerpt = collect_excerpt(path, 30, 20).unwrap_or_default();
        if excerpt.trim().is_empty() {
            body.push_str(&format!("--- {} ---\n(빈 세션)\n\n", i + 1));
        } else {
            body.push_str(&format!("--- {} ---\n{}\n\n", i + 1, excerpt));
        }
    }

    let out = run_codex_headless(&body)?;
    let mut result: std::collections::HashMap<String, (String, String)> = Default::default();

    // === N === 블록 단위 파싱
    let mut current_idx: Option<usize> = None;
    let mut current_name = String::new();
    let mut current_desc = String::new();

    let commit = |idx: Option<usize>,
                  name: &str,
                  desc: &str,
                  result: &mut std::collections::HashMap<String, (String, String)>| {
        if let Some(i) = idx {
            if i >= 1 && i <= items.len() && (!name.is_empty() || !desc.is_empty()) {
                let id = items[i - 1].0.clone();
                let n = if name.is_empty() {
                    desc.chars().take(12).collect()
                } else {
                    name.to_string()
                };
                result.insert(id, (n, desc.to_string()));
            }
        }
    };

    for line in out.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("===").and_then(|s| s.strip_suffix("===")) {
            commit(current_idx, &current_name, &current_desc, &mut result);
            current_idx = rest.trim().parse::<usize>().ok();
            current_name.clear();
            current_desc.clear();
        } else if let Some(rest) = line.strip_prefix("NAME:") {
            current_name = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("DESC:") {
            current_desc = rest.trim().trim_matches('"').to_string();
        }
    }
    commit(current_idx, &current_name, &current_desc, &mut result);

    Ok(result)
}

/// 세션 description+name 자동 생성.
/// previous_summary가 있으면 재생성으로 간주해 이전 내용을 피해 다른 관점으로 요약.
pub fn auto_summarize_session(
    file_path: &str,
    previous_summary: Option<&str>,
) -> Result<(String, String)> {
    let excerpt = collect_excerpt(file_path, 80, 40)?;
    if excerpt.trim().is_empty() {
        return Err(anyhow!("세션이 비어있음"));
    }

    let regen_hint = match previous_summary {
        Some(prev) if !prev.trim().is_empty() => format!(
            "\n\n이전 요약: \"{}\"\n위와 겹치지 않게 덜 강조한 부분/다른 관점으로 다시 요약하라.",
            prev
        ),
        _ => String::new(),
    };

    let prompt = format!(
        "다음은 Codex 세션의 일부 발췌다. 이 세션이 무엇에 관한 것인지 한국어로 두 줄만 출력하라.{}\n\n\
        형식:\n\
        NAME: <12자 이내 짧은 제목, 따옴표 없이>\n\
        DESC: <100자 이내 한 문장 요약, 따옴표 없이>\n\n\
        세션 발췌:\n{}",
        regen_hint, excerpt
    );

    let out = run_codex_headless(&prompt)?;
    let mut name = String::new();
    let mut desc = String::new();
    for line in out.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("NAME:") {
            name = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("DESC:") {
            desc = rest.trim().trim_matches('"').to_string();
        }
    }
    if name.is_empty() && desc.is_empty() {
        // 형식 무시하고 그냥 한 줄 떨어진 경우 — desc만 채움
        desc = out.lines().next().unwrap_or("").trim().to_string();
    }
    if name.is_empty() {
        // name 누락 시 desc 앞부분 사용
        name = desc.chars().take(12).collect();
    }
    Ok((name, desc))
}

#[cfg(test)]
mod tests {
    use super::collect_excerpt;
    use std::fs;

    #[test]
    fn collect_excerpt_reads_codex_user_and_assistant_records() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("session.jsonl");
        fs::write(
            &file,
            [
                r#"{"timestamp":"2026-06-19T00:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"사용자가 요청한 내용"}}"#,
                r#"{"timestamp":"2026-06-19T00:00:01Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"어시스턴트 응답"}]}}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let excerpt = collect_excerpt(file.to_str().unwrap(), 10, 10).unwrap();
        assert!(excerpt.contains("[user] 사용자가 요청한 내용"));
        assert!(excerpt.contains("[assistant] 어시스턴트 응답"));
    }
}
