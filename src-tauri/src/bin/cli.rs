// CLI 하네스 요청을 애플리케이션 서비스로 위임한다.
use anyhow::{anyhow, Result};
use codex_session_manager_lib::{
    adapters::outbound::session_ports::DefaultSessionPorts,
    adapters::outbound::system_ports::DefaultSystemPorts,
    application::session_service::SessionService,
    application::system_service::SystemService,
    types::{Session, SessionMeta, Settings},
};
use std::process::ExitCode;

fn print_help() {
    eprintln!(
        "session-cli — headless harness for codex-session-manager\n\n\
USAGE:\n  \
  session-cli list                              List all local sessions (JSON)\n  \
  session-cli get-config                        Print current config (JSON)\n  \
  session-cli set-name <session-id> <name>      Save a name for a session\n  \
  session-cli set-desc <session-id> <desc>      Save a description\n  \
  session-cli delete <session-id>               Delete a Codex session\n  \
  session-cli delete-meta <session-id>          Remove saved metadata\n  \
  session-cli set-favorite <session-id> <0|1>   Toggle favorite flag\n  \
  session-cli archive <session-id>              Archive a Codex session\n  \
  session-cli unarchive <session-id>            Unarchive a Codex session\n  \
  session-cli auto-summarize <session-id>       Generate name+desc via codex exec (also saves)\n  \
  session-cli resume-plan <session-id> [cwd]    Print the resume command (no spawn)\n  \
  session-cli messages <file-path> [n]          Print first N user messages from a JSONL\n  \
  session-cli paths                             Print resolved paths (config, projects)\n  \
  session-cli detect-gdrive                     Detect Google Drive folder\n  \
  session-cli connect-gdrive                    Auto-connect Google Drive as cloud folder\n  \
  session-cli upload <session-id>               Upload session to cloud\n  \
  session-cli checkout <session-id>             Download from cloud to local + acquire lock\n  \
  session-cli checkin <session-id>              Re-upload local + release lock + delete local\n  \
  session-cli check-env                         Detect codex CLI + available terminals (JSON)\n  \
  session-cli set-terminal <kind|none>          Set preferred terminal (git-bash|wt|powershell|cmd|terminal|none)\n"
    );
}

fn session_service() -> SessionService<DefaultSessionPorts> {
    SessionService::new(DefaultSessionPorts)
}

fn system_service() -> SystemService<DefaultSystemPorts> {
    SystemService::new(DefaultSystemPorts)
}

fn required_arg(args: &[String], index: usize, name: &str) -> Result<String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| anyhow!("{name} required"))
}

