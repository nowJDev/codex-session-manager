use codex_session_manager_lib::{
    cloud, config, environment, resume, scanner, summary, terminal,
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
    std::env::set_var("CODEX_SESSION_HOME", dir.path());
    std::env::set_var("CODEX_HOME", dir.path().join(".codex"));
    std::env::remove_var("CLAUDE_SESSION_HOME");
    TempHome { _dir: dir, _guard: guard }
}

fn write_jsonl(path: &std::path::Path, lines: &[&str]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n")).unwrap();
}

fn codex_rollout_path(home: &TempHome, date: &str, session_id: &str) -> std::path::PathBuf {
    let _ = home;
    let parts: Vec<&str> = date.split('-').collect();
    scanner::sessions_dir()
        .join(parts[0])
        .join(parts[1])
        .join(parts[2])
        .join(format!("rollout-{}T10-00-00-{}.jsonl", date, session_id))
}

fn archived_codex_rollout_path(home: &TempHome, date: &str, session_id: &str) -> std::path::PathBuf {
    let _ = home;
    let parts: Vec<&str> = date.split('-').collect();
    scanner::archived_sessions_dir()
        .join(parts[0])
        .join(parts[1])
        .join(parts[2])
        .join(format!("rollout-{}T10-00-00-{}.jsonl", date, session_id))
}

#[cfg(target_os = "windows")]
fn write_fake_codex(dir: &std::path::Path, log_path: &std::path::Path, target_path: Option<&std::path::Path>) -> std::path::PathBuf {
    let cli = dir.join("codex.cmd");
    let target = target_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    fs::write(
        &cli,
        format!(
            "@echo off\r\necho %*>>\"{}\"\r\nif \"%1\"==\"delete\" del \"{}\"\r\nexit /b 0\r\n",
            log_path.display(),
            target
        ),
    )
    .unwrap();
    cli
}

#[cfg(not(target_os = "windows"))]
fn write_fake_codex(dir: &std::path::Path, log_path: &std::path::Path, target_path: Option<&std::path::Path>) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let cli = dir.join("codex");
    let target = target_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    fs::write(
        &cli,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"$1\" = delete ]; then rm -f '{}'; fi\nexit 0\n",
            log_path.display(),
            target
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(&cli).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&cli, perms).unwrap();
    cli
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
fn config_drops_legacy_anthropic_api_key_on_save() {
    let _h = setup_temp_home();
    fs::create_dir_all(config::config_dir()).unwrap();
    fs::write(
        config::config_file(),
        r#"{"sessions":{},"settings":{"locale":"ko","anthropicApiKey":"legacy-secret"}}"#,
    )
    .unwrap();

    let cfg = config::load_config();
    config::save_config(&cfg).unwrap();
    let body = fs::read_to_string(config::config_file()).unwrap();
    assert!(!body.contains("anthropicApiKey"));
    assert!(body.contains("\"locale\": \"ko\""));
}

#[test]
fn settings_update_only_overwrites_provided_fields() {
    let _h = setup_temp_home();
    config::update_settings(codex_session_manager_lib::types::Settings {
        locale: Some("ko".into()),
        cloud_path: Some("/tmp/cloud".into()),
        ..Default::default()
    })
    .unwrap();
    config::update_settings(codex_session_manager_lib::types::Settings {
        locale: Some("en".into()),
        ..Default::default()
    })
    .unwrap();
    let cfg = config::load_config();
    assert_eq!(cfg.settings.locale.as_deref(), Some("en"));
    assert_eq!(cfg.settings.cloud_path.as_deref(), Some("/tmp/cloud"));
}

#[test]
fn settings_update_persists_excluded_scan_paths() {
    // 회귀 방지: v0.4.7에서 추가한 excludedScanPaths 필드가 update_settings의 분기에
    // 빠져있어 저장이 안 되던 버그를 재현. v0.4.8에서 수정.
    let _h = setup_temp_home();
    config::update_settings(codex_session_manager_lib::types::Settings {
        excluded_scan_paths: Some(vec!["currency-edge".into(), "other-bot".into()]),
        ..Default::default()
    })
    .unwrap();
    let cfg = config::load_config();
    assert_eq!(
        cfg.settings.excluded_scan_paths.as_deref(),
        Some(&["currency-edge".to_string(), "other-bot".to_string()][..])
    );

    // 빈 배열로 클리어도 가능해야 함
    config::update_settings(codex_session_manager_lib::types::Settings {
        excluded_scan_paths: Some(vec![]),
        ..Default::default()
    })
    .unwrap();
    let cfg = config::load_config();
    assert_eq!(cfg.settings.excluded_scan_paths.as_deref(), Some(&[][..]));
}

