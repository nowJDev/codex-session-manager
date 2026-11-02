// 세션 관리 use case를 조율하는 애플리케이션 서비스이다.
use crate::application::ports::{ResumePlan, SessionPorts};
use crate::domain::session::{classify_storage_state, is_retryable_auto_summary, StorageState};
use crate::types::{
    Config, DeleteSessionResult, DeleteSessionStatus, DeleteSessionTarget, DeletionPlan, Session,
    SessionMeta, Settings,
};
use anyhow::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct SessionService<P> {
    ports: P,
}

impl<P: SessionPorts> SessionService<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let local = self.ports.scan_local_sessions()?;
        let cloud_all = self.ports.list_cloud_sessions().unwrap_or_default();
        Ok(merge_local_and_cloud_sessions(local, cloud_all))
    }

    pub fn get_config(&self) -> Config {
        self.ports.load_config()
    }

    pub fn save_session_meta(&self, session_id: &str, patch: SessionMeta) -> Result<()> {
        self.ports.save_session_meta(session_id, patch)
    }

    pub fn prepare_session_deletion(
        &self,
        targets: &[DeleteSessionTarget],
    ) -> Result<DeletionPlan> {
        self.ports.prepare_session_deletion(targets)
    }

    pub fn delete_session(
        &self,
        session_id: &str,
        approved_ids: &[String],
    ) -> Result<DeleteSessionResult> {
        self.delete_sessions(
            vec![DeleteSessionTarget {
                session_id: session_id.into(),
            }],
            approved_ids,
        )?
        .into_iter()
        .find(|r| r.session_id == session_id)
        .ok_or_else(|| anyhow::anyhow!("삭제 결과가 없습니다."))
    }

    pub fn delete_session_meta(&self, session_id: &str) -> Result<()> {
        self.ports.delete_session_meta(session_id)
    }

    pub fn delete_sessions(
        &self,
        targets: Vec<DeleteSessionTarget>,
        approved_ids: &[String],
    ) -> Result<Vec<DeleteSessionResult>> {
        let mut results = self.ports.delete_sessions(&targets, approved_ids)?;
        for result in &mut results {
            if matches!(
                result.status,
                DeleteSessionStatus::Deleted | DeleteSessionStatus::AlreadyMissing
            ) {
                if let Err(error) = self.ports.delete_session_meta(&result.session_id) {
                    result.warning = Some(format!(
                        "세션은 삭제됐지만 앱 메타데이터 정리에 실패했습니다: {error:#}"
                    ));
                }
            }
        }
        Ok(results)
    }

    pub fn archive_session(&self, session_id: &str) -> Result<()> {
        self.ports.archive_session(session_id)
    }

    pub fn unarchive_session(&self, session_id: &str) -> Result<()> {
        self.ports.unarchive_session(session_id)
    }

    pub fn save_settings(&self, patch: Settings) -> Result<()> {
        self.ports.update_settings(patch)
    }

    pub fn set_cloud_folder(&self, root: &str) -> Result<PathBuf> {
        let folder = self.ports.set_cloud_folder(root)?;
        self.ports.update_settings(Settings {
            cloud_path: Some(folder.to_string_lossy().to_string()),
            ..Default::default()
        })?;
        Ok(folder)
    }

    pub fn upload_to_cloud(&self, session: &Session) -> Result<()> {
        self.ports.upload_to_cloud(session)?;
        self.ports.save_session_meta(
            &session.session_id,
            SessionMeta {
                storage_type: Some("cloud".into()),
                ..Default::default()
            },
        )
    }

    pub fn checkout_session(&self, session: &Session) -> Result<String> {
        self.ports.checkout_session(session)
    }

    pub fn checkin_session(&self, session: &Session) -> Result<()> {
        self.ports.checkin_session(session)
    }

    pub fn resume_session(&self, session_id: &str, cwd: Option<&str>) -> Result<()> {
        self.ports.resume_session(session_id, cwd)
    }

    pub fn build_resume_plan(&self, session_id: &str, cwd: Option<&str>) -> Result<ResumePlan> {
        self.ports.build_resume_plan(session_id, cwd)
    }

    pub fn get_session_messages(
        &self,
        file_path: &str,
        max_messages: usize,
    ) -> Result<Vec<String>> {
        self.ports.get_session_messages(file_path, max_messages)
    }

    pub fn pending_auto_summary_batch(&self, batch_size: usize) -> Result<Vec<(String, String)>> {
        Ok(self
            .ports
            .scan_local_sessions()?
            .into_iter()
            .filter(|session| {
                session.registered
                    && !session.hidden
                    && session.history_mode == "legacy"
                    && session.file_path.is_some()
                    && session.description.as_deref().unwrap_or("").is_empty()
                    && is_retryable_auto_summary(session.auto_summary.as_deref())
            })
            .take(batch_size)
            .filter_map(|session| session.file_path.map(|path| (session.session_id, path)))
            .collect())
    }

    pub fn summarize_batch(
        &self,
        pending: &[(String, String)],
    ) -> Result<HashMap<String, (String, String)>> {
        self.ports.summarize_batch(pending)
    }

    pub fn save_summary(&self, session_id: &str, name: String, description: String) -> Result<()> {
        self.ports.save_session_meta(
            session_id,
            SessionMeta {
                name: Some(name),
                auto_summary: Some(description),
                ..Default::default()
            },
        )
    }

    pub fn mark_summary_missing(&self, session_id: &str) -> Result<()> {
        self.ports.save_session_meta(
            session_id,
            SessionMeta {
                auto_summary: Some("(요약 누락 — 재시도 예정)".into()),
                ..Default::default()
            },
        )
    }

    pub fn mark_summary_failed(&self, session_id: &str, error: &str) -> Result<()> {
        self.ports.save_session_meta(
            session_id,
            SessionMeta {
                auto_summary: Some(format!("(자동 요약 실패: {})", error)),
                ..Default::default()
            },
        )
    }

    pub fn generate_summary_pair(&self, session_id: &str) -> Result<(String, String)> {
        let cfg = self.ports.load_config();
        let previous_summary = cfg
            .sessions
            .get(session_id)
            .and_then(|meta| meta.description.clone().or(meta.auto_summary.clone()));

        let (name, description) = self
            .ports
            .summarize_session(session_id, previous_summary.as_deref())?;
        self.save_summary(session_id, name.clone(), description.clone())?;
        Ok((name, description))
    }

    pub fn generate_summary(&self, session_id: &str) -> Result<String> {
        self.generate_summary_pair(session_id)
            .map(|(_name, description)| description)
    }
}

