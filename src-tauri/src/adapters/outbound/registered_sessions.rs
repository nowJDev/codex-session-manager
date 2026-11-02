// Codex 등록 세션을 앱 모델로 변환하고 ID 삭제의 결과를 확인한다.
use super::codex_app_server::{AppServer, RegisteredThread, RpcError};
use crate::types::{
    Config, DeleteSessionResult, DeleteSessionStatus, DeleteSessionTarget, DeletionPlan, Session,
};
use anyhow::{bail, Result};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) fn list_sessions() -> Result<Vec<Session>> {
    let threads = AppServer::connect()?.list_threads()?;
    let config = crate::config::load_config();
    let ids: HashSet<_> = threads.iter().map(|t| t.id.clone()).collect();
    let mut sessions: Vec<_> = threads
        .into_iter()
        .filter(|t| !t.ephemeral)
        .map(|t| map_thread(t, &config))
        .collect();
    for session in crate::scanner::scan_extra_sessions()? {
        if !ids.contains(&session.session_id) {
            sessions.push(session);
        }
    }
    Ok(sessions)
}

fn parent_id(thread: &RegisteredThread) -> Option<&str> {
    thread.parent_thread_id.as_deref().or_else(|| {
        thread
            .source
            .pointer("/subAgent/thread_spawn/parent_thread_id")
            .and_then(serde_json::Value::as_str)
    })
}

fn map_thread(thread: RegisteredThread, config: &Config) -> Session {
    let meta = config.sessions.get(&thread.id).cloned().unwrap_or_default();
    let path = thread
        .path
        .as_ref()
        .filter(|p| Path::new(p).is_file())
        .cloned();
    let size = path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len());
    let exclusions = config
        .settings
        .excluded_scan_paths
        .clone()
        .unwrap_or_default();
    let encoded: Vec<_> = exclusions
        .iter()
        .map(|p| crate::domain::session::encode_scan_match_text(p))
        .collect();
    let hidden = crate::domain::session::is_internal_summary_run(Some(&thread.cwd))
        || thread
            .source
            .pointer("/subAgent/other")
            .and_then(serde_json::Value::as_str)
            == Some("guardian")
        || crate::domain::session::scan_exclusion_matches(
            thread.path.as_deref().unwrap_or(""),
            Some(&thread.cwd),
            &exclusions,
            &encoded,
        );
    let parent = parent_id(&thread).map(str::to_owned);
    let is_subagent = parent.is_some() || thread.source.get("subAgent").is_some();
    let project = Path::new(&thread.cwd)
        .file_name()
        .and_then(|p| p.to_str())
        .unwrap_or("Codex")
        .to_owned();
    let timestamp = |seconds| chrono::DateTime::from_timestamp(seconds, 0).map(|d| d.to_rfc3339());
    let preview = (!thread.preview.trim().is_empty()).then_some(thread.preview);
    let name = meta.name.or(thread.name).or_else(|| {
        preview
            .as_deref()
            .and_then(crate::domain::transcript::first_user_message_name)
    });
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
        file_path: path,
        size,
        preview,
        history_mode: thread.history_mode,
        registered: true,
        hidden,
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
    let threads = AppServer::connect()?.list_threads()?;
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
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for thread in threads {
        if let Some(parent) = parent_id(thread) {
            children.entry(parent).or_default().push(&thread.id);
        }
    }
    let mut stack: Vec<_> = selected.iter().map(String::as_str).collect();
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
    })
}

trait RegistryApi {
    fn list(&mut self) -> Result<Vec<RegisteredThread>>;
    fn delete(&mut self, id: &str) -> Result<()>;
}
impl RegistryApi for AppServer {
    fn list(&mut self) -> Result<Vec<RegisteredThread>> {
        self.list_threads()
    }
    fn delete(&mut self, id: &str) -> Result<()> {
        self.request("thread/delete", json!({"threadId":id}))
            .map(|_| ())
    }
}

pub(crate) fn delete_sessions(
    targets: &[DeleteSessionTarget],
    approved: &[String],
) -> Result<Vec<DeleteSessionResult>> {
    delete_using(&mut AppServer::connect()?, targets, approved)
}
fn delete_using(
    api: &mut impl RegistryApi,
    targets: &[DeleteSessionTarget],
    approved: &[String],
) -> Result<Vec<DeleteSessionResult>> {
    let before = api.list()?;
    let plan = deletion_plan(&before, targets)?;
    let approved: HashSet<_> = approved.iter().map(String::as_str).collect();
    if plan
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
            match api.delete(&root.session_id) {
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
            })
            .collect()
    }
    #[test]
    fn db_only_mapping_preserves_registration_and_separates_preview() {
        let mapped = map_thread(thread(P, None), &Config::default());
        assert!(mapped.registered);
        assert!(mapped.file_path.is_none());
        assert!(mapped.size.is_none());
        assert_eq!(mapped.preview.as_deref(), Some("preview only"));
        assert!(mapped.first_user_message.is_none());
        assert!(!mapped.hidden);
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
        let results = delete_using(&mut api, &targets(&[P, C]), &[P.into(), C.into()]).unwrap();
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
        let results = delete_using(&mut api, &targets(&[P]), &[P.into(), C.into()]).unwrap();
        assert!(results
            .iter()
            .all(|r| r.status == DeleteSessionStatus::Failed));
    }
    #[test]
    fn scope_growth_prevents_any_delete_request() {
        let mut api = fake();
        assert!(delete_using(&mut api, &targets(&[P]), &[P.into()]).is_err());
        assert!(api.requests.is_empty());
    }
    #[test]
    fn transport_uncertainty_stops_further_requests_and_never_claims_success() {
        let mut api = fake();
        api.uncertain = true;
        let results =
            delete_using(&mut api, &targets(&[P, O]), &[P.into(), C.into(), O.into()]).unwrap();
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
        let results = delete_using(&mut api, &targets(&[P]), &[P.into(), C.into()]).unwrap();
        assert!(results
            .iter()
            .all(|r| r.status == DeleteSessionStatus::Unconfirmed));
    }
}
