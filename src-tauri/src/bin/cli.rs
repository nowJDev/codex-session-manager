use claude_session_manager_lib::{cloud, config, environment, resume, scanner, summary, terminal, types::SessionMeta};
use std::process::ExitCode;

fn print_help() {
    eprintln!(
        "session-cli — headless harness for claude-session-manager\n\n\
USAGE:\n  \
  session-cli list                              List all local sessions (JSON)\n  \
  session-cli get-config                        Print current config (JSON)\n  \
  session-cli set-name <session-id> <name>      Save a name for a session\n  \
  session-cli set-desc <session-id> <desc>      Save a description\n  \
  session-cli delete-meta <session-id>          Remove saved metadata\n  \
  session-cli set-favorite <session-id> <0|1>   Toggle favorite flag\n  \
  session-cli auto-summarize <session-id>       Generate name+desc via claude -p (also saves)\n  \
  session-cli resume-plan <session-id> [cwd]    Print the resume command (no spawn)\n  \
  session-cli messages <file-path> [n]          Print first N user messages from a JSONL\n  \
  session-cli paths                             Print resolved paths (config, projects)\n  \
  session-cli detect-gdrive                     Detect Google Drive folder\n  \
  session-cli connect-gdrive                    Auto-connect Google Drive as cloud folder\n  \
  session-cli upload <session-id>               Upload session to cloud (deletes local)\n  \
  session-cli checkout <session-id>             Download from cloud to local + acquire lock\n  \
  session-cli checkin <session-id>              Re-upload local + release lock + delete local\n  \
  session-cli check-env                         Detect claude CLI + available terminals (JSON)\n  \
  session-cli set-terminal <kind|none>          Set preferred terminal (git-bash|wt|powershell|cmd|terminal|none)\n"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");

    let result: anyhow::Result<()> = (|| match cmd {
        "list" => {
            let sessions = scanner::scan_local_sessions()?;
            println!("{}", serde_json::to_string_pretty(&sessions)?);
            Ok(())
        }
        "get-config" => {
            let cfg = config::load_config();
            println!("{}", serde_json::to_string_pretty(&cfg)?);
            Ok(())
        }
        "set-name" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            let name = args.get(2).ok_or_else(|| anyhow::anyhow!("name required"))?;
            config::upsert_session_meta(
                id,
                SessionMeta { name: Some(name.clone()), ..Default::default() },
            )?;
            println!("ok");
            Ok(())
        }
        "set-desc" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            let desc = args.get(2).ok_or_else(|| anyhow::anyhow!("desc required"))?;
            config::upsert_session_meta(
                id,
                SessionMeta { description: Some(desc.clone()), ..Default::default() },
            )?;
            println!("ok");
            Ok(())
        }
        "delete-meta" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            config::delete_session_meta(id)?;
            println!("ok");
            Ok(())
        }
        "auto-summarize-batch" => {
            // 빈 description 세션 5개 골라서 배치 호출
            let n: usize = args.get(1).map(|s| s.parse().unwrap_or(5)).unwrap_or(5);
            let sessions = scanner::scan_local_sessions()?;
            let pending: Vec<(String, String)> = sessions
                .into_iter()
                .filter(|s| {
                    s.description.as_deref().unwrap_or("").is_empty()
                        && s.auto_summary.as_deref().unwrap_or("").is_empty()
                })
                .take(n)
                .map(|s| (s.session_id, s.file_path))
                .collect();
            eprintln!("배치 대상 {}개", pending.len());
            for (id, _) in &pending {
                eprintln!("  - {}", &id[..8]);
            }
            let result = summary::auto_summarize_batch(&pending)?;
            for (id, _) in &pending {
                if let Some((name, desc)) = result.get(id) {
                    config::upsert_session_meta(
                        id,
                        SessionMeta {
                            name: Some(name.clone()),
                            auto_summary: Some(desc.clone()),
                            ..Default::default()
                        },
                    )?;
                }
            }
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }
        "auto-summarize" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            // file_path 찾기
            let sessions = scanner::scan_local_sessions()?;
            let s = sessions
                .iter()
                .find(|s| s.session_id == *id)
                .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;
            let prev = s.description.clone().or(s.auto_summary.clone());
            let (name, desc) = summary::auto_summarize_session(&s.file_path, prev.as_deref())?;
            config::upsert_session_meta(
                id,
                SessionMeta {
                    name: Some(name.clone()),
                    auto_summary: Some(desc.clone()),
                    ..Default::default()
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "name": name,
                "description": desc,
            }))?);
            Ok(())
        }
        "set-favorite" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            let flag = args.get(2).map(|s| s.as_str()).unwrap_or("1");
            let val = matches!(flag, "1" | "true" | "yes" | "on");
            config::upsert_session_meta(
                id,
                SessionMeta { favorite: Some(val), ..Default::default() },
            )?;
            println!("ok");
            Ok(())
        }
        "resume-plan" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            let cwd = args.get(2).map(|s| s.as_str());
            let target = if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "macos") {
                "macos"
            } else {
                "linux"
            };
            let plan = resume::build_resume_plan(id, cwd, target);
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "program": plan.program,
                "args": plan.args,
                "target": target,
            }))?);
            Ok(())
        }
        "messages" => {
            let file = args.get(1).ok_or_else(|| anyhow::anyhow!("file-path required"))?;
            let n: usize = args.get(2).map(|s| s.parse().unwrap_or(5)).unwrap_or(5);
            let msgs = scanner::get_session_messages(file, n)?;
            println!("{}", serde_json::to_string_pretty(&msgs)?);
            Ok(())
        }
        "check-env" => {
            let report = environment::check_environment();
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        "set-terminal" => {
            let value = args.get(1).ok_or_else(|| anyhow::anyhow!("kind required (auto|git-bash|wt|powershell|cmd|terminal)"))?;
            if value != "auto" && terminal::TerminalKind::parse(value).is_none() {
                return Err(anyhow::anyhow!("unknown terminal kind: {}", value));
            }
            config::update_settings(claude_session_manager_lib::types::Settings {
                preferred_terminal: Some(value.clone()),
                ..Default::default()
            })?;
            println!("ok");
            Ok(())
        }
        "detect-gdrive" => {
            let r = cloud::detect_google_drive_result();
            println!("{}", serde_json::to_string_pretty(&r)?);
            Ok(())
        }
        "connect-gdrive" => {
            let p = cloud::detect_google_drive()
                .ok_or_else(|| anyhow::anyhow!("Google Drive 폴더를 찾지 못함"))?;
            let folder = cloud::set_cloud_root(&p.to_string_lossy())?;
            println!("connected: {}", folder.display());
            Ok(())
        }
        "upload" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            let sessions = scanner::scan_local_sessions()?;
            let s = sessions
                .iter()
                .find(|s| s.session_id == *id)
                .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;
            cloud::upload_session(s)?;
            println!("uploaded: {}", id);
            Ok(())
        }
        "checkout" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            let sessions = cloud::list_cloud_sessions()?;
            let s = sessions
                .iter()
                .find(|s| s.session_id == *id)
                .ok_or_else(|| anyhow::anyhow!("cloud session not found: {}", id))?;
            let path = cloud::checkout(s)?;
            println!("checked out to: {}", path);
            Ok(())
        }
        "checkin" => {
            let id = args.get(1).ok_or_else(|| anyhow::anyhow!("session-id required"))?;
            // 로컬 또는 클라우드 메타에서 정보 가져오기
            let cloud_list = cloud::list_cloud_sessions()?;
            let s = cloud_list
                .iter()
                .find(|s| s.session_id == *id)
                .ok_or_else(|| anyhow::anyhow!("cloud session not found: {}", id))?;
            cloud::checkin(s)?;
            println!("checked in: {}", id);
            Ok(())
        }
        "paths" => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "config_dir": config::config_dir(),
                "config_file": config::config_file(),
                "claude_dir": scanner::claude_dir(),
                "projects_dir": scanner::projects_dir(),
                "projects_roots": scanner::projects_roots(),
                "home_override": std::env::var("CLAUDE_SESSION_HOME").ok(),
            }))?);
            Ok(())
        }
        _ => {
            print_help();
            if cmd == "help" {
                Ok(())
            } else {
                Err(anyhow::anyhow!("__HELP_EXIT__"))
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
