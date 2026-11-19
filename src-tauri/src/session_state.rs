// 세션 파일과 Codex 등록 정보를 읽기 전용으로 확인한다.
#[cfg(test)]
use anyhow::anyhow;
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::fs;
#[cfg(test)]
use std::io::{BufRead, BufReader};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
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

#[cfg(test)]
mod inventory_tests {
    use super::*;
    #[test]
    fn inventory_reads_supported_db_without_writes_and_preserves_parent_and_archive() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("state_5.sqlite");
        let db = Connection::open(&file).unwrap();
        db.execute_batch(r#"CREATE TABLE threads(id TEXT PRIMARY KEY, archived INTEGER NOT NULL, source TEXT NOT NULL); INSERT INTO threads VALUES ('p',0,'"cli"'), ('c',1,'{"subagent":{"thread_spawn":{"parent_thread_id":"p"}}}');"#).unwrap();
        drop(db);
        let before = fs::read(&file).unwrap();
        let records = registered_inventory(dir.path()).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records["c"].parent_id.as_deref(), Some("p"));
        assert!(records["c"].archived);
        assert_eq!(fs::read(&file).unwrap(), before);
        fs::write(dir.path().join("state_99.sqlite"), "future").unwrap();
        assert!(
            registered_inventory(dir.path()).is_err(),
            "unknown active state must not fall back to old rows"
        );
    }
    #[test]
    fn age_recheck_reads_legacy_seconds_and_source_descendants() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(dir.path().join("state_5.sqlite")).unwrap();
        db.execute_batch(
            r#"CREATE TABLE threads(id TEXT PRIMARY KEY, source TEXT NOT NULL, updated_at INTEGER NOT NULL);
               INSERT INTO threads VALUES ('p','cli',90);"#,
        )
        .unwrap();
        let approved = ["p"].into_iter().collect();
        assert!(revalidate_deletion_age(dir.path(), &approved, 100).is_ok());
        db.execute_batch(
            r#"INSERT INTO threads VALUES ('c','{"subagent":{"thread_spawn":{"parent_thread_id":"p"}}}',91);"#,
        )
        .unwrap();
        assert!(revalidate_deletion_age(dir.path(), &approved, 100).is_err());
        assert!(
            revalidate_deletion_age(dir.path(), &["p", "c"].into_iter().collect(), 100).is_ok()
        );
    }

    #[test]
    fn age_recheck_uses_latest_milliseconds_and_additional_db_relations() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(dir.path().join("state_5.sqlite")).unwrap();
        db.execute_batch(
            r#"CREATE TABLE threads(id TEXT PRIMARY KEY, source TEXT NOT NULL, updated_at INTEGER NOT NULL,
                                   updated_at_ms INTEGER, attached_thread_parent_id TEXT);
               CREATE TABLE thread_spawn_edges(parent_thread_id TEXT, child_thread_id TEXT PRIMARY KEY);
               INSERT INTO threads VALUES ('p','cli',90,90000,NULL), ('c','cli',99,99999,NULL), ('o','cli',95,NULL,NULL);
               INSERT INTO thread_spawn_edges VALUES ('p','c');"#,
        )
        .unwrap();
        let approved = ["p", "c"].into_iter().collect();
        assert!(revalidate_deletion_age(dir.path(), &approved, 100).is_ok());
        db.execute(
            "UPDATE threads SET updated_at_ms = 100000 WHERE id = 'c'",
            [],
        )
        .unwrap();
        assert!(revalidate_deletion_age(dir.path(), &approved, 100).is_err());
        db.execute(
            "UPDATE threads SET updated_at_ms = NULL, updated_at = 0 WHERE id = 'c'",
            [],
        )
        .unwrap();
        assert!(revalidate_deletion_age(dir.path(), &approved, 100).is_err());
        db.execute("UPDATE threads SET updated_at = 99 WHERE id = 'c'", [])
            .unwrap();
        assert!(revalidate_deletion_age(dir.path(), &approved, 100).is_ok());
        db.execute(
            "UPDATE threads SET attached_thread_parent_id = 'c' WHERE id = 'o'",
            [],
        )
        .unwrap();
        assert!(revalidate_deletion_age(dir.path(), &approved, 100).is_err());
    }

    #[test]
    fn inventory_missing_corrupt_schema_and_locked_db_never_mean_empty_registration() {
        let dir = tempfile::tempdir().unwrap();
        assert!(registered_inventory(dir.path()).is_err());
        let file = dir.path().join("state_5.sqlite");
        let db = Connection::open(&file).unwrap();
        db.execute_batch("CREATE TABLE threads(id TEXT); INSERT INTO threads VALUES ('p');")
            .unwrap();
        assert!(registered_inventory(dir.path()).is_err());
        db.execute_batch("DROP TABLE threads; CREATE TABLE threads(id TEXT, archived INTEGER, source TEXT); BEGIN EXCLUSIVE;").unwrap();
        assert!(registered_inventory(dir.path()).is_err());
        db.execute_batch("ROLLBACK;").unwrap();
        drop(db);
        fs::write(file, "corrupt").unwrap();
        assert!(registered_inventory(dir.path()).is_err());
    }
}
fn open_registered_database(home: &Path) -> Result<Connection> {
    // Codex 0.166/0.169의 활성 상태 형식은 state_5이다. 미래 번호를 이전 DB로 대체하지 않는다.
    for entry in fs::read_dir(home).context("Codex 홈 등록 목록 접근 실패")? {
        let entry = entry?;
        let name = entry.file_name();
        if let Some(version) = name
            .to_str()
            .and_then(|n| n.strip_prefix("state_"))
            .and_then(|n| n.strip_suffix(".sqlite"))
        {
            let version: u64 = version.parse().context("알 수 없는 Codex 상태 DB 이름")?;
            if version > 5 {
                anyhow::bail!("지원하지 않는 Codex 상태 DB 버전: {version}");
            }
        }
    }
    let connection = Connection::open_with_flags(
        home.join("state_5.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .context("Codex state_5 DB 읽기 전용 열기 실패")?;
    connection.busy_timeout(std::time::Duration::ZERO)?;
    Ok(connection)
}

pub(crate) fn registered_inventory(
    home: &Path,
) -> Result<crate::adapters::outbound::full_catalog::Registry> {
    use crate::adapters::outbound::full_catalog::{Registration, Registry};
    let mut connection = open_registered_database(home)?;
    let transaction = connection.transaction()?;
    let mut query = transaction
        .prepare("SELECT id, archived, source FROM threads")
        .context("지원하지 않는 Codex threads 스키마")?;
    let mut records = Registry::new();
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let archived: i64 = row.get(1)?;
        if id.trim().is_empty() || !matches!(archived, 0 | 1) {
            anyhow::bail!("잘못된 Codex 등록 메타데이터");
        }
        let source: String = row.get(2)?;
        let source: Value = serde_json::from_str(&source).unwrap_or(Value::String(source));
        let parent_id = source
            .pointer("/subagent/thread_spawn/parent_thread_id")
            .or_else(|| source.pointer("/subAgent/thread_spawn/parent_thread_id"))
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned);
        records.insert(
            id,
            Registration {
                archived: archived == 1,
                parent_id,
            },
        );
    }
    drop(rows);
    drop(query);
    transaction.commit()?;
    Ok(records)
}

