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
                    &crate::environment::locate_codex().unwrap_or_else(|| "codex".into()),
                    &crate::scanner::codex_home().to_string_lossy(),
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
        DetectedTerminal {
            kind,
            program,
            display_name: kind.display_name().into(),
        }
    });
    let codex = crate::environment::locate_codex().unwrap_or_else(|| "codex".into());
    let mut plan =
        build_resume_command_with_codex(&term, session_id, cwd, flags.as_deref(), &codex);
    prepare_terminal_args(
        &mut plan,
        term.kind,
        &crate::scanner::codex_home().to_string_lossy(),
    );
    plan
}

fn prepare_terminal_args(plan: &mut ResumePlan, kind: TerminalKind, home: &str) {
    if let Some(command) = plan.args.last_mut() {
        match kind {
            TerminalKind::WindowsTerminal | TerminalKind::PowerShell => {
                *command = format!(
                    "$env:CODEX_HOME='{}'; {}",
                    home.replace('\'', "''"),
                    command
                );
            }
            TerminalKind::Cmd => {
                *command = format!("set \"CODEX_HOME={}\" && {}", home, command);
            }
            TerminalKind::GitBash | TerminalKind::LinuxDefault => {
                *command = format!(
                    "export CODEX_HOME='{}'; {}",
                    home.replace('\'', "'\\''"),
                    command
                );
            }
            TerminalKind::MacTerminal => {
                let export = format!("export CODEX_HOME='{}'; ", home.replace('\'', "'\\''"));
                let escaped = export.replace('\\', "\\\\").replace('"', "\\\"");
                *command =
                    command.replacen("to do script \"", &format!("to do script \"{}", escaped), 1);
            }
            TerminalKind::Custom => {}
        }
    }
    if kind == TerminalKind::WindowsTerminal {
        // wt.exe treats unescaped semicolons anywhere in its arguments as command separators.
        for arg in &mut plan.args {
            *arg = arg.replace(';', r"\;");
        }
    }
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
                if Command::new(term)
                    .env("CODEX_HOME", crate::scanner::codex_home())
                    .args(&args)
                    .spawn()
                    .is_ok()
                {
                    crate::debuglog::log(
                        "resume",
                        &format!("spawned fallback linux terminal: {}", term),
                    );
                    return Ok(());
                }
            }
            crate::debuglog::log("resume", "ERROR: no terminal emulator found");
            return Err(anyhow!("no terminal emulator found"));
        }
    }

    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args);
    cmd.env("CODEX_HOME", crate::scanner::codex_home());
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

#[cfg(test)]
mod tests {
    use super::{
        build_resume_command_with_codex, prepare_terminal_args, DetectedTerminal, TerminalKind,
    };

    #[test]
    fn windows_terminal_escapes_semicolons_without_changing_powershell() {
        let temp = tempfile::Builder::new()
            .prefix("work;dir")
            .tempdir()
            .unwrap();
        let cwd = temp.path().to_str().unwrap();
        let home = r"C:\Codex's;home";
        let codex = r"C:\Program Files\codex.exe";
        let flags = "--config 'value;next'";
        let script = r"$env:CODEX_HOME='C:\Codex''s;home'; & 'C:\Program Files\codex.exe' resume --config 'value;next' sid";

        let mut terminal = DetectedTerminal {
            kind: TerminalKind::WindowsTerminal,
            program: "wt.exe".into(),
            display_name: "Windows Terminal".into(),
        };
        let mut plan =
            build_resume_command_with_codex(&terminal, "sid", Some(cwd), Some(flags), codex);
        prepare_terminal_args(&mut plan, terminal.kind, home);
        assert_eq!(
            plan.args,
            [
                "new-tab".to_string(),
                "-d".into(),
                cwd.replace(';', r"\;"),
                "powershell".into(),
                "-NoExit".into(),
                "-Command".into(),
                script.replace(';', r"\;"),
            ]
        );

        terminal.kind = TerminalKind::PowerShell;
        terminal.program = "powershell.exe".into();
        let mut plan = build_resume_command_with_codex(&terminal, "sid", None, Some(flags), codex);
        prepare_terminal_args(&mut plan, terminal.kind, home);
        assert_eq!(plan.args, ["-NoExit", "-Command", script]);
    }

    #[test]
    fn windows_resume_plan_sets_home_inside_the_terminal_command() {
        let plan = super::build_resume_plan("sid", None, "windows");
        let command = plan.args.join(" ");
        assert!(
            command.contains("CODEX_HOME"),
            "terminal reuse must preserve the API home: {command}"
        );
    }
}
