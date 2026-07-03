// 시스템 진단과 OS 상호작용을 애플리케이션 outbound port 구현으로 감싼다.
use crate::application::ports::{
    CodexLimitStatus, CodexStatus, DebugLogInfo, DetectedTerminalInfo, DriveDetectResult,
    EnvironmentReport, PathsInfo, SystemPorts, UpdateInfo,
};
use anyhow::{anyhow, Result};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultSystemPorts;

impl SystemPorts for DefaultSystemPorts {
    fn debug_log_info(&self) -> DebugLogInfo {
        let path = crate::debuglog::log_path();
        let exists = path.exists();
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let tail = if exists {
            std::fs::read_to_string(&path)
                .map(|s| {
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

    fn open_debug_log_folder(&self) -> Result<()> {
        let path = crate::debuglog::log_path();
        let folder = path.parent().ok_or_else(|| anyhow!("no parent dir"))?;
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer").arg(folder).spawn()?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open").arg(folder).spawn()?;
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open").arg(folder).spawn()?;
        }
        Ok(())
    }

    fn detect_google_drive(&self) -> DriveDetectResult {
        let result = crate::cloud::detect_google_drive_result();
        DriveDetectResult {
            found: result.found,
            path: result.path,
        }
    }

    fn connect_google_drive(&self) -> Result<String> {
        let path = crate::cloud::detect_google_drive().ok_or_else(|| {
            anyhow!("Google Drive 폴더를 찾지 못했어요. 데스크탑 클라이언트가 설치돼 있나요?")
        })?;
        crate::cloud::set_cloud_root(&path.to_string_lossy())
            .map(|p| p.to_string_lossy().to_string())
    }

    fn check_environment(&self) -> EnvironmentReport {
        let report = crate::environment::check_environment();
        EnvironmentReport {
            target_os: report.target_os,
            codex_cli_found: report.codex_cli_found,
            codex_cli_path: report.codex_cli_path,
            codex_cli_version: report.codex_cli_version,
            terminals: report
                .terminals
                .into_iter()
                .map(|terminal| DetectedTerminalInfo {
                    kind: terminal_kind_value(terminal.kind).into(),
                    program: terminal.program,
                    display_name: terminal.display_name,
                })
                .collect(),
        }
    }

    fn get_codex_status(&self) -> CodexStatus {
        let status = crate::codex_status::get_codex_status();
        CodexStatus {
            checked_at: status.checked_at,
            cli_found: status.cli_found,
            cli_path: status.cli_path,
            cli_version: status.cli_version,
            model: status.model,
            model_reasoning_effort: status.model_reasoning_effort,
            status_line: status.status_line,
            limits: status
                .limits
                .into_iter()
                .map(|limit| CodexLimitStatus {
                    key: limit.key,
                    label: limit.label,
                    available: limit.available,
                    value: limit.value,
                    detail: limit.detail,
                })
                .collect(),
            note: status.note,
        }
    }

    fn check_update(&self) -> Pin<Box<dyn Future<Output = Result<UpdateInfo>> + Send + '_>> {
        Box::pin(async {
            let update = crate::update::check_latest_release().await?;
            Ok(UpdateInfo {
                current_version: update.current_version,
                latest_version: update.latest_version,
                has_update: update.has_update,
                release_url: update.release_url,
            })
        })
    }

    fn paths_info(&self) -> PathsInfo {
        PathsInfo {
            config_dir: crate::config::config_dir().to_string_lossy().to_string(),
            config_file: crate::config::config_file().to_string_lossy().to_string(),
            codex_home: crate::scanner::codex_home().to_string_lossy().to_string(),
            sessions_dir: crate::scanner::sessions_dir().to_string_lossy().to_string(),
            archived_sessions_dir: crate::scanner::archived_sessions_dir()
                .to_string_lossy()
                .to_string(),
            projects_roots: crate::scanner::projects_roots()
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            home_override: std::env::var("CODEX_SESSION_HOME").ok(),
            codex_home_override: std::env::var("CODEX_HOME").ok(),
        }
    }
}

fn terminal_kind_value(kind: crate::terminal::TerminalKind) -> &'static str {
    match kind {
        crate::terminal::TerminalKind::GitBash => "git-bash",
        crate::terminal::TerminalKind::WindowsTerminal => "wt",
        crate::terminal::TerminalKind::PowerShell => "powershell",
        crate::terminal::TerminalKind::Cmd => "cmd",
        crate::terminal::TerminalKind::MacTerminal => "terminal",
        crate::terminal::TerminalKind::LinuxDefault => "linux-default",
        crate::terminal::TerminalKind::Custom => "custom",
    }
}
