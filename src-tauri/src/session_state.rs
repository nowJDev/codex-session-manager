// 세션 파일과 Codex 등록 정보를 읽기 전용으로 확인한다.
use anyhow::{anyhow, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub(crate) fn is_session_absent(
    home: &Path,
    roots: &[PathBuf],
    session_id: &str,
    file_path: &Path,
) -> Result<bool> {
    if file_path.try_exists()? {
        return Ok(false);
    }
    let index = home.join("session_index.jsonl");
    if index.try_exists()? {
        for line in BufReader::new(fs::File::open(&index)?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value =
                serde_json::from_str(&line).context("Codex session_index를 읽을 수 없음")?;
            let id = value
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("알 수 없는 Codex session_index 형식"))?;
            if id == session_id {
                return Ok(false);
            }
        }
    }

    let mut databases = Vec::new();
    if home.try_exists()? {
        for entry in fs::read_dir(home)? {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if let Some(version) = name
                .strip_prefix("state_")
                .and_then(|name| name.strip_suffix(".sqlite"))
            {
                databases.push((
                    version
                        .parse::<u64>()
                        .context("알 수 없는 Codex 상태 DB 이름")?,
                    path,
                ));
            }
        }
    }
    if let Some((_, database)) = databases.into_iter().max_by_key(|(version, _)| *version) {
        let connection =
            Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .with_context(|| format!("Codex 상태 DB 열기 실패: {}", database.display()))?;
        let registered: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM threads WHERE id = ?1)",
                [session_id],
                |row| row.get(0),
            )
            .context("Codex 상태 DB에서 세션 등록 여부 확인 실패")?;
        if registered {
            return Ok(false);
        }
    }

    for root in roots {
        if !root.try_exists()? {
            continue;
        }
        // 읽기 실패와 링크 순환은 부재로 간주하지 않고 호출자에게 전달한다.
        for entry in walkdir::WalkDir::new(root).follow_links(true) {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().and_then(|s| s.to_str()) != Some("jsonl")
            {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem == session_id || stem.ends_with(&format!("-{session_id}")) {
                return Ok(false);
            }
            for line in BufReader::new(fs::File::open(path)?).lines().take(200) {
                let line = line?;
                let Ok(value) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if value.get("type").and_then(Value::as_str) == Some("session_meta")
                    && value.pointer("/payload/id").and_then(Value::as_str) == Some(session_id)
                {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_files_and_registry_allow_stale_metadata_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        assert!(
            is_session_absent(temp.path(), &[], "missing", &temp.path().join("gone.jsonl"))
                .unwrap()
        );
    }

    #[test]
    fn registry_records_and_unreadable_registry_prevent_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let missing = home.join("gone.jsonl");
        let db = Connection::open(home.join("state_5.sqlite")).unwrap();
        db.execute_batch("CREATE TABLE threads (id TEXT PRIMARY KEY); INSERT INTO threads VALUES ('registered');").unwrap();
        assert!(!is_session_absent(home, &[], "registered", &missing).unwrap());
        assert!(is_session_absent(home, &[], "missing", &missing).unwrap());

        fs::write(home.join("session_index.jsonl"), "{\"id\":\"indexed\"}\n").unwrap();
        assert!(!is_session_absent(home, &[], "indexed", &missing).unwrap());
        fs::write(home.join("session_index.jsonl"), "{broken").unwrap();
        assert!(is_session_absent(home, &[], "missing", &missing).is_err());
        fs::remove_file(home.join("session_index.jsonl")).unwrap();

        // 최신 DB를 읽을 수 없으면 이전 DB의 빈 결과로 대체하지 않는다.
        fs::write(home.join("state_10.sqlite"), "corrupt database").unwrap();
        assert!(is_session_absent(home, &[], "missing", &missing).is_err());
    }

    #[test]
    fn moved_and_duplicate_rollouts_prevent_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("archived_sessions");
        fs::create_dir_all(&root).unwrap();
        let missing = temp.path().join("original.jsonl");
        let duplicate = root.join("rollout-2026-moved.jsonl");
        fs::write(&duplicate, "").unwrap();
        assert!(!is_session_absent(temp.path(), &[root.clone()], "moved", &missing).unwrap());
        fs::write(
            root.join("renamed.jsonl"),
            r#"{"type":"session_meta","payload":{"id":"renamed"}}"#,
        )
        .unwrap();
        assert!(!is_session_absent(temp.path(), &[root], "renamed", &missing).unwrap());
        assert!(!is_session_absent(temp.path(), &[], "other", &duplicate).unwrap());
    }
}
