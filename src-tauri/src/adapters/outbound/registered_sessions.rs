// Codex 등록 세션을 앱 모델로 변환하고 ID 삭제의 결과를 확인한다.
use super::codex_app_server::{AppServer, RegisteredThread, RpcError};
use super::full_catalog::parent_id;
use crate::application::refresh_budget::RefreshBudget;
use crate::types::{
    Config, DeleteProgress, DeleteSessionResult, DeleteSessionStatus, DeleteSessionTarget,
    DeletionPlan, Session, SessionKind, SessionSnapshot,
};
use anyhow::{bail, Result};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

pub(crate) fn list_sessions(budget: RefreshBudget) -> Result<SessionSnapshot> {
    let mut out = SessionSnapshot {
        sessions: vec![],
        complete: true,
        missing_count: 0,
        errors: vec![],
    };
    if !budget.record_timeout(&mut out) {
        return Ok(out);
    }
    let snapshot = AppServer::connect()?.collect_threads(budget.deadline());
    let config = if budget.check().is_ok() {
        crate::config::load_config()
    } else {
        Config::default()
    };
    let ids: HashSet<_> = snapshot.threads.iter().map(|t| t.id.clone()).collect();
    out = map_collected_threads(snapshot, &config, budget);
    if budget.record_timeout(&mut out) {
        match crate::scanner::scan_extra_sessions(budget) {
            Ok(extra) => {
                out.complete &= extra.complete;
                out.errors.extend(extra.errors);
                for session in extra.sessions {
                    budget.record_timeout(&mut out);
                    if !ids.contains(&session.session_id) {
                        out.sessions.push(session);
                    }
                }
            }
            Err(error) => {
                out.complete = false;
                out.errors.push(format!("추가 파일 조회 실패: {error:#}"));
            }
        }
    }
    budget.record_timeout(&mut out);
    Ok(out)
}

fn map_collected_threads(
    snapshot: super::full_catalog::ThreadSnapshot,
    config: &Config,
    budget: RefreshBudget,
) -> SessionSnapshot {
    let mut out = SessionSnapshot {
        sessions: vec![],
        complete: true,
        missing_count: 0,
        errors: vec![],
    };
    out.errors = snapshot.errors;
    out.missing_count = snapshot.missing_count;
    out.complete = out.errors.is_empty() && out.missing_count == 0;
    budget.record_timeout(&mut out);
    for thread in snapshot.threads {
        if thread.ephemeral {
            continue;
        }
        let path = thread.path.clone();
        let mut session = map_thread(thread, config);
        // API 메타의 순수 변환은 항상 보존하고, 확인이 필요한 파일 I/O만 남은 예산에서 수행한다.
        if budget.record_timeout(&mut out) {
            if let Some(path) = path {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.is_file() {
                        session.file_path = Some(path);
                        session.size = Some(metadata.len());
                    }
                }
            }
        }
        out.sessions.push(session);
        budget.record_timeout(&mut out);
    }
    out
}

fn map_thread(thread: RegisteredThread, config: &Config) -> Session {
    let meta = config.sessions.get(&thread.id).cloned().unwrap_or_default();
    let parent = parent_id(&thread).map(str::to_owned);
    let session_kind = crate::types::SessionKind::from_source(&thread.source);
    let is_subagent = session_kind == crate::types::SessionKind::Worker;
    let is_summary_run = crate::domain::session::is_internal_summary_run(Some(&thread.cwd));
    let project = Path::new(&thread.cwd)
        .file_name()
        .and_then(|p| p.to_str())
        .unwrap_or("Codex")
        .to_owned();
    let timestamp = |seconds| chrono::DateTime::from_timestamp(seconds, 0).map(|d| d.to_rfc3339());
    let preview = (!thread.preview.trim().is_empty()).then_some(thread.preview);
    let name = meta
        .name
        .filter(|s| !s.trim().is_empty())
        .or(thread.name.filter(|s| !s.trim().is_empty()))
        .or(thread
            .agent_nickname
            .clone()
            .filter(|s| !s.trim().is_empty()))
        .or_else(|| {
            preview
                .as_deref()
                .and_then(crate::domain::transcript::first_user_message_name)
        })
        .or_else(|| Some(session_kind.fallback_name(&thread.id)));
    Session {
        session_id: thread.id,
        parent_id: parent,
        is_subagent,
        agent_nickname: thread.agent_nickname,
        name,
        description: meta.description,
        auto_summary: meta.auto_summary,
        project,
        project_dir: thread.cwd.clone(),
        cwd: Some(thread.cwd),
        file_path: None,
        size: None,
        preview,
        history_mode: thread.history_mode,
        registered: true,
        session_kind,
        is_summary_run,
        total_lines: 0,
        first_timestamp: timestamp(thread.created_at),
        last_timestamp: timestamp(thread.updated_at),
        version: Some(thread.cli_version),
        first_user_message: None,
        storage_type: "local-only".into(),
        archived: thread.archived,
        favorite: meta.favorite.unwrap_or(false),
        locked_by: None,
    }
}

