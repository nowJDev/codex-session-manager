use claude_session_manager_lib::{
    config, environment, resume, scanner, terminal,
    terminal::{DetectedTerminal, TerminalKind},
    types::SessionMeta,
};
use std::fs;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct TempHome {
    _dir: tempfile::TempDir,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn setup_temp_home() -> TempHome {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    std::env::set_var("CLAUDE_SESSION_HOME", dir.path());
    TempHome { _dir: dir, _guard: guard }
}

fn write_jsonl(path: &std::path::Path, lines: &[&str]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n")).unwrap();
}

#[test]
fn config_roundtrip_creates_and_reads() {
    let _h = setup_temp_home();
    let cfg = config::load_config();
    assert!(cfg.sessions.is_empty());
    assert!(cfg.settings.locale.is_none());

    config::upsert_session_meta(
        "abc-123",
        SessionMeta {
            name: Some("my-feature".into()),
            description: Some("auth refactor".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let cfg2 = config::load_config();
    let entry = cfg2.sessions.get("abc-123").expect("entry exists");
    assert_eq!(entry.name.as_deref(), Some("my-feature"));
    assert_eq!(entry.description.as_deref(), Some("auth refactor"));
    assert!(entry.updated_at.is_some());
}

#[test]
fn config_partial_update_preserves_other_fields() {
    let _h = setup_temp_home();
    config::upsert_session_meta(
        "s1",
        SessionMeta {
            name: Some("name1".into()),
            description: Some("desc1".into()),
            ..Default::default()
        },
    )
    .unwrap();
    config::upsert_session_meta(
        "s1",
        SessionMeta { description: Some("desc2".into()), ..Default::default() },
    )
    .unwrap();

    let cfg = config::load_config();
    let e = cfg.sessions.get("s1").unwrap();
    assert_eq!(e.name.as_deref(), Some("name1"));
    assert_eq!(e.description.as_deref(), Some("desc2"));
}

#[test]
fn config_delete_removes_entry() {
    let _h = setup_temp_home();
    config::upsert_session_meta(
        "to-del",
        SessionMeta { name: Some("x".into()), ..Default::default() },
    )
    .unwrap();
    config::delete_session_meta("to-del").unwrap();
    let cfg = config::load_config();
    assert!(!cfg.sessions.contains_key("to-del"));
}

#[test]
fn settings_update_only_overwrites_provided_fields() {
    let _h = setup_temp_home();
    config::update_settings(claude_session_manager_lib::types::Settings {
        locale: Some("ko".into()),
        cloud_path: Some("/tmp/cloud".into()),
        ..Default::default()
    })
    .unwrap();
    config::update_settings(claude_session_manager_lib::types::Settings {
        locale: Some("en".into()),
        ..Default::default()
    })
    .unwrap();
    let cfg = config::load_config();
    assert_eq!(cfg.settings.locale.as_deref(), Some("en"));
    assert_eq!(cfg.settings.cloud_path.as_deref(), Some("/tmp/cloud"));
}

#[test]
fn scanner_returns_empty_when_no_projects_dir() {
    let _h = setup_temp_home();
    let sessions = scanner::scan_local_sessions().unwrap();
    assert!(sessions.is_empty());
}

#[test]
fn scanner_parses_jsonl_meta_correctly() {
    let _h = setup_temp_home();
    let projects = scanner::projects_dir();
    let project_dir = projects.join("C--Git--demo");
    let session_id = "11111111-2222-3333-4444-555555555555";
    let file = project_dir.join(format!("{}.jsonl", session_id));

    let lines = [
        r#"{"type":"user","timestamp":"2026-04-01T10:00:00Z","cwd":"C:/Git/demo","version":"1.0.0","message":{"content":"hello world"}}"#,
        r#"{"type":"assistant","timestamp":"2026-04-01T10:00:05Z","message":{"content":[{"type":"text","text":"hi"}]}}"#,
        r#"{"type":"user","timestamp":"2026-04-01T10:01:00Z","message":{"content":"second"}}"#,
    ];
    write_jsonl(&file, &lines);

    let sessions = scanner::scan_local_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    let s = &sessions[0];
    assert_eq!(s.session_id, session_id);
    assert_eq!(s.project_dir, "C--Git--demo");
    assert_eq!(s.project, "C:/Git/demo");
    assert_eq!(s.total_lines, 3);
    assert_eq!(s.first_timestamp.as_deref(), Some("2026-04-01T10:00:00Z"));
    assert_eq!(s.last_timestamp.as_deref(), Some("2026-04-01T10:01:00Z"));
    assert_eq!(s.cwd.as_deref(), Some("C:/Git/demo"));
    assert_eq!(s.version.as_deref(), Some("1.0.0"));
    assert_eq!(s.first_user_message.as_deref(), Some("hello world"));
    assert_eq!(s.storage_type, "local");
}

#[test]
fn scanner_extracts_text_from_array_content() {
    let _h = setup_temp_home();
    let projects = scanner::projects_dir();
    let file = projects.join("proj").join("sess.jsonl");
    let lines = [
        r#"{"type":"user","timestamp":"2026-04-01T10:00:00Z","message":{"content":[{"type":"text","text":"array text msg"}]}}"#,
    ];
    write_jsonl(&file, &lines);

    let msgs = scanner::get_session_messages(file.to_str().unwrap(), 5).unwrap();
    assert_eq!(msgs, vec!["array text msg".to_string()]);
}

#[test]
fn scanner_sorts_by_last_timestamp_desc() {
    let _h = setup_temp_home();
    let projects = scanner::projects_dir();
    let p = projects.join("proj");

    write_jsonl(
        &p.join("old.jsonl"),
        &[r#"{"type":"user","timestamp":"2026-01-01T00:00:00Z","message":{"content":"old"}}"#],
    );
    write_jsonl(
        &p.join("new.jsonl"),
        &[r#"{"type":"user","timestamp":"2026-04-01T00:00:00Z","message":{"content":"new"}}"#],
    );

    let sessions = scanner::scan_local_sessions().unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].session_id, "new");
    assert_eq!(sessions[1].session_id, "old");
}

#[test]
fn scanner_merges_saved_metadata() {
    let _h = setup_temp_home();
    let projects = scanner::projects_dir();
    let file = projects.join("proj").join("with-meta.jsonl");
    write_jsonl(
        &file,
        &[r#"{"type":"user","timestamp":"2026-04-01T00:00:00Z","message":{"content":"hi"}}"#],
    );

    config::upsert_session_meta(
        "with-meta",
        SessionMeta {
            name: Some("nice-name".into()),
            description: Some("nice-desc".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let sessions = scanner::scan_local_sessions().unwrap();
    let s = sessions.iter().find(|s| s.session_id == "with-meta").unwrap();
    assert_eq!(s.name.as_deref(), Some("nice-name"));
    assert_eq!(s.description.as_deref(), Some("nice-desc"));
}

#[test]
fn scanner_skips_malformed_jsonl_lines() {
    let _h = setup_temp_home();
    let projects = scanner::projects_dir();
    let file = projects.join("proj").join("mixed.jsonl");
    let lines = [
        "not json at all",
        r#"{"type":"user","timestamp":"2026-04-01T10:00:00Z","message":{"content":"good"}}"#,
        "{broken",
    ];
    write_jsonl(&file, &lines);

    let sessions = scanner::scan_local_sessions().unwrap();
    let s = sessions.iter().find(|s| s.session_id == "mixed").unwrap();
    assert_eq!(s.total_lines, 3);
    assert_eq!(s.first_user_message.as_deref(), Some("good"));
}

#[test]
fn scanner_delete_removes_jsonl_file() {
    let _h = setup_temp_home();
    let projects = scanner::projects_dir();
    let file = projects.join("proj").join("doomed.jsonl");
    write_jsonl(
        &file,
        &[r#"{"type":"user","timestamp":"2026-04-01T00:00:00Z","message":{"content":"x"}}"#],
    );
    assert!(file.exists());

    scanner::delete_session_file(file.to_str().unwrap()).unwrap();
    assert!(!file.exists());
}

#[test]
fn resume_plan_windows_with_git_bash_or_cmd() {
    let plan = resume::build_resume_plan("sess-id", Some("C:/some/path"), "windows");
    assert!(plan.args.iter().any(|a| a.contains("claude --resume sess-id")));
}

#[test]
fn resume_plan_macos_uses_osascript() {
    let plan = resume::build_resume_plan("sid", None, "macos");
    assert_eq!(plan.program, "osascript");
    assert!(plan.args[0] == "-e");
    assert!(plan.args[1].contains("claude --resume sid"));
}

#[test]
fn resume_plan_linux_includes_bash_command() {
    let plan = resume::build_resume_plan("xyz", None, "linux");
    assert_eq!(plan.args[0], "-e");
    assert_eq!(plan.args[1], "bash");
    assert!(plan.args.last().unwrap().contains("claude --resume xyz"));
}

#[test]
fn resume_plan_skips_cwd_if_path_missing() {
    let plan = resume::build_resume_plan("sid", Some("/definitely/not/here/xyz123"), "linux");
    let joined = plan.args.join(" ");
    assert!(!joined.contains("/definitely/not/here"), "missing path should be filtered");
}

fn make_term(kind: TerminalKind, program: &str) -> DetectedTerminal {
    DetectedTerminal {
        kind,
        program: program.into(),
        display_name: kind.display_name().into(),
    }
}

#[test]
fn build_command_windows_terminal_uses_new_tab_with_dir() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().to_string_lossy().to_string();
    let term = make_term(TerminalKind::WindowsTerminal, "wt.exe");
    let plan = terminal::build_resume_command(&term, "abc-123", Some(&cwd), None);
    assert_eq!(plan.program, "wt.exe");
    assert!(plan.args.contains(&"new-tab".to_string()));
    assert!(plan.args.contains(&"-d".to_string()));
    assert!(plan.args.contains(&cwd));
    assert!(plan.args.iter().any(|a| a.contains("claude --resume abc-123")));
}

#[test]
fn build_command_powershell_uses_set_location_and_noexit() {
    let term = make_term(TerminalKind::PowerShell, "powershell.exe");
    let plan = terminal::build_resume_command(&term, "sid", None, None);
    assert_eq!(plan.args[0], "-NoExit");
    assert_eq!(plan.args[1], "-Command");
    assert!(plan.args[2].contains("claude --resume sid"));
}

#[test]
fn build_command_cmd_uses_slash_k() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().to_string_lossy().to_string();
    let term = make_term(TerminalKind::Cmd, "cmd");
    let plan = terminal::build_resume_command(&term, "sid", Some(&cwd), None);
    assert_eq!(plan.args[0], "/k");
    assert!(plan.args[1].contains("cd /d"));
    assert!(plan.args[1].contains("claude --resume sid"));
}

#[test]
fn terminal_kind_parse_aliases() {
    assert_eq!(TerminalKind::parse("git-bash"), Some(TerminalKind::GitBash));
    assert_eq!(TerminalKind::parse("gitbash"), Some(TerminalKind::GitBash));
    assert_eq!(TerminalKind::parse("wt"), Some(TerminalKind::WindowsTerminal));
    assert_eq!(TerminalKind::parse("windows-terminal"), Some(TerminalKind::WindowsTerminal));
    assert_eq!(TerminalKind::parse("pwsh"), Some(TerminalKind::PowerShell));
    assert_eq!(TerminalKind::parse("cmd"), Some(TerminalKind::Cmd));
    assert_eq!(TerminalKind::parse("auto"), None);
    assert_eq!(TerminalKind::parse("nonsense"), None);
}

#[test]
fn settings_persist_preferred_terminal() {
    let _h = setup_temp_home();
    config::update_settings(claude_session_manager_lib::types::Settings {
        preferred_terminal: Some("git-bash".into()),
        ..Default::default()
    })
    .unwrap();
    let cfg = config::load_config();
    assert_eq!(cfg.settings.preferred_terminal.as_deref(), Some("git-bash"));
}

#[test]
fn environment_check_returns_consistent_target() {
    let report = environment::check_environment();
    assert!(["windows", "macos", "linux"].contains(&report.target_os.as_str()));
    // claude_cli_found must agree with claude_cli_path being Some/None
    assert_eq!(report.claude_cli_found, report.claude_cli_path.is_some());
}
