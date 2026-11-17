// 등록 ID와 API 메타데이터를 대조하여 전체 세션과 완전성을 수집한다.
use super::codex_app_server::RegisteredThread;
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Registration {
    pub(crate) archived: bool,
    pub(crate) parent_id: Option<String>,
}
pub(crate) type Registry = BTreeMap<String, Registration>;
#[derive(Debug, Default)]
pub(crate) struct ThreadSnapshot {
    pub(crate) threads: Vec<RegisteredThread>,
    pub(crate) missing_count: usize,
    pub(crate) errors: Vec<String>,
}
impl ThreadSnapshot {
    pub(crate) fn require_complete(self) -> Result<Vec<RegisteredThread>> {
        if self.missing_count > 0 || !self.errors.is_empty() {
            anyhow::bail!(
                "전체 세션 조회 불완전 (누락 {}): {}",
                self.missing_count,
                self.errors.join("; ")
            );
        }
        Ok(self.threads)
    }
}
pub(crate) fn collect(
    mut registry: impl FnMut() -> Result<Registry>,
    mut request: impl FnMut(&str, Value) -> Result<Value>,
    deadline: Instant,
) -> ThreadSnapshot {
    use std::collections::{HashMap, HashSet};
    for attempt in 0..2 {
        let mut snapshot = ThreadSnapshot::default();
        let before = match registry() {
            Ok(rows) => Some(rows),
            Err(error) => {
                snapshot
                    .errors
                    .push(format!("등록 목록 확인 실패: {error:#}"));
                None
            }
        };
        let mut found = HashMap::new();
        for archived in [false, true] {
            if let Err(error) = list_pages(&mut request, None, archived, deadline, &mut found) {
                snapshot
                    .errors
                    .push(format!("기본 목록 조회 실패: {error:#}"));
            }
        }
        if let Some(rows) = &before {
            let mut ancestors = Vec::new();
            let mut unique = HashSet::new();
            for (id, row) in rows
                .iter()
                .filter(|(id, _)| !found.contains_key(id.as_str()))
            {
                let mut ancestor = row.parent_id.clone();
                let mut seen = HashSet::from([id.clone()]);
                while let Some(parent) = ancestor.as_ref() {
                    if !seen.insert(parent.clone()) {
                        ancestor = None;
                        break;
                    }
                    match rows.get(parent).and_then(|r| r.parent_id.clone()) {
                        Some(next) => ancestor = Some(next),
                        None => break,
                    }
                }
                if let Some(parent) = ancestor {
                    if unique.insert(parent.clone()) {
                        ancestors.push((id.clone(), parent));
                    }
                }
            }
            for (missing, ancestor) in ancestors {
                // 상위 조상 조회가 이 가지를 이미 회수했으면 같은 하위 가지를 다시 요청하지 않는다.
                if found.contains_key(&missing) {
                    continue;
                }
                for archived in [false, true] {
                    if let Err(error) = list_pages(
                        &mut request,
                        Some(&ancestor),
                        archived,
                        deadline,
                        &mut found,
                    ) {
                        snapshot
                            .errors
                            .push(format!("자손 조회 실패 ({ancestor}): {error:#}"));
                    }
                }
            }
            for (id, row) in rows
                .iter()
                .filter(|(id, _)| !found.contains_key(id.as_str()))
                .map(|(id, r)| (id.clone(), r.clone()))
                .collect::<Vec<_>>()
            {
                let read = (|| -> Result<RegisteredThread> {
                    check_deadline(deadline)?;
                    let value = request(
                        "thread/read",
                        serde_json::json!({"threadId":id,"includeTurns":false}),
                    )?;
                    let mut thread: RegisteredThread = serde_json::from_value(
                        value
                            .get("thread")
                            .cloned()
                            .ok_or_else(|| anyhow::anyhow!("thread/read missing thread"))?,
                    )?;
                    if thread.id != id || thread.ephemeral {
                        anyhow::bail!("thread/read ID 또는 지속 등록 불일치");
                    }
                    thread.archived = row.archived;
                    Ok(thread)
                })();
                match read {
                    Ok(thread) => {
                        found.insert(id, thread);
                    }
                    Err(error) => snapshot
                        .errors
                        .push(format!("메타데이터 조회 실패 ({id}): {error:#}")),
                }
            }
        }
        let after = match registry() {
            Ok(rows) => Some(rows),
            Err(error) => {
                snapshot
                    .errors
                    .push(format!("등록 목록 재확인 실패: {error:#}"));
                None
            }
        };
        if before.is_some() && after.is_some() && before != after {
            if attempt == 0 && Instant::now() < deadline {
                continue;
            }
            snapshot
                .errors
                .push("조회 중 등록 목록이 다시 변경됐습니다. 새로고침해 주세요.".into());
        }
        if let Some(rows) = after.as_ref().or(before.as_ref()) {
            snapshot.missing_count = rows
                .keys()
                .filter(|id| !found.contains_key(id.as_str()))
                .count();
        }
        if let Err(error) = check_deadline(deadline) {
            snapshot.errors.push(error.to_string());
        }
        snapshot.threads = found.into_values().collect();
        snapshot.threads.sort_by(|a, b| a.id.cmp(&b.id));
        return snapshot;
    }
    unreachable!()
}
fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        anyhow::bail!("전체 세션 조회 60초 예산 초과");
    }
    Ok(())
}
fn list_pages(
    request: &mut impl FnMut(&str, Value) -> Result<Value>,
    ancestor: Option<&str>,
    archived: bool,
    deadline: Instant,
    found: &mut std::collections::HashMap<String, RegisteredThread>,
) -> Result<()> {
    use serde_json::json;
    let mut cursor = None::<String>;
    let mut cursors = std::collections::HashSet::new();
    loop {
        check_deadline(deadline)?;
        let mut params = json!({"archived":archived,"cursor":cursor,"limit":100,"sortKey":"created_at","sortDirection":"desc","modelProviders":[],"useStateDbOnly":true,"sourceKinds":["cli","vscode","exec","appServer","subAgent","subAgentReview","subAgentCompact","subAgentThreadSpawn","subAgentOther","unknown"]});
        if let Some(ancestor) = ancestor {
            params["ancestorThreadId"] = ancestor.into();
        }
        let page = request("thread/list", params)?;
        let data = page
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("malformed thread/list data"))?;
        for value in data {
            let mut thread: RegisteredThread = serde_json::from_value(value.clone())?;
            if !thread.ephemeral {
                thread.archived = archived;
                found.entry(thread.id.clone()).or_insert(thread);
            }
        }
        match page.get("nextCursor") {
            Some(Value::Null) => break,
            Some(Value::String(next)) if cursors.insert(next.clone()) => {
                cursor = Some(next.clone())
            }
            _ => anyhow::bail!("malformed or repeated thread/list cursor"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::time::Duration;
    fn row(id: &str, parent: Option<&str>) -> Value {
        json!({"id":id,"parentThreadId":parent,"cwd":"project","path":null,"cliVersion":"0.169.0","createdAt":1,"updatedAt":2,"source":"cli","ephemeral":false})
    }
    fn registry() -> Registry {
        [
            ("p", None, false),
            ("c", Some("p"), true),
            ("g", Some("c"), false),
            ("o", Some("absent"), true),
        ]
        .into_iter()
        .map(|(id, p, a)| {
            (
                id.into(),
                Registration {
                    archived: a,
                    parent_id: p.map(str::to_owned),
                },
            )
        })
        .collect()
    }
    #[test]
    fn catalog_recovers_deep_archived_children_orphans_and_deduplicates() {
        let mut pages = VecDeque::from([
            json!({"data":[row("p",None)],"nextCursor":null}),
            json!({"data":[],"nextCursor":null}),
            json!({"data":[row("g",Some("c"))],"nextCursor":"more"}),
            json!({"data":[row("p",None)],"nextCursor":null}),
            json!({"data":[row("c",Some("p"))],"nextCursor":null}),
            json!({"data":[],"nextCursor":null}),
            json!({"data":[],"nextCursor":null}),
            json!({"thread":row("o",Some("absent"))}),
        ]);
        let snapshot = collect(
            || Ok(registry()),
            |method, params| {
                if method == "thread/list" {
                    assert_eq!(params["useStateDbOnly"], true);
                    assert_eq!(params["modelProviders"], json!([]));
                    if pages.len() <= 6 {
                        assert!(
                            params["ancestorThreadId"] == "p"
                                || params["ancestorThreadId"] == "absent"
                        );
                    }
                } else {
                    assert_eq!(method, "thread/read");
                    assert_eq!(params["includeTurns"], false);
                }
                Ok(pages.pop_front().expect("unexpected extra RPC"))
            },
            Instant::now() + Duration::from_secs(60),
        );
        assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);
        assert_eq!(snapshot.missing_count, 0);
        let threads = snapshot.require_complete().unwrap();
        assert_eq!(threads.len(), 4);
        assert_eq!(
            threads
                .iter()
                .find(|t| t.id == "g")
                .unwrap()
                .parent_thread_id
                .as_deref(),
            Some("c")
        );
        assert!(threads.iter().find(|t| t.id == "c").unwrap().archived);
        assert!(threads.iter().find(|t| t.id == "o").unwrap().archived);
        assert!(pages.is_empty());
    }
    #[test]
    fn catalog_retains_partial_rows_but_rpc_errors_missing_ids_and_deadline_block_deletion() {
        let mut n = 0;
        let snapshot = collect(
            || Ok(registry()),
            |_, _| {
                n += 1;
                if n == 1 {
                    Ok(json!({"data":[row("p",None)],"nextCursor":"more"}))
                } else {
                    anyhow::bail!("unavailable")
                }
            },
            Instant::now() + Duration::from_secs(60),
        );
        assert_eq!(snapshot.threads.len(), 1);
        assert_eq!(snapshot.missing_count, 3);
        assert!(!snapshot.errors.is_empty());
        assert!(snapshot.require_complete().is_err());
        let snapshot = collect(
            || anyhow::bail!("locked or unsupported schema"),
            |_, _| Ok(json!({"data":[],"nextCursor":null})),
            Instant::now() + Duration::from_secs(60),
        );
        assert!(snapshot.require_complete().is_err());
        let snapshot = collect(
            || Ok(registry()),
            |_, _| panic!("expired refresh must not send RPC"),
            Instant::now() - Duration::from_secs(1),
        );
        assert_eq!(snapshot.missing_count, 4);
        assert!(snapshot.require_complete().is_err());
    }
    #[test]
    fn catalog_retries_registry_change_once_and_refuses_repeated_changes() {
        for unstable in [false, true] {
            let mut reads = 0;
            let snapshot = collect(
                || {
                    reads += 1;
                    let mut r = Registry::new();
                    if reads >= 2 && (!unstable || reads % 2 == 0) {
                        r.insert(
                            "p".into(),
                            Registration {
                                archived: false,
                                parent_id: None,
                            },
                        );
                    }
                    Ok(r)
                },
                |method, _| {
                    Ok(if method == "thread/read" {
                        json!({"thread":row("p",None)})
                    } else {
                        json!({"data":[row("p",None)],"nextCursor":null})
                    })
                },
                Instant::now() + Duration::from_secs(60),
            );
            assert_eq!(reads, 4);
            assert_eq!(snapshot.errors.is_empty(), !unstable);
            assert_eq!(snapshot.require_complete().is_ok(), !unstable);
        }
    }
    #[test]
    fn catalog_old_cli_complete_base_list_skips_all_descendant_queries() {
        let snapshot = collect(
            || Ok(registry()),
            |method, params| {
                assert_eq!(method, "thread/list");
                assert!(params.get("ancestorThreadId").is_none());
                Ok(
                    json!({"data":[row("p",None),row("c",Some("p")),row("g",Some("c")),row("o",Some("absent"))],"nextCursor":null}),
                )
            },
            Instant::now() + Duration::from_secs(60),
        );
        assert_eq!(snapshot.require_complete().unwrap().len(), 4);
    }
}