pub(crate) fn prepare_deletion(targets: &[DeleteSessionTarget]) -> Result<DeletionPlan> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let threads = AppServer::connect()?
        .collect_threads(deadline)
        .require_complete()?;
    deletion_plan(&threads, targets)
}

fn validate_id(id: &str) -> Result<()> {
    if id.len() != 36
        || !id.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
    {
        bail!("올바르지 않은 세션 ID입니다.");
    }
    Ok(())
}

fn guardian_blocked_ids(threads: &[RegisteredThread]) -> HashSet<&str> {
    let parents: HashMap<_, _> = threads
        .iter()
        .filter_map(|thread| parent_id(thread).map(|parent| (thread.id.as_str(), parent)))
        .collect();
    let mut blocked = HashSet::new();
    for thread in threads {
        if SessionKind::from_source(&thread.source) == SessionKind::Guardian {
            continue;
        }
        let mut current = Some(thread.id.as_str());
        while let Some(id) = current {
            if !blocked.insert(id) {
                break;
            }
            current = parents.get(id).copied();
        }
    }
    blocked
}

fn deletion_plan(
    threads: &[RegisteredThread],
    targets: &[DeleteSessionTarget],
) -> Result<DeletionPlan> {
    let mut selected = HashSet::new();
    let mut unique = Vec::new();
    for target in targets {
        validate_id(&target.session_id)?;
        if selected.insert(target.session_id.clone()) {
            unique.push(target.clone());
        }
    }
    let mut skipped_guardian_count = 0;
    if unique.iter().any(|target| target.guardian_only) {
        let blocked = guardian_blocked_ids(threads);
        let original_count = unique.len();
        unique.retain(|target| {
            !target.guardian_only || !blocked.contains(target.session_id.as_str())
        });
        skipped_guardian_count = original_count - unique.len();
    }
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for thread in threads {
        if let Some(parent) = parent_id(thread) {
            children.entry(parent).or_default().push(&thread.id);
        }
    }
    let mut stack: Vec<_> = unique
        .iter()
        .map(|target| target.session_id.as_str())
        .collect();
    let mut affected = HashSet::new();
    while let Some(id) = stack.pop() {
        if !affected.insert(id.to_owned()) {
            continue;
        }
        if let Some(descendants) = children.get(id) {
            stack.extend(descendants);
        }
    }
    let mut affected_ids: Vec<_> = affected.into_iter().collect();
    affected_ids.sort();
    Ok(DeletionPlan {
        targets: unique,
        affected_ids,
        skipped_guardian_count,
    })
}

trait RegistryApi {
    fn list(&mut self) -> Result<Vec<RegisteredThread>>;
    fn delete(&mut self, id: &str) -> Result<()>;
}
impl RegistryApi for AppServer {
    fn list(&mut self) -> Result<Vec<RegisteredThread>> {
        self.complete_threads()
    }
    fn delete(&mut self, id: &str) -> Result<()> {
        self.request("thread/delete", json!({"threadId":id}))
            .map(|_| ())
    }
}