fn merge_local_and_cloud_sessions(
    mut local: Vec<Session>,
    cloud_all: Vec<Session>,
) -> Vec<Session> {
    let cloud_ids: HashSet<String> = cloud_all.iter().map(|c| c.session_id.clone()).collect();
    for session in local.iter_mut() {
        if let Some(state) = classify_storage_state(true, cloud_ids.contains(&session.session_id)) {
            session.storage_type = storage_type_value(state).into();
        }
    }

    let local_ids: HashSet<String> = local.iter().map(|s| s.session_id.clone()).collect();
    let cloud_only = cloud_all
        .into_iter()
        .filter(|session| !local_ids.contains(&session.session_id))
        .map(|mut session| {
            if let Some(state) = classify_storage_state(false, true) {
                session.storage_type = storage_type_value(state).into();
            }
            session
        });
    local.extend(cloud_only);
    local
}

fn storage_type_value(state: StorageState) -> &'static str {
    match state {
        StorageState::Synced => "synced",
        StorageState::LocalOnly => "local-only",
        StorageState::CloudOnly => "cloud-only",
    }
}

#[cfg(test)]
mod tests {
    use super::SessionService;
    use crate::application::ports::{
        CloudSyncPort, ResumePlan, ResumePort, SessionCommandPort, SessionMessagePort,
        SessionMetadataPort, SessionScanPort, SummaryPort,
    };
    use crate::types::{
        Config, DeleteSessionResult, DeleteSessionStatus, DeleteSessionTarget, DeletionPlan,
        Session, SessionMeta, Settings,
    };
    use anyhow::Result;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[derive(Default)]
    struct FakeState {
        local: Vec<Session>,
        cloud: Vec<Session>,
        config: Config,
        saved_meta: Vec<(String, SessionMeta)>,
        deleted_meta: Vec<String>,
        deleted_sessions: Vec<(String, String)>,
        delete_errors: HashMap<String, String>,
        already_missing: Vec<String>,
        metadata_errors: HashMap<String, String>,
        updated_settings: Vec<Settings>,
        cloud_folders: Vec<String>,
        uploaded_sessions: Vec<String>,
        summaries: HashMap<String, (String, String)>,
    }

    #[derive(Clone, Default)]
    struct FakePorts {
        state: Rc<RefCell<FakeState>>,
    }