#[test]
fn scanner_skips_excluded_scan_paths() {
    let h = setup_temp_home();
    let excluded_session = "aaaaaaaa-1111-2222-3333-444444444444";
    let f1 = codex_rollout_path(&h, "2026-04-01", excluded_session);
    write_jsonl(
        &f1,
        &[
            r#"{"timestamp":"2026-04-01T10:00:00Z","type":"session_meta","payload":{"id":"aaaaaaaa-1111-2222-3333-444444444444","timestamp":"2026-04-01T10:00:00Z","cwd":"C:/Git/currency-edge","originator":"codex_cli","cli_version":"codex-cli 0.141.0"}}"#,
            r#"{"timestamp":"2026-04-01T10:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"x"}}"#,
        ],
    );

    let keep_session = "bbbbbbbb-1111-2222-3333-444444444444";
    let f2 = codex_rollout_path(&h, "2026-04-02", keep_session);
    write_jsonl(
        &f2,
        &[
            r#"{"timestamp":"2026-04-02T10:00:00Z","type":"session_meta","payload":{"id":"bbbbbbbb-1111-2222-3333-444444444444","timestamp":"2026-04-02T10:00:00Z","cwd":"C:/Git/keep","originator":"codex_cli","cli_version":"codex-cli 0.141.0"}}"#,
            r#"{"timestamp":"2026-04-02T10:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"y"}}"#,
        ],
    );

    config::update_settings(codex_session_manager_lib::types::Settings {
        excluded_scan_paths: Some(vec!["currency-edge".into()]),
        ..Default::default()
    })
    .unwrap();

    let sessions = scanner::scan_local_sessions().unwrap();
    let ids: Vec<&str> = sessions.iter().map(|s| s.session_id.as_str()).collect();
    assert!(
        !ids.contains(&excluded_session),
        "excluded session should not appear, got {:?}",
        ids
    );
    assert!(
        ids.contains(&keep_session),
        "keep session should appear, got {:?}",
        ids
    );
}

#[test]
fn scanner_returns_empty_when_no_projects_dir() {
    let _h = setup_temp_home();
    let sessions = scanner::scan_local_sessions().unwrap();
    assert!(sessions.is_empty());
}

#[test]
fn scanner_parses_jsonl_meta_correctly() {
    let h = setup_temp_home();
    let session_id = "11111111-2222-3333-4444-555555555555";
    let file = codex_rollout_path(&h, "2026-04-01", session_id);

    let lines = [
        r#"{"timestamp":"2026-04-01T10:00:00Z","type":"session_meta","payload":{"id":"11111111-2222-3333-4444-555555555555","timestamp":"2026-04-01T10:00:00Z","cwd":"C:/Git/demo","originator":"codex_cli","cli_version":"codex-cli 0.141.0","model_provider":"openai"}}"#,
        r#"{"timestamp":"2026-04-01T10:00:05Z","type":"turn_context","payload":{"turn_id":"t1","cwd":"C:/Git/demo-renamed","model":"gpt-5-codex"}}"#,
        r#"{"timestamp":"2026-04-01T10:00:06Z","type":"event_msg","payload":{"type":"user_message","message":"hello world"}}"#,
        r#"{"timestamp":"2026-04-01T10:01:00Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}}"#,
    ];
    write_jsonl(&file, &lines);

    let sessions = scanner::scan_local_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    let s = &sessions[0];
    assert_eq!(s.session_id, session_id);
    assert_eq!(s.project_dir, "C:/Git/demo-renamed");
    assert_eq!(s.project, "demo-renamed");
    assert_eq!(s.total_lines, 4);
    assert_eq!(s.first_timestamp.as_deref(), Some("2026-04-01T10:00:00Z"));
    assert_eq!(s.last_timestamp.as_deref(), Some("2026-04-01T10:01:00Z"));
    assert_eq!(s.cwd.as_deref(), Some("C:/Git/demo-renamed"));
    assert_eq!(s.version.as_deref(), Some("codex-cli 0.141.0"));
    assert_eq!(s.first_user_message.as_deref(), Some("hello world"));
    assert_eq!(s.storage_type, "local");
}

