use crate::config::load_config;
use crate::terminal::{
    build_custom_resume_command, build_resume_command_with_codex, detect_all_terminals,
    pick_terminal, DetectedTerminal, ResumePlan, TerminalKind,
};
use anyhow::{anyhow, Result};
use std::process::Command;

pub fn current_target_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

pub fn build_resume_plan(session_id: &str, cwd: Option<&str>, target_os: &str) -> ResumePlan {
    let settings = load_config().settings;
    let preferred = settings.preferred_terminal.clone();
    let flags = settings.resume_flags.clone();

    // Custom 터미널 우선 처리
    if preferred.as_deref() == Some("custom") {
        if let (Some(program), Some(args_tpl)) = (
            settings.custom_terminal_program.as_deref(),
            settings.custom_terminal_args.as_deref(),
        ) {
            if !program.trim().is_empty() {
                return build_custom_resume_command(
                    program,
                    args_tpl,
                    session_id,
                    cwd,
                    flags.as_deref(),
                );
            }
        }
    }

    let term = pick_terminal(target_os, preferred.as_deref()).unwrap_or_else(|| {
        // Pick a sensible non-detecting fallback so we still emit a plan even
        // when no real terminal is installed (mostly relevant in tests).
        let kind = match target_os {
            "windows" => TerminalKind::Cmd,
            "macos" => TerminalKind::MacTerminal,
            _ => TerminalKind::LinuxDefault,
        };
        let program = match kind {
            TerminalKind::Cmd => "cmd".to_string(),
            TerminalKind::MacTerminal => "osascript".to_string(),
            _ => "x-terminal-emulator".to_string(),
        };
        DetectedTerminal { kind, program, display_name: kind.display_name().into() }
    });
    let codex = crate::environment::locate_codex().unwrap_or_else(|| "codex".into());
    build_resume_command_with_codex(&term, session_id, cwd, flags.as_deref(), &codex)
}

pub fn resume_in_new_terminal(session_id: &str, cwd: Option<&str>) -> Result<()> {
    let target = current_target_os();
    let plan = build_resume_plan(session_id, cwd, target);

    // 디버그 로그: 어떤 명령이 spawn되는지 전부 기록
    let env = crate::environment::check_environment();
    crate::debuglog::log(
        "resume",
        &format!(
            "session={} cwd={:?} target={} codex_version={:?} codex_path={:?}\n  program: {}\n  args: {:?}",
            session_id,
            cwd,
            target,
            env.codex_cli_version,
            env.codex_cli_path,
            plan.program,
            plan.args,
        ),
    );

    if target == "linux" {
        let detected = detect_all_terminals(target);
        if detected.is_empty() {
            for term in &["x-terminal-emulator", "gnome-terminal", "konsole", "xterm"] {
                let args: Vec<&str> = plan.args.iter().map(|s| s.as_str()).collect();
                if Command::new(term).args(&args).spawn().is_ok() {
                    crate::debuglog::log("resume", &format!("spawned fallback linux terminal: {}", term));
                    return Ok(());
                }
            }
            crate::debuglog::log("resume", "ERROR: no terminal emulator found");
            return Err(anyhow!("no terminal emulator found"));
        }
    }

    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        cmd.creation_flags(CREATE_NEW_CONSOLE);
    }

    match cmd.spawn() {
        Ok(_) => {
            crate::debuglog::log("resume", "spawn OK");
            Ok(())
        }
        Err(e) => {
            crate::debuglog::log("resume", &format!("ERROR spawn failed: {}", e));
            Err(anyhow!(e))
        }
    }
}
