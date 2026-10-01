// 실제 세션 대신 임시 파일과 가짜 CLI로 삭제 실패 처리를 검증한다.
use super::*;

fn failing_cli(home: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    let (name, script) = ("codex.cmd", "@echo off\r\nif \"%1\"==\"--version\" (echo fake-codex 1.0 & exit /b 0)\r\necho %CODEX_HOME%>\"%~dp0observed-home.txt\"\r\necho failed to delete session 1>&2\r\nexit /b 1\r\n");
    #[cfg(not(target_os = "windows"))]
    let (name, script) = ("codex", "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'fake-codex 1.0'; exit 0; fi\nprintf '%s' \"$CODEX_HOME\" > \"$(dirname \"$0\")/observed-home.txt\"\necho 'failed to delete session' >&2\nexit 1\n");
    let path = home.join(name);
    fs::write(&path, script).unwrap();
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

#[test]
fn failed_cli_classifies_missing_only_after_registry_and_file_checks() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let fake = failing_cli(home);
    let file = home.join("session.jsonl");
    fs::write(&file, "session content").unwrap();
    assert!(delete_session_using(fake.to_str(), home, &[], "session", &file).is_err());
    assert_eq!(fs::read_to_string(&file).unwrap(), "session content");
    assert_eq!(
        fs::read_to_string(home.join("observed-home.txt"))
            .unwrap()
            .trim(),
        home.to_str().unwrap()
    );
    fs::remove_file(&file).unwrap();
    assert!(matches!(
        delete_session_using(fake.to_str(), home, &[], "session", &file).unwrap(),
        DeleteSessionStatus::AlreadyMissing
    ));
    let db = rusqlite::Connection::open(home.join("state_5.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE threads (id TEXT); INSERT INTO threads VALUES ('session');")
        .unwrap();
    assert!(delete_session_using(fake.to_str(), home, &[], "session", &file).is_err());
}

#[test]
fn unavailable_cli_never_removes_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("session.jsonl");
    fs::write(&file, "session content").unwrap();
    assert!(delete_session_using(None, temp.path(), &[], "session", &file).is_err());
    assert!(file.exists());
}

#[test]
fn invalid_session_ids_are_not_cli_options_or_missing_sessions() {
    let temp = tempfile::tempdir().unwrap();
    let fake = failing_cli(temp.path());
    for id in ["", "--help", "bad id", "bad&command"] {
        assert!(
            delete_session_using(
                fake.to_str(),
                temp.path(),
                &[],
                id,
                &temp.path().join("gone.jsonl")
            )
            .is_err(),
            "invalid id must fail: {id}"
        );
    }
}
