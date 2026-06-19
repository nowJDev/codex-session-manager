// Codex 세션 내용을 읽어 이름과 요약을 생성한다.
use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const MODEL: &str = "gpt-5-codex";

/// Isolated cwd for `codex exec` summary runs.
/// The scanner skips projects whose path contains this marker.
pub fn isolation_cwd() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(".codex-sessions").join(".summary-runs")
}

pub const ISOLATION_MARKER: &str = "summary-runs";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexExecInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub prompt_on_stdin: bool,
}

pub fn build_codex_exec_invocation(codex: &str) -> CodexExecInvocation {
    CodexExecInvocation {
        program: codex.to_string(),
        args: vec![
            "exec".into(),
            "--model".into(),
            MODEL.into(),
            "-".into(),
        ],
        prompt_on_stdin: true,
    }
}

fn collect_excerpt(file_path: &str, head_n: usize, tail_n: usize) -> Result<String> {
    use std::io::{BufRead, BufReader};
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let total = lines.len();

    let picked: Vec<&String> = if total <= head_n + tail_n {
        lines.iter().collect()
    } else {
        lines[..head_n]
            .iter()
            .chain(lines[total - tail_n..].iter())
            .collect()
    };

    let mut out = String::new();
    for line in picked {
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
        if text.trim().is_empty() {
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

pub fn run_codex_headless(prompt: &str) -> Result<String> {
    let cwd = isolation_cwd();
    fs::create_dir_all(&cwd)?;

    let projects_root = crate::scanner::projects_dir();
    let codex = crate::environment::locate_codex().ok_or_else(|| {
        crate::debuglog::log("summary", "ERROR: codex CLI not found anywhere");
        anyhow!("codex CLI를 찾을 수 없습니다. Codex CLI 설치와 PATH를 확인하세요.")
    })?;
    let invocation = build_codex_exec_invocation(&codex);
    crate::debuglog::log(
        "summary",
        &format!("codex path: {}; args: {:?}", invocation.program, invocation.args),
    );

    let mut cmd = Command::new(&invocation.program);
    cmd.args(&invocation.args)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| {
        crate::debuglog::log("summary", &format!("ERROR spawn failed: {}", e));
        anyhow!("codex CLI 실행 실패 ({}): {}", invocation.program, e)
    })?;
    if invocation.prompt_on_stdin {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("codex stdin을 열 수 없습니다."))?;
        stdin.write_all(prompt.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    cleanup_summary_rollouts(&projects_root);

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

fn cleanup_summary_rollouts(projects_root: &std::path::Path) {
    if !projects_root.exists() {
        return;
    }
    let Ok(entries) = fs::read_dir(projects_root) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.contains(ISOLATION_MARKER) {
            continue;
        }
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
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

pub fn auto_summarize_batch(
    items: &[(String, String)],
) -> Result<std::collections::HashMap<String, (String, String)>> {
    if items.is_empty() {
        return Ok(Default::default());
    }

    let mut body = String::new();
    body.push_str("Summarize the following Codex sessions in Korean.\n");
    body.push_str("Return every session in this exact format, without markdown fences:\n\n");
    body.push_str("=== 1 ===\n");
    body.push_str("NAME: <short Korean title, 12 chars or fewer, no quotes>\n");
    body.push_str("DESC: <one Korean sentence, 100 chars or fewer, no quotes>\n");
    body.push_str("=== 2 ===\n");
    body.push_str("NAME: ...\n");
    body.push_str("DESC: ...\n\n");
    body.push_str("Sessions:\n\n");

    for (i, (_id, path)) in items.iter().enumerate() {
        let excerpt = collect_excerpt(path, 30, 20).unwrap_or_default();
        if excerpt.trim().is_empty() {
            body.push_str(&format!("--- {} ---\n(empty session)\n\n", i + 1));
        } else {
            body.push_str(&format!("--- {} ---\n{}\n\n", i + 1, excerpt));
        }
    }

    let out = run_codex_headless(&body)?;
    let mut result: std::collections::HashMap<String, (String, String)> = Default::default();

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

pub fn auto_summarize_session(
    file_path: &str,
    previous_summary: Option<&str>,
) -> Result<(String, String)> {
    let excerpt = collect_excerpt(file_path, 80, 40)?;
    if excerpt.trim().is_empty() {
        return Err(anyhow!("세션이 비어 있습니다."));
    }

    let regen_hint = match previous_summary {
        Some(prev) if !prev.trim().is_empty() => format!(
            "\nPrevious summary: \"{}\"\nUse a different angle and avoid repeating it.",
            prev
        ),
        _ => String::new(),
    };

    let prompt = format!(
        "Summarize this Codex session in Korean.{}\n\n\
        Output exactly two lines, without markdown fences:\n\
        NAME: <short Korean title, 12 chars or fewer, no quotes>\n\
        DESC: <one Korean sentence, 100 chars or fewer, no quotes>\n\n\
        Session excerpt:\n{}",
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
        desc = out.lines().next().unwrap_or("").trim().to_string();
    }
    if name.is_empty() {
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
                r#"{"timestamp":"2026-06-19T00:00:01Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"어시스턴트의 응답"}]}}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let excerpt = collect_excerpt(file.to_str().unwrap(), 10, 10).unwrap();
        assert!(excerpt.contains("[user] 사용자가 요청한 내용"));
        assert!(excerpt.contains("[assistant] 어시스턴트의 응답"));
    }
}
