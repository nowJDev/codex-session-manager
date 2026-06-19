use crate::terminal::{detect_all_terminals, DetectedTerminal};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub target_os: String,
    pub codex_cli_found: bool,
    pub codex_cli_path: Option<String>,
    pub codex_cli_version: Option<String>,
    pub terminals: Vec<DetectedTerminal>,
}

fn current_target_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn locate_in_path(name: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for ext in ["", ".exe", ".cmd", ".bat"] {
            let cand = dir.join(format!("{}{}", name, ext));
            if cand.exists() {
                return Some(cand.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// codex CLI를 찾는다. PATH 검색이 실패하면(Tauri GUI 앱은 사용자 PATH를
/// 못 받는 경우가 있음) 일반적인 설치 위치도 직접 검사한다.
pub fn locate_codex() -> Option<String> {
    // 1순위: 환경변수
    if let Ok(p) = std::env::var("CODEX_CLI") {
        if std::path::Path::new(&p).exists() {
            return Some(p);
        }
    }
    // 2순위: PATH
    if let Some(p) = locate_in_path("codex") {
        return Some(p);
    }
    // 3순위: 알려진 설치 위치들
    if let Some(home) = dirs::home_dir() {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();
        candidates.push(home.join(".local").join("bin").join("codex.exe"));
        candidates.push(home.join(".local").join("bin").join("codex"));
        // npm global (사용자)
        candidates.push(home.join("AppData").join("Roaming").join("npm").join("codex.cmd"));
        candidates.push(home.join("AppData").join("Roaming").join("npm").join("codex"));
        // pnpm
        candidates.push(home.join("AppData").join("Local").join("pnpm").join("codex.cmd"));
        // yarn global
        candidates.push(home.join("AppData").join("Local").join("Yarn").join("bin").join("codex.cmd"));

        // macOS / Linux Homebrew
        candidates.push(std::path::PathBuf::from("/usr/local/bin/codex"));
        candidates.push(std::path::PathBuf::from("/opt/homebrew/bin/codex"));

        for c in candidates {
            if c.exists() {
                return Some(c.to_string_lossy().to_string());
            }
        }
    }
    None
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

pub fn check_environment() -> EnvironmentReport {
    let target_os = current_target_os();
    let codex_path = locate_codex();
    let codex_found = codex_path.is_some();
    let codex_version = if let Some(p) = &codex_path {
        run_with_timeout(p, &["--version"], Duration::from_secs(5))
    } else {
        None
    };
    let terminals = detect_all_terminals(target_os);

    EnvironmentReport {
        target_os: target_os.to_string(),
        codex_cli_found: codex_found,
        codex_cli_path: codex_path,
        codex_cli_version: codex_version,
        terminals,
    }
}