    impl SessionScanPort for FakePorts {
        fn scan_local_sessions(&self) -> Result<Vec<Session>> {
            Ok(self.state.borrow().local.clone())
        }
    }

    impl SessionMessagePort for FakePorts {
        fn get_session_messages(
            &self,
            _file_path: &str,
            _max_messages: usize,
        ) -> Result<Vec<String>> {
            Ok(vec!["message".into()])
        }
    }

    impl SessionCommandPort for FakePorts {
        fn prepare_session_deletion(
            &self,
            targets: &[DeleteSessionTarget],
        ) -> Result<DeletionPlan> {
            Ok(DeletionPlan {
                targets: targets.to_vec(),
                affected_ids: targets.iter().map(|t| t.session_id.clone()).collect(),
            })
        }
        fn delete_sessions(
            &self,
            targets: &[DeleteSessionTarget],
            _approved_ids: &[String],
        ) -> Result<Vec<DeleteSessionResult>> {
            Ok(targets
                .iter()
                .map(|target| {
                    let id = &target.session_id;
                    let mut state = self.state.borrow_mut();
                    state.deleted_sessions.push((id.clone(), String::new()));
                    let error = state.delete_errors.get(id).cloned();
                    let status = if error.is_some() {
                        DeleteSessionStatus::Failed
                    } else if state.already_missing.contains(id) {
                        DeleteSessionStatus::AlreadyMissing
                    } else {
                        DeleteSessionStatus::Deleted
                    };
                    DeleteSessionResult {
                        session_id: id.clone(),
                        status,
                        error,
                        warning: None,
                    }
                })
                .collect())
        }

        fn archive_session(&self, _session_id: &str) -> Result<()> {
            Ok(())
        }

        fn unarchive_session(&self, _session_id: &str) -> Result<()> {
            Ok(())
        }
    }

    impl SessionMetadataPort for FakePorts {
        fn load_config(&self) -> Config {
            self.state.borrow().config.clone()
        }

        fn save_session_meta(&self, session_id: &str, patch: SessionMeta) -> Result<()> {
            self.state
                .borrow_mut()
                .saved_meta
                .push((session_id.to_string(), patch));
            Ok(())
        }

        fn delete_session_meta(&self, session_id: &str) -> Result<()> {
            if let Some(error) = self.state.borrow().metadata_errors.get(session_id) {
                anyhow::bail!("{error}");
            }
            self.state
                .borrow_mut()
                .deleted_meta
                .push(session_id.to_string());
            Ok(())
        }

        fn update_settings(&self, patch: Settings) -> Result<()> {
            self.state.borrow_mut().updated_settings.push(patch);
            Ok(())
        }
    }

    impl CloudSyncPort for FakePorts {
        fn list_cloud_sessions(&self) -> Result<Vec<Session>> {
            Ok(self.state.borrow().cloud.clone())
        }

        fn set_cloud_folder(&self, root: &str) -> Result<PathBuf> {
            self.state.borrow_mut().cloud_folders.push(root.to_string());
            Ok(PathBuf::from("cloud"))
        }

        fn upload_to_cloud(&self, session: &Session) -> Result<()> {
            self.state
                .borrow_mut()
                .uploaded_sessions
                .push(session.session_id.clone());
            Ok(())
        }

        fn checkout_session(&self, _session: &Session) -> Result<String> {
            Ok("checked-out.jsonl".into())
        }

        fn checkin_session(&self, _session: &Session) -> Result<()> {
            Ok(())
        }
    }

    impl ResumePort for FakePorts {
        fn resume_session(&self, _session_id: &str, _cwd: Option<&str>) -> Result<()> {
            Ok(())
        }

        fn build_resume_plan(&self, session_id: &str, _cwd: Option<&str>) -> Result<ResumePlan> {
            Ok(ResumePlan {
                program: "codex".into(),
                args: vec!["resume".into(), session_id.into()],
            })
        }
    }

    impl SummaryPort for FakePorts {
        fn summarize_batch(
            &self,
            _items: &[(String, String)],
        ) -> Result<HashMap<String, (String, String)>> {
            Ok(self.state.borrow().summaries.clone())
        }

        fn summarize_session(
            &self,
            _file_path: &str,
            previous_summary: Option<&str>,
        ) -> Result<(String, String)> {
            Ok((
                "generated".into(),
                format!("summary after {}", previous_summary.unwrap_or("none")),
            ))
        }
    }