#[test]
fn scanner_reads_cwd_from_latest_turn_context() {
    let h = setup_temp_home();
    let session_id = "22222222-3333-4444-5555-666666666666";
    let file = codex_rollout_path(&h, "2026-04-01", session_id);

    let lines = [
        r#"{"timestamp":"2026-04-01T10:00:00Z","type":"session_meta","payload":{"id":"22222222-3333-4444-5555-666666666666","timestamp":"2026-04-01T10:00:00Z","cwd":"C:/Git/original","originator":"codex_cli","cli_version":"codex-cli 0.141.0"}}"#,
        r#"{"timestamp":"2026-04-01T10:00:05Z","type":"turn_context","payload":{"turn_id":"t1","cwd":"C:/Git/middle","model":"gpt-5-codex"}}"#,
        r#"{"timestamp":"2026-04-01T10:01:00Z","type":"turn_context","payload":{"turn_id":"t2","cwd":"C:/Git/latest","model":"gpt-5-codex"}}"#,
    ];
    write_jsonl(&file, &lines);

    let sessions = scanner::scan_local_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    let s = &sessions[0];
    assert_eq!(s.cwd.as_deref(), Some("C:/Git/latest"));
    assert_eq!(s.project_dir, "C:/Git/latest");
}

#[test]
fn scanner_marks_archived_sessions() {
    let h = setup_temp_home();
    let active_id = "24242424-2424-2424-2424-242424242424";
    let archived_id = "25252525-2525-2525-2525-252525252525";
    write_jsonl(
        &codex_rollout_path(&h, "2026-04-01", active_id),
        &[r#"{"timestamp":"2026-04-01T10:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"active"}}"#],
    );
    write_jsonl(
        &archived_codex_rollout_path(&h, "2026-04-02", archived_id),
        &[r#"{"timestamp":"2026-04-02T10:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"archived"}}"#],
    );

    let sessions = scanner::scan_local_sessions().unwrap();
    let active = sessions.iter().find(|s| s.session_id == active_id).unwrap();
    let archived = sessions.iter().find(|s| s.session_id == archived_id).unwrap();
    assert!(!active.archived);
    assert!(archived.archived);
}

#[test]
fn scanner_extracts_text_from_array_content() {
    let h = setup_temp_home();
    let file = codex_rollout_path(&h, "2026-04-01", "33333333-3333-3333-3333-333333333333");
    let lines = [
        r#"{"timestamp":"2026-04-01T10:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"array text msg"}}"#,
    ];
    write_jsonl(&file, &lines);

    let msgs = scanner::get_session_messages(file.to_str().unwrap(), 5).unwrap();
    assert_eq!(msgs, vec!["array text msg".to_string()]);
}

#[test]
fn scanner_sorts_by_last_timestamp_desc() {
    let h = setup_temp_home();

    write_jsonl(
        &codex_rollout_path(&h, "2026-01-01", "44444444-4444-4444-4444-444444444444"),
        &[r#"{"timestamp":"2026-01-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"old"}}"#],
    );
    write_jsonl(
        &codex_rollout_path(&h, "2026-04-01", "55555555-5555-5555-5555-555555555555"),
        &[r#"{"timestamp":"2026-04-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"new"}}"#],
    );

    let sessions = scanner::scan_local_sessions().unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].session_id, "55555555-5555-5555-5555-555555555555");
    assert_eq!(sessions[1].session_id, "44444444-4444-4444-4444-444444444444");
}

#[test]
fn scanner_merges_saved_metadata() {
    let h = setup_temp_home();
    let session_id = "66666666-6666-6666-6666-666666666666";
    let file = codex_rollout_path(&h, "2026-04-01", session_id);
    write_jsonl(
        &file,
        &[r#"{"timestamp":"2026-04-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"hi"}}"#],
    );

    config::upsert_session_meta(
        session_id,
        SessionMeta {
            name: Some("nice-name".into()),
            description: Some("nice-desc".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let sessions = scanner::scan_local_sessions().unwrap();
    let s = sessions.iter().find(|s| s.session_id == session_id).unwrap();
    assert_eq!(s.name.as_deref(), Some("nice-name"));
    assert_eq!(s.description.as_deref(), Some("nice-desc"));
}

#[test]
fn scanner_skips_malformed_jsonl_lines() {
    let h = setup_temp_home();
    let session_id = "77777777-7777-7777-7777-777777777777";
    let file = codex_rollout_path(&h, "2026-04-01", session_id);
    let lines = [
        "not json at all",
        r#"{"timestamp":"2026-04-01T10:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"good"}}"#,
        "{broken",
    ];
    write_jsonl(&file, &lines);

    let sessions = scanner::scan_local_sessions().unwrap();
    let s = sessions.iter().find(|s| s.session_id == session_id).unwrap();
    assert_eq!(s.total_lines, 3);
    assert_eq!(s.first_user_message.as_deref(), Some("good"));
}

#[test]
fn scanner_delete_removes_jsonl_file() {
    let h = setup_temp_home();
    let file = codex_rollout_path(&h, "2026-04-01", "88888888-8888-8888-8888-888888888888");
    write_jsonl(
        &file,
        &[r#"{"timestamp":"2026-04-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"x"}}"#],
    );
    assert!(file.exists());

    scanner::delete_session_file(file.to_str().unwrap()).unwrap();
    assert!(!file.exists());
}

#[test]
fn scanner_delete_session_uses_codex_cli_before_file_fallback() {
    let h = setup_temp_home();
    let cli_dir = tempfile::tempdir().unwrap();
    let log = cli_dir.path().join("codex.log");
    let session_id = "12121212-1212-1212-1212-121212121212";
    let file = codex_rollout_path(&h, "2026-04-01", session_id);
    write_jsonl(
        &file,
        &[r#"{"timestamp":"2026-04-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"x"}}"#],
    );
    let fake = write_fake_codex(cli_dir.path(), &log, Some(&file));
    std::env::set_var("CODEX_CLI", &fake);

    scanner::delete_session(session_id, file.to_str().unwrap()).unwrap();

    assert!(!file.exists());
    assert_eq!(fs::read_to_string(log).unwrap().trim(), format!("delete {session_id}"));
}

#[test]
fn scanner_archive_actions_use_codex_cli() {
    let _h = setup_temp_home();
    let cli_dir = tempfile::tempdir().unwrap();
    let log = cli_dir.path().join("codex.log");
    let session_id = "23232323-2323-2323-2323-232323232323";
    let fake = write_fake_codex(cli_dir.path(), &log, None);
    std::env::set_var("CODEX_CLI", &fake);

    scanner::archive_session(session_id).unwrap();
    scanner::unarchive_session(session_id).unwrap();

    let body = fs::read_to_string(log).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines, vec![
        format!("archive {session_id}"),
        format!("unarchive {session_id}"),
    ]);
}

#[test]
fn cloud_checkout_restores_codex_rollout_date_path() {
    let h = setup_temp_home();
    let cloud_root = tempfile::tempdir().unwrap();
    let session_id = "99999999-9999-9999-9999-999999999999";
    let file = codex_rollout_path(&h, "2026-04-03", session_id);
    write_jsonl(
        &file,
        &[
            r#"{"timestamp":"2026-04-03T10:00:00Z","type":"session_meta","payload":{"id":"99999999-9999-9999-9999-999999999999","timestamp":"2026-04-03T10:00:00Z","cwd":"C:/Git/cloud-demo","originator":"codex_cli","cli_version":"codex-cli 0.141.0"}}"#,
            r#"{"timestamp":"2026-04-03T10:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"sync me"}}"#,
        ],
    );

    let cloud_folder = cloud::set_cloud_root(cloud_root.path().to_str().unwrap()).unwrap();
    assert!(cloud_folder.ends_with("Codex Sessions"));

    let session = scanner::scan_local_sessions()
        .unwrap()
        .into_iter()
        .find(|s| s.session_id == session_id)
        .unwrap();
    cloud::upload_session(&session).unwrap();
    scanner::delete_session_file(file.to_str().unwrap()).unwrap();

    let cloud_session = cloud::list_cloud_sessions()
        .unwrap()
        .into_iter()
        .find(|s| s.session_id == session_id)
        .unwrap();
    let checked_out = cloud::checkout(&cloud_session).unwrap();
    let expected = scanner::sessions_dir()
        .join("2026")
        .join("04")
        .join("03")
        .join(file.file_name().unwrap());
    assert_eq!(std::path::PathBuf::from(&checked_out), expected);
    assert!(expected.exists());
}

#[test]
fn cloud_only_sessions_are_reported_with_cloud_only_storage_type() {
    let h = setup_temp_home();
    let cloud_root = tempfile::tempdir().unwrap();
    let session_id = "31313131-3131-3131-3131-313131313131";
    let file = codex_rollout_path(&h, "2026-04-04", session_id);
    write_jsonl(
        &file,
        &[
            r#"{"timestamp":"2026-04-04T10:00:00Z","type":"session_meta","payload":{"id":"31313131-3131-3131-3131-313131313131","timestamp":"2026-04-04T10:00:00Z","cwd":"C:/Git/cloud-only","originator":"codex_cli","cli_version":"codex-cli 0.141.0"}}"#,
            r#"{"timestamp":"2026-04-04T10:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"cloud only"}}"#,
        ],
    );

    cloud::set_cloud_root(cloud_root.path().to_str().unwrap()).unwrap();
    let session = scanner::scan_local_sessions()
        .unwrap()
        .into_iter()
        .find(|s| s.session_id == session_id)
        .unwrap();
    cloud::upload_session(&session).unwrap();
    scanner::delete_session_file(file.to_str().unwrap()).unwrap();

    let cloud_session = cloud::list_cloud_sessions()
        .unwrap()
        .into_iter()
        .find(|s| s.session_id == session_id)
        .unwrap();
    assert_eq!(cloud_session.storage_type, "cloud-only");
}

#[test]
fn resume_plan_windows_with_git_bash_or_cmd() {
    let plan = resume::build_resume_plan("sess-id", Some("C:/some/path"), "windows");
    assert!(plan.args.iter().any(|a| a.contains("codex resume sess-id")));
}

#[test]
fn resume_plan_macos_uses_osascript() {
    let plan = resume::build_resume_plan("sid", None, "macos");
    assert_eq!(plan.program, "osascript");
    assert!(plan.args[0] == "-e");
    assert!(plan.args[1].contains("codex resume sid"));
}

#[test]
fn resume_plan_linux_includes_bash_command() {
    let plan = resume::build_resume_plan("xyz", None, "linux");
    assert_eq!(plan.args[0], "-e");
    assert_eq!(plan.args[1], "bash");
    assert!(plan.args.last().unwrap().contains("codex resume xyz"));
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
    assert!(plan.args.iter().any(|a| a.contains("codex resume abc-123")));
}

#[test]
fn build_command_powershell_uses_set_location_and_noexit() {
    let term = make_term(TerminalKind::PowerShell, "powershell.exe");
    let plan = terminal::build_resume_command(&term, "sid", None, None);
    assert_eq!(plan.args[0], "-NoExit");
    assert_eq!(plan.args[1], "-Command");
    assert!(plan.args[2].contains("codex resume sid"));
}

#[test]
fn build_command_cmd_uses_slash_k() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().to_string_lossy().to_string();
    let term = make_term(TerminalKind::Cmd, "cmd");
    let plan = terminal::build_resume_command(&term, "sid", Some(&cwd), None);
    assert_eq!(plan.args[0], "/k");
    assert!(plan.args[1].contains("cd /d"));
    assert!(plan.args[1].contains("codex resume sid"));
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
    config::update_settings(codex_session_manager_lib::types::Settings {
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
    // codex_cli_found must agree with codex_cli_path being Some/None
    assert_eq!(report.codex_cli_found, report.codex_cli_path.is_some());
}

#[test]
fn summary_exec_invocation_reads_prompt_from_stdin() {
    let invocation = summary::build_codex_exec_invocation("C:/Users/me/AppData/Roaming/npm/codex.cmd");
    assert_eq!(invocation.program, "C:/Users/me/AppData/Roaming/npm/codex.cmd");
    assert_eq!(invocation.args, vec!["exec", "--model", "gpt-5-codex", "-"]);
    assert!(invocation.prompt_on_stdin);
}

#[cfg(target_os = "windows")]
#[test]
fn environment_prefers_cmd_shim_over_extensionless_npm_shim() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let old_path = std::env::var_os("PATH");
    let old_cli = std::env::var_os("CODEX_CLI");
    fs::write(dir.path().join("codex"), "extensionless shim").unwrap();
    fs::write(dir.path().join("codex.cmd"), "@echo off\r\necho codex-cli 9.9.9\r\n").unwrap();

    std::env::remove_var("CODEX_CLI");
    std::env::set_var("PATH", dir.path());
    let found = environment::locate_codex().expect("codex should be found");
    assert!(
        found.ends_with("codex.cmd"),
        "Windows should prefer executable cmd shim, got {found}"
    );

    if let Some(path) = old_path {
        std::env::set_var("PATH", path);
    }
    if let Some(cli) = old_cli {
        std::env::set_var("CODEX_CLI", cli);
    }
}