pub(crate) fn revalidate_deletion_age(
    home: &Path,
    approved: &std::collections::HashSet<&str>,
    cutoff: i64,
) -> Result<()> {
    let mut connection = open_registered_database(home)?;
    let transaction = connection.transaction()?;
    let (has_milliseconds, has_attached_parent, has_spawn_edges) = transaction.query_row(
        "SELECT
            EXISTS(SELECT 1 FROM pragma_table_info('threads') WHERE name = 'updated_at_ms'),
            EXISTS(SELECT 1 FROM pragma_table_info('threads') WHERE name = 'attached_thread_parent_id'),
            EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'thread_spawn_edges')",
        [],
        |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?, row.get::<_, bool>(2)?)),
    )?;
    let millis_column = if has_milliseconds {
        "threads.updated_at_ms"
    } else {
        "NULL"
    };
    let attached_relation = if has_attached_parent {
        "UNION ALL SELECT attached_thread_parent_id, id FROM threads"
    } else {
        ""
    };
    let spawn_relation = if has_spawn_edges {
        "UNION ALL SELECT parent_thread_id, child_thread_id FROM thread_spawn_edges"
    } else {
        ""
    };
    // 승인된 범위 전체에서 새 자손을 탐색하고 알려진 활동 시각을 같은 스냅샷에서 검사한다.
    let sql = format!(
        "WITH RECURSIVE relations(parent, child) AS (
            SELECT CASE WHEN json_valid(source) THEN COALESCE(
                json_extract(source, '$.subagent.thread_spawn.parent_thread_id'),
                json_extract(source, '$.subAgent.thread_spawn.parent_thread_id')
            ) END, id FROM threads
            {attached_relation}
            {spawn_relation}
        ), scope(id) AS (
            SELECT value FROM json_each(?1)
            UNION SELECT relations.child FROM relations JOIN scope ON relations.parent = scope.id
        )
        SELECT scope.id, threads.updated_at, {millis_column}
        FROM scope LEFT JOIN threads ON threads.id = scope.id"
    );
    let cutoff_ms = cutoff
        .checked_mul(1000)
        .context("올바르지 않은 활동 기간입니다.")?;
    let mut query = transaction
        .prepare(&sql)
        .context("Codex 삭제 범위 메타데이터 읽기 실패")?;
    let approved_json = serde_json::to_string(approved)?;
    let mut rows = query.query([approved_json])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        if !approved.contains(id.as_str()) {
            anyhow::bail!("삭제 영향 범위에 새 자손이 생겨 삭제 요청을 건너뛰었습니다.");
        }
        let seconds: Option<i64> = row.get(1)?;
        let milliseconds: Option<i64> = row.get(2)?;
        if !seconds.is_some_and(|time| time > 0 && time < cutoff)
            || milliseconds.is_some_and(|time| time <= 0 || time >= cutoff_ms)
        {
            anyhow::bail!(
                "최근 활동이 있거나 활동 시각을 확인할 수 없어 삭제 요청을 건너뛰었습니다."
            );
        }
    }
    Ok(())
}