    fn session(id: &str, file_path: &str) -> Session {
        Session {
            session_id: id.into(),
            parent_id: None,
            is_subagent: false,
            agent_nickname: None,
            name: None,
            description: None,
            auto_summary: None,
            project: "Agent".into(),
            project_dir: "C:/Agent".into(),
            file_path: Some(file_path.into()),
            size: Some(1),
            preview: None,
            history_mode: "legacy".into(),
            registered: true,
            hidden: false,
            total_lines: 1,
            first_timestamp: None,
            last_timestamp: None,
            cwd: None,
            version: None,
            first_user_message: Some("hello".into()),
            storage_type: "local".into(),
            archived: false,
            favorite: false,
            locked_by: None,
        }
    }

    #[test]
    fn list_sessions_merges_storage_state_without_real_files() {
        let ports = FakePorts::default();
        {
            let mut state = ports.state.borrow_mut();
            state.local = vec![
                session("same", "local.jsonl"),
                session("local", "only.jsonl"),
            ];
            state.cloud = vec![
                session("same", "cloud.jsonl"),
                session("cloud", "cloud.jsonl"),
            ];
        }

        let sessions = SessionService::new(ports).list_sessions().unwrap();
        let by_id = sessions
            .iter()
            .map(|session| (session.session_id.as_str(), session.storage_type.as_str()))
            .collect::<HashMap<_, _>>();

        assert_eq!(by_id.get("same"), Some(&"synced"));
        assert_eq!(by_id.get("local"), Some(&"local-only"));
        assert_eq!(by_id.get("cloud"), Some(&"cloud-only"));
    }

    #[test]
    fn delete_session_removes_file_and_metadata_through_ports() {
        let ports = FakePorts::default();
        let state = ports.state.clone();

        SessionService::new(ports)
            .delete_session("s1", &["s1".into()])
            .unwrap();

        let state = state.borrow();
        assert_eq!(
            state.deleted_sessions,
            vec![("s1".to_string(), String::new())]
        );
        assert_eq!(state.deleted_meta, vec!["s1".to_string()]);
    }

    #[test]
    fn bulk_delete_continues_after_failure_and_preserves_failed_metadata() {
        let ports = FakePorts::default();
        ports
            .state
            .borrow_mut()
            .delete_errors
            .insert("bad".into(), "locked".into());
        let state = ports.state.clone();
        let targets = ["first", "bad", "last"].map(|id| crate::types::DeleteSessionTarget {
            session_id: id.into(),
        });
        let results = SessionService::new(ports)
            .delete_sessions(targets.to_vec(), &[])
            .unwrap();
        assert_eq!(
            results.iter().map(|r| r.status).collect::<Vec<_>>(),
            vec![
                DeleteSessionStatus::Deleted,
                DeleteSessionStatus::Failed,
                DeleteSessionStatus::Deleted
            ]
        );
        assert_eq!(results[1].error.as_deref(), Some("locked"));
        let state = state.borrow();
        assert_eq!(
            state.deleted_sessions.len(),
            3,
            "a failed item must not stop the batch"
        );
        assert_eq!(state.deleted_meta, vec!["first", "last"]);
    }

    #[test]
    fn bulk_delete_reports_missing_and_metadata_failure_separately() {
        let ports = FakePorts::default();
        ports.state.borrow_mut().already_missing.push("gone".into());
        ports
            .state
            .borrow_mut()
            .metadata_errors
            .insert("meta".into(), "metadata write failed".into());
        let state = ports.state.clone();
        let targets = ["gone", "meta", "last"].map(|id| crate::types::DeleteSessionTarget {
            session_id: id.into(),
        });
        let results = SessionService::new(ports)
            .delete_sessions(targets.to_vec(), &[])
            .unwrap();
        assert_eq!(
            results.iter().map(|r| r.status).collect::<Vec<_>>(),
            vec![
                DeleteSessionStatus::AlreadyMissing,
                DeleteSessionStatus::Deleted,
                DeleteSessionStatus::Deleted
            ]
        );
        assert!(results[1]
            .warning
            .as_deref()
            .unwrap()
            .contains("metadata write failed"));
        assert_eq!(state.borrow().deleted_meta, vec!["gone", "last"]);
    }

