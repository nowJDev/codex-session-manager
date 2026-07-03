// 세션 관리 use case를 조율하는 애플리케이션 서비스이다.
use crate::application::ports::{ResumePlan, SessionPorts};
use crate::domain::session::{classify_storage_state, is_retryable_auto_summary, StorageState};
use crate::types::{Config, DeleteSessionTarget, Session, SessionMeta, Settings};
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

    pub fn delete_session(&self, session_id: &str, file_path: &str) -> Result<()> {
        self.ports.delete_session(session_id, file_path)?;
        self.ports.delete_session_meta(session_id)
    }

    pub fn delete_session_meta(&self, session_id: &str) -> Result<()> {
        self.ports.delete_session_meta(session_id)
    }

    pub fn delete_sessions(&self, targets: Vec<DeleteSessionTarget>) -> Result<()> {
        for target in targets {
            self.delete_session(&target.session_id, &target.file_path)?;
        }
        Ok(())
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
                session.description.as_deref().unwrap_or("").is_empty()
                    && is_retryable_auto_summary(session.auto_summary.as_deref())
            })
            .take(batch_size)
            .map(|session| (session.session_id, session.file_path))
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

    pub fn generate_summary_pair(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> Result<(String, String)> {
        let cfg = self.ports.load_config();
        let previous_summary = cfg
            .sessions
            .get(session_id)
            .and_then(|meta| meta.description.clone().or(meta.auto_summary.clone()));

        let (name, description) = self
            .ports
            .summarize_session(file_path, previous_summary.as_deref())?;
        self.save_summary(session_id, name.clone(), description.clone())?;
        Ok((name, description))
    }

    pub fn generate_summary(&self, session_id: &str, file_path: &str) -> Result<String> {
        self.generate_summary_pair(session_id, file_path)
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
    use crate::types::{Config, Session, SessionMeta, Settings};
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
        fn delete_session(&self, session_id: &str, file_path: &str) -> Result<()> {
            self.state
                .borrow_mut()
                .deleted_sessions
                .push((session_id.to_string(), file_path.to_string()));
            Ok(())
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
            name: None,
            description: None,
            auto_summary: None,
            project: "Agent".into(),
            project_dir: "C:/Agent".into(),
            file_path: file_path.into(),
            size: 1,
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
            .delete_session("s1", "session.jsonl")
            .unwrap();

        let state = state.borrow();
        assert_eq!(
            state.deleted_sessions,
            vec![("s1".to_string(), "session.jsonl".to_string())]
        );
        assert_eq!(state.deleted_meta, vec!["s1".to_string()]);
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

        let description = SessionService::new(ports)
            .generate_summary("s1", "session.jsonl")
            .unwrap();

        assert_eq!(description, "summary after old");
        let saved = &state.borrow().saved_meta[0];
        assert_eq!(saved.0, "s1");
        assert_eq!(saved.1.name.as_deref(), Some("generated"));
        assert_eq!(saved.1.auto_summary.as_deref(), Some("summary after old"));
    }
}