pub(crate) fn delete_sessions(
    targets: &[DeleteSessionTarget],
    approved: &[String],
    report_progress: &mut dyn FnMut(DeleteProgress),
) -> Result<Vec<DeleteSessionResult>> {
    delete_using(
        &mut AppServer::connect()?,
        targets,
        approved,
        report_progress,
    )
}
fn delete_using(
    api: &mut impl RegistryApi,
    targets: &[DeleteSessionTarget],
    approved: &[String],
    report_progress: &mut dyn FnMut(DeleteProgress),
) -> Result<Vec<DeleteSessionResult>> {
    report_progress(DeleteProgress::CheckingScope);
    let before = api.list()?;
    let plan = deletion_plan(&before, targets)?;
    let approved: HashSet<_> = approved.iter().map(String::as_str).collect();
    if plan.skipped_guardian_count > 0
        || plan
            .affected_ids
            .iter()
            .any(|id| !approved.contains(id.as_str()))
    {
        bail!("삭제 영향 범위가 변경됐습니다. 목록을 새로 확인하고 다시 승인해 주세요.");
    }
    let by_id: HashMap<_, _> = before.iter().map(|t| (t.id.as_str(), t)).collect();
    let selected: HashSet<_> = plan.targets.iter().map(|t| t.session_id.as_str()).collect();
    let mut roots: Vec<_> = plan
        .targets
        .iter()
        .filter(|target| {
            let mut cursor = by_id
                .get(target.session_id.as_str())
                .and_then(|t| parent_id(t));
            let mut seen = HashSet::new();
            while let Some(id) = cursor {
                if !seen.insert(id) {
                    break;
                }
                if id != target.session_id && selected.contains(id) {
                    return false;
                }
                cursor = by_id.get(id).and_then(|t| parent_id(t));
            }
            true
        })
        .cloned()
        .collect();
    // 순환 메타데이터에도 승인된 대상을 빠뜨리지 않는다.
    let mut covered = HashSet::new();
    for root in &roots {
        covered.extend(deletion_plan(&before, std::slice::from_ref(root))?.affected_ids);
    }
    for target in &plan.targets {
        if !covered.contains(&target.session_id) {
            roots.push(target.clone());
            covered.extend(deletion_plan(&before, std::slice::from_ref(target))?.affected_ids);
        }
    }
    let mut attempts: HashMap<String, (DeleteSessionStatus, Option<String>)> = HashMap::new();
    let mut disconnected = false;
    let total = roots
        .iter()
        .filter(|root| by_id.contains_key(root.session_id.as_str()))
        .count();
    let started = Instant::now();
    let mut report_deleting = |processed| {
        report_progress(DeleteProgress::Deleting {
            processed,
            total,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    };
    let mut processed = 0;
    report_deleting(processed);
    for root in roots {
        let group = deletion_plan(&before, std::slice::from_ref(&root))?.affected_ids;
        let outcome = if !by_id.contains_key(root.session_id.as_str()) {
            (DeleteSessionStatus::AlreadyMissing, None)
        } else if disconnected {
            (
                DeleteSessionStatus::Failed,
                Some("통신 중단으로 삭제 요청을 보내지 않았습니다.".into()),
            )
        } else {
            let result = api.delete(&root.session_id);
            processed += 1;
            report_deleting(processed);
            match result {
                Ok(()) => (DeleteSessionStatus::Deleted, None),
                Err(error) => {
                    let rejected = error.downcast_ref::<RpcError>().is_some();
                    if !rejected {
                        disconnected = true;
                    }
                    (
                        if rejected {
                            DeleteSessionStatus::Failed
                        } else {
                            DeleteSessionStatus::Unconfirmed
                        },
                        Some(format!("{error:#}")),
                    )
                }
            }
        };
        for id in group {
            attempts.entry(id).or_insert_with(|| outcome.clone());
        }
    }
    report_progress(DeleteProgress::Verifying);
    let after = api
        .list()
        .map(|threads| threads.into_iter().map(|t| t.id).collect::<HashSet<_>>());
    let mut results = Vec::new();
    for id in plan.affected_ids {
        let (attempt, error) = attempts.remove(&id).unwrap_or((
            DeleteSessionStatus::Failed,
            Some("삭제 요청을 처리하지 못했습니다.".into()),
        ));
        let (status, error) = match &after {
            Ok(ids) if !ids.contains(&id) => (
                if by_id.contains_key(id.as_str()) {
                    DeleteSessionStatus::Deleted
                } else {
                    DeleteSessionStatus::AlreadyMissing
                },
                None,
            ),
            Ok(_) => (
                if attempt == DeleteSessionStatus::Unconfirmed {
                    attempt
                } else {
                    DeleteSessionStatus::Failed
                },
                Some(error.unwrap_or_else(|| "삭제 후에도 Codex에 세션이 등록돼 있습니다.".into())),
            ),
            Err(_) if attempt == DeleteSessionStatus::AlreadyMissing => (attempt, error),
            Err(verification) => (
                DeleteSessionStatus::Unconfirmed,
                Some(format!("삭제 결과 확인 실패: {verification:#}")),
            ),
        };
        results.push(DeleteSessionResult {
            session_id: id,
            status,
            error,
            warning: None,
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    const P: &str = "11111111-1111-4111-8111-111111111111";
    const C: &str = "22222222-2222-4222-8222-222222222222";
    const O: &str = "33333333-3333-4333-8333-333333333333";
    fn thread(id: &str, parent: Option<&str>) -> RegisteredThread {
        serde_json::from_value(
            json!({"id":id,"parentThreadId":parent,"cwd":"project", "path":null,
            "preview":"preview only", "historyMode":"paginated", "cliVersion":"0.166.0",
            "createdAt":1,"updatedAt":2,"source":"cli","ephemeral":false}),
        )
        .unwrap()
    }
    fn targets(ids: &[&str]) -> Vec<DeleteSessionTarget> {
        ids.iter()
            .map(|id| DeleteSessionTarget {
                session_id: (*id).into(),
                guardian_only: false,
            })
            .collect()
    }
    #[test]
    fn guardian_plan_skips_branches_with_other_kinds_but_includes_archived_guardians() {
        let mut parent = thread(P, None);
        parent.source = json!({"subAgent":{"other":"guardian"}});
        let mut other = thread(O, None);
        other.source = parent.source.clone();
        other.archived = true;
        let threads = vec![parent, thread(C, Some(P)), other];
        let targets = serde_json::from_value::<Vec<DeleteSessionTarget>>(json!([
            {"sessionId":P,"guardianOnly":true},
            {"sessionId":C,"guardianOnly":true},
            {"sessionId":O,"guardianOnly":true}
        ]))
        .unwrap();

        let plan = deletion_plan(&threads, &targets).unwrap();
        assert_eq!(plan.affected_ids, vec![O.to_owned()]);
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.skipped_guardian_count, 2);
    }

    #[test]
    fn guardian_delete_rechecks_kind_and_descendants_before_any_request() {
        let mut parent = thread(P, None);
        parent.source = json!({"subAgent":{"other":"guardian"}});
        let mut child = thread(C, Some(P));
        child.source = parent.source.clone();
        let target = DeleteSessionTarget {
            session_id: P.into(),
            guardian_only: true,
        };
        let approved = deletion_plan(
            &[parent.clone(), child.clone()],
            std::slice::from_ref(&target),
        )
        .unwrap()
        .affected_ids;
        child.source = json!("cli");
        let mut api = Fake {
            threads: vec![parent, child],
            requests: vec![],
            remain: false,
            uncertain: false,
            fail_verify: false,
            lists: 0,
        };
        assert!(delete_using(&mut api, &[target], &approved, &mut |_| {}).is_err());
        assert!(api.requests.is_empty());
    }

    thread_local! {static MAP_CLOCK:std::cell::Cell<Option<(std::time::Instant,usize,usize)>>=const{std::cell::Cell::new(None)};}
    fn mapping_now() -> std::time::Instant {
        MAP_CLOCK.with(|c| {
            let (base, calls, expire) = c.get().unwrap();
            c.set(Some((base, calls + 1, expire)));
            base + std::time::Duration::from_secs(if calls >= expire { 61 } else { 0 })
        })
    }
    fn acquired_snapshot(file: &Path) -> super::super::full_catalog::ThreadSnapshot {
        let mut worker = thread(C, Some(P));
        worker.source = json!({"subAgent":{"thread_spawn":{"parent_thread_id":P}}});
        let mut guardian = thread(O, None);
        guardian.source = json!({"subAgent":{"other":"guardian"}});
        let mut threads = vec![thread(P, None), worker, guardian];
        for row in &mut threads {
            row.path = Some(file.to_string_lossy().into_owned());
        }
        super::super::full_catalog::ThreadSnapshot {
            threads,
            missing_count: 2,
            errors: vec!["existing RPC timeout".into()],
        }
    }
    #[test]
    fn fix_round_2_preserves_all_collected_rows_when_budget_already_expired() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("record.jsonl");
        std::fs::write(&file, "record").unwrap();
        MAP_CLOCK.with(|c| c.set(Some((std::time::Instant::now(), 0, 1))));
        let budget = RefreshBudget::new(mapping_now);
        let out = map_collected_threads(acquired_snapshot(&file), &Config::default(), budget);
        assert_eq!(
            out.sessions.len(),
            3,
            "expired API collection must preserve every acquired row"
        );
        assert_eq!(out.missing_count, 2);
        assert!(!out.complete);
        assert!(out.errors.contains(&"existing RPC timeout".into()));
        assert_eq!(out.sessions[1].parent_id.as_deref(), Some(P));
        assert_eq!(
            out.sessions[1].session_kind,
            crate::types::SessionKind::Worker
        );
        assert_eq!(
            out.sessions[2].session_kind,
            crate::types::SessionKind::Guardian
        );
        assert!(out
            .sessions
            .iter()
            .all(|s| s.file_path.is_none() && s.size.is_none()));
    }
    #[test]
    fn fix_round_2_preserves_remaining_rows_when_mapping_budget_expires() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("record.jsonl");
        std::fs::write(&file, "record").unwrap();
        MAP_CLOCK.with(|c| c.set(Some((std::time::Instant::now(), 0, 3))));
        let budget = RefreshBudget::new(mapping_now);
        let out = map_collected_threads(acquired_snapshot(&file), &Config::default(), budget);
        assert_eq!(
            out.sessions.len(),
            3,
            "mapping expiry must not discard remaining metadata"
        );
        assert!(!out.complete);
        assert_eq!(out.missing_count, 2);
        assert!(out.errors.contains(&"existing RPC timeout".into()));
        assert!(out
            .sessions
            .iter()
            .any(|s| s.file_path.is_none() && s.size.is_none()));
        assert_eq!(out.sessions[1].parent_id.as_deref(), Some(P));
    }
    #[test]
    fn mapping_distinguishes_guardian_worker_unknown_and_summary_without_hidden() {
        for (source, parent, kind, worker) in [
            (
                json!({"subAgent":{"other":"guardian"}}),
                Some(P),
                "guardian",
                false,
            ),
            (
                json!({"subAgent":{"thread_spawn":{"parent_thread_id":P}}}),
                Some(P),
                "worker",
                true,
            ),
            (json!({"subAgent":"review"}), None, "otherInternal", false),
            (json!("cli"), None, "main", false),
            (json!("future"), None, "unknown", false),
        ] {
            let mut item = thread(C, parent);
            item.source = source;
            item.cwd = "C:/summary-runs".into();
            let mapped = serde_json::to_value(map_thread(item, &Config::default())).unwrap();
            assert_eq!(mapped["sessionKind"], kind);
            assert_eq!(mapped["isSubagent"], worker);
            assert_eq!(mapped["isSummaryRun"], true);
            assert!(mapped.get("hidden").is_none());
        }
    }
    #[test]
    fn mapping_uses_nickname_before_preview_and_ignores_blank_parent_field() {
        let mut child = thread(C, Some(""));
        child.agent_nickname = Some("Builder".into());
        child.source = json!({"subAgent":{"thread_spawn":{"parent_thread_id":P}}});
        let mapped = map_thread(child, &Config::default());
        assert_eq!(mapped.parent_id.as_deref(), Some(P));
        assert_eq!(mapped.name.as_deref(), Some("Builder"));
        let mut guardian = thread(O, None);
        guardian.preview.clear();
        guardian.source = json!({"subAgent":{"other":"guardian"}});
        assert_eq!(
            map_thread(guardian, &Config::default()).name.as_deref(),
            Some("Guardian · 33333333")
        );
    }
    #[test]
    fn db_only_mapping_preserves_registration_and_separates_preview() {
        let mapped = map_thread(thread(P, None), &Config::default());
        assert!(mapped.registered);
        assert!(mapped.file_path.is_none());
        assert!(mapped.size.is_none());
        assert_eq!(mapped.preview.as_deref(), Some("preview only"));
        assert!(mapped.first_user_message.is_none());
        assert_eq!(mapped.session_kind, crate::types::SessionKind::Main);
    }
    #[test]
    fn scope_includes_descendants_and_validates_uuid_before_commands() {
        let list = vec![thread(P, None), thread(C, Some(P)), thread(O, None)];
        let plan = deletion_plan(&list, &targets(&[P, P])).unwrap();
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.affected_ids, vec![P.to_string(), C.to_string()]);
        assert!(deletion_plan(&list, &targets(&["--force"])).is_err());
    }
    struct Fake {
        threads: Vec<RegisteredThread>,
        requests: Vec<String>,
        remain: bool,
        uncertain: bool,
        fail_verify: bool,
        lists: usize,
    }
    impl RegistryApi for Fake {
        fn list(&mut self) -> Result<Vec<RegisteredThread>> {
            self.lists += 1;
            if self.fail_verify && self.lists > 1 {
                bail!("verification disconnected");
            }
            Ok(self.threads.clone())
        }
        fn delete(&mut self, id: &str) -> Result<()> {
            self.requests.push(id.into());
            if self.uncertain {
                bail!("timeout");
            }
            if !self.remain {
                self.threads
                    .retain(|t| t.id != id && parent_id(t) != Some(id));
            }
            Ok(())
        }
    }
    fn fake() -> Fake {
        Fake {
            threads: vec![thread(P, None), thread(C, Some(P)), thread(O, None)],
            requests: vec![],
            remain: false,
            uncertain: false,
            fail_verify: false,
            lists: 0,
        }
    }
    #[test]
    fn deletion_verifies_absence_and_deduplicates_selected_parent_and_child() {
        let mut api = fake();
        let results = delete_using(
            &mut api,
            &targets(&[P, C]),
            &[P.into(), C.into()],
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(api.requests, vec![P.to_string()]);
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| r.status == DeleteSessionStatus::Deleted));
    }
    #[test]
    fn success_response_with_retained_registration_is_a_failure() {
        let mut api = fake();
        api.remain = true;
        let results =
            delete_using(&mut api, &targets(&[P]), &[P.into(), C.into()], &mut |_| {}).unwrap();
        assert!(results
            .iter()
            .all(|r| r.status == DeleteSessionStatus::Failed));
    }
    #[test]
    fn deletion_progress_counts_requests_and_checks_results_afterwards() {
        const MISSING: &str = "44444444-4444-4444-8444-444444444444";
        let mut api = fake();
        let mut progress = Vec::new();
        let result = delete_using(
            &mut api,
            &targets(&[P, C, O, MISSING]),
            &[P.into(), C.into(), O.into(), MISSING.into()],
            &mut |update| progress.push(serde_json::to_value(update).unwrap()),
        )
        .unwrap();
        assert_eq!(api.requests, [P, O]);
        assert_eq!(result.len(), 4);
        assert_eq!(
            progress
                .iter()
                .map(|update| update["phase"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "checkingScope",
                "deleting",
                "deleting",
                "deleting",
                "verifying"
            ]
        );
        let requests = &progress[1..4];
        for (processed, update) in requests.iter().enumerate() {
            assert_eq!(update["processed"], processed);
            assert_eq!(update["total"], 2);
            assert!(update["elapsedMs"].as_u64().is_some());
        }
        assert!(requests
            .windows(2)
            .all(|pair| pair[0]["elapsedMs"].as_u64() <= pair[1]["elapsedMs"].as_u64()));
    }

    #[test]
    fn deletion_progress_does_not_count_unattempted_requests_after_disconnect() {
        let mut api = fake();
        api.uncertain = true;
        let mut progress = Vec::new();
        delete_using(
            &mut api,
            &targets(&[P, O]),
            &[P.into(), C.into(), O.into()],
            &mut |update| progress.push(serde_json::to_value(update).unwrap()),
        )
        .unwrap();
        assert_eq!(api.requests, [P]);
        assert_eq!(progress.len(), 4);
        assert_eq!(progress[2]["processed"], 1);
        assert_eq!(progress[2]["total"], 2);
        assert_eq!(progress[3], json!({"phase":"verifying"}));
    }

    #[test]
    fn scope_growth_prevents_any_delete_request() {
        let mut api = fake();
        assert!(delete_using(&mut api, &targets(&[P]), &[P.into()], &mut |_| {}).is_err());
        assert!(api.requests.is_empty());
    }
    #[test]
    fn transport_uncertainty_stops_further_requests_and_never_claims_success() {
        let mut api = fake();
        api.uncertain = true;
        let results = delete_using(
            &mut api,
            &targets(&[P, O]),
            &[P.into(), C.into(), O.into()],
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(api.requests.len(), 1);
        assert!(results
            .iter()
            .all(|r| r.status != DeleteSessionStatus::Deleted));
        assert_eq!(
            results.iter().find(|r| r.session_id == P).unwrap().status,
            DeleteSessionStatus::Unconfirmed
        );
    }
    #[test]
    fn verification_failure_does_not_mean_already_missing() {
        let mut api = fake();
        api.fail_verify = true;
        let results =
            delete_using(&mut api, &targets(&[P]), &[P.into(), C.into()], &mut |_| {}).unwrap();
        assert!(results
            .iter()
            .all(|r| r.status == DeleteSessionStatus::Unconfirmed));
    }
}