fn find_session(
    service: &SessionService<DefaultSessionPorts>,
    session_id: &str,
) -> Result<Session> {
    service
        .list_sessions()?
        .into_iter()
        .find(|session| session.session_id == session_id)
        .ok_or_else(|| anyhow!("session not found: {}", session_id))
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

fn validate_terminal(value: &str) -> Result<()> {
    match value {
        "auto" | "git-bash" | "gitbash" | "bash" | "wt" | "windows-terminal" | "powershell"
        | "pwsh" | "cmd" | "terminal" | "mac-terminal" | "linux-default" | "custom" => Ok(()),
        _ => Err(anyhow!("unknown terminal kind: {}", value)),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");

    let result: Result<()> = (|| match cmd {
        "list" => {
            let service = session_service();
            println!(
                "{}",
                serde_json::to_string_pretty(&service.list_sessions()?)?
            );
            Ok(())
        }
        "get-config" => {
            let service = session_service();
            println!("{}", serde_json::to_string_pretty(&service.get_config())?);
            Ok(())
        }
        "set-name" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let name = required_arg(&args, 2, "name")?;
            service.save_session_meta(
                &id,
                SessionMeta {
                    name: Some(name),
                    ..Default::default()
                },
            )?;
            println!("ok");
            Ok(())
        }
        "set-desc" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let desc = required_arg(&args, 2, "desc")?;
            service.save_session_meta(
                &id,
                SessionMeta {
                    description: Some(desc),
                    ..Default::default()
                },
            )?;
            println!("ok");
            Ok(())
        }
        "delete-meta" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            service.delete_session_meta(&id)?;
            println!("ok");
            Ok(())
        }
        "delete" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let session = find_session(&service, &id)?;
            service.delete_session(&id, &session.file_path)?;
            println!("deleted: {}", id);
            Ok(())
        }
        "auto-summarize-batch" => {
            let service = session_service();
            let n: usize = args.get(1).map(|s| s.parse().unwrap_or(5)).unwrap_or(5);
            let pending = service.pending_auto_summary_batch(n)?;
            eprintln!("배치 대상 {}개", pending.len());
            for (id, _) in &pending {
                eprintln!("  - {}", id.chars().take(8).collect::<String>());
            }
            let result = service.summarize_batch(&pending)?;
            for (id, _) in &pending {
                if let Some((name, desc)) = result.get(id) {
                    service.save_summary(id, name.clone(), desc.clone())?;
                }
            }
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }
        "auto-summarize" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let session = find_session(&service, &id)?;
            let (name, desc) = service.generate_summary_pair(&id, &session.file_path)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "name": name,
                    "description": desc,
                }))?
            );
            Ok(())
        }
        "set-favorite" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let flag = args.get(2).map(|s| s.as_str()).unwrap_or("1");
            let val = matches!(flag, "1" | "true" | "yes" | "on");
            service.save_session_meta(
                &id,
                SessionMeta {
                    favorite: Some(val),
                    ..Default::default()
                },
            )?;
            println!("ok");
            Ok(())
        }
        "archive" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            service.archive_session(&id)?;
            println!("archived: {}", id);
            Ok(())
        }
        "unarchive" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            service.unarchive_session(&id)?;
            println!("unarchived: {}", id);
            Ok(())
        }
        "resume-plan" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let cwd = args.get(2).map(|s| s.as_str());
            let plan = service.build_resume_plan(&id, cwd)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "program": plan.program,
                    "args": plan.args,
                    "target": current_target_os(),
                }))?
            );
            Ok(())
        }
        "messages" => {
            let service = session_service();
            let file = required_arg(&args, 1, "file-path")?;
            let n: usize = args.get(2).map(|s| s.parse().unwrap_or(5)).unwrap_or(5);
            let msgs = service.get_session_messages(&file, n)?;
            println!("{}", serde_json::to_string_pretty(&msgs)?);
            Ok(())
        }
        "check-env" => {
            let service = system_service();
            println!(
                "{}",
                serde_json::to_string_pretty(&service.check_environment())?
            );
            Ok(())
        }
        "set-terminal" => {
            let service = session_service();
            let value = required_arg(&args, 1, "kind")?;
            validate_terminal(&value)?;
            service.save_settings(Settings {
                preferred_terminal: Some(value),
                ..Default::default()
            })?;
            println!("ok");
            Ok(())
        }
        "detect-gdrive" => {
            let service = system_service();
            println!(
                "{}",
                serde_json::to_string_pretty(&service.detect_google_drive())?
            );
            Ok(())
        }
        "connect-gdrive" => {
            let service = system_service();
            let folder = service.connect_google_drive()?;
            println!("connected: {}", folder);
            Ok(())
        }
        "upload" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let session = find_session(&service, &id)?;
            service.upload_to_cloud(&session)?;
            println!("uploaded: {}", id);
            Ok(())
        }
        "checkout" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let session = find_session(&service, &id)?;
            let path = service.checkout_session(&session)?;
            println!("checked out to: {}", path);
            Ok(())
        }
        "checkin" => {
            let service = session_service();
            let id = required_arg(&args, 1, "session-id")?;
            let session = find_session(&service, &id)?;
            service.checkin_session(&session)?;
            println!("checked in: {}", id);
            Ok(())
        }
        "paths" => {
            let service = system_service();
            println!("{}", serde_json::to_string_pretty(&service.paths_info())?);
            Ok(())
        }
        _ => {
            print_help();
            if cmd == "help" {
                Ok(())
            } else {
                Err(anyhow!("__HELP_EXIT__"))
            }
        }
    })();

    if let Err(ref e) = result {
        if e.to_string() == "__HELP_EXIT__" {
            return ExitCode::from(2);
        }
    }

    if let Err(e) = result {
        eprintln!("error: {:#}", e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