    #[test]
    fn set_cloud_folder_prepares_folder_and_persists_setting_through_ports() {
        let ports = FakePorts::default();
        let state = ports.state.clone();

        let folder = SessionService::new(ports)
            .set_cloud_folder("drive")
            .unwrap();

        let state = state.borrow();
        assert_eq!(folder, PathBuf::from("cloud"));
        assert_eq!(state.cloud_folders, vec!["drive".to_string()]);
        assert_eq!(state.updated_settings.len(), 1);
        assert_eq!(
            state.updated_settings[0].cloud_path.as_deref(),
            Some("cloud")
        );
    }

    #[test]
    fn upload_to_cloud_persists_storage_metadata_through_ports() {
        let ports = FakePorts::default();
        let state = ports.state.clone();
        let s = session("s1", "session.jsonl");

        SessionService::new(ports).upload_to_cloud(&s).unwrap();

        let state = state.borrow();
        assert_eq!(state.uploaded_sessions, vec!["s1".to_string()]);
        assert_eq!(state.saved_meta.len(), 1);
        assert_eq!(state.saved_meta[0].0, "s1");
        assert_eq!(state.saved_meta[0].1.storage_type.as_deref(), Some("cloud"));
    }

    #[test]
    fn generate_summary_uses_previous_summary_and_persists_result() {
        let ports = FakePorts::default();
        ports.state.borrow_mut().config.sessions.insert(
            "s1".into(),
            SessionMeta {
                auto_summary: Some("old".into()),
                ..Default::default()
            },
        );
        let state = ports.state.clone();

        let description = SessionService::new(ports).generate_summary("s1").unwrap();

        assert_eq!(description, "summary after old");
        let saved = &state.borrow().saved_meta[0];
        assert_eq!(saved.0, "s1");
        assert_eq!(saved.1.name.as_deref(), Some("generated"));
        assert_eq!(saved.1.auto_summary.as_deref(), Some("summary after old"));
    }

    #[test]
    fn registered_database_session_can_have_no_rollout_or_size() {
        let mut value = serde_json::to_value(session("db-only", "old.jsonl")).unwrap();
        value["filePath"] = serde_json::Value::Null;
        value["size"] = serde_json::Value::Null;
        value["registered"] = true.into();
        value["historyMode"] = "paginated".into();
        let parsed = serde_json::from_value::<Session>(value);
        assert!(
            parsed.is_ok(),
            "registered DB-only sessions must not require a file: {parsed:?}"
        );
    }
    #[test]
    fn paginated_stub_must_not_overwrite_existing_cloud_backup() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("stub.jsonl");
        std::fs::write(
            &file,
            r#"{"type":"session_meta","payload":{"id":"s1","history_mode":"paginated"}}"#,
        )
        .unwrap();
        let cloud = dir.path().join("cloud");
        std::fs::create_dir(&cloud).unwrap();
        let backup = cloud.join("s1.jsonl");
        std::fs::write(&backup, "preserve backup").unwrap();
        let mut s = session("s1", file.to_str().unwrap());
        s.history_mode = "legacy".into();
        assert!(
            crate::cloud::upload_session(&cloud, &s).is_err(),
            "read actual file mode instead of trusting frontend"
        );
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "preserve backup");
    }

    #[test]
    fn paginated_sessions_never_enter_legacy_auto_summary_queue() {
        let ports = FakePorts::default();
        let mut db = session("db", "stub.jsonl");
        db.history_mode = "paginated".into();
        let mut missing = session("missing", "gone.jsonl");
        missing.file_path = None;
        ports.state.borrow_mut().local = vec![db, missing, session("legacy", "complete.jsonl")];
        let pending = SessionService::new(ports)
            .pending_auto_summary_batch(5)
            .unwrap();
        assert_eq!(pending, vec![("legacy".into(), "complete.jsonl".into())]);
    }

    #[test]
    fn unidentified_or_mismatched_files_preserve_existing_backup() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("source.jsonl");
        let cloud = dir.path().join("cloud");
        std::fs::create_dir(&cloud).unwrap();
        let backup = cloud.join("s1.jsonl");
        for body in [
            "",
            "invalid json",
            r#"{"type":"event_msg"}"#,
            r#"{"type":"session_meta","payload":{"id":"different"}}"#,
        ] {
            std::fs::write(&file, body).unwrap();
            std::fs::write(&backup, "keep").unwrap();
            let s = session("s1", file.to_str().unwrap());
            assert!(
                crate::cloud::upload_session(&cloud, &s).is_err(),
                "unknown file must be refused: {body}"
            );
            assert_eq!(std::fs::read_to_string(&backup).unwrap(), "keep");
        }
    }
}
