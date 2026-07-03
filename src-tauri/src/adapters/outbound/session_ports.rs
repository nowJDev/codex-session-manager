// 기존 인프라 모듈을 애플리케이션 outbound port 구현으로 감싼다.
use crate::application::ports::SessionPorts;
use crate::types::{Config, Session, SessionMeta, Settings};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultSessionPorts;

impl SessionPorts for DefaultSessionPorts {
    fn scan_local_sessions(&self) -> Result<Vec<Session>> {
        crate::scanner::scan_local_sessions()
    }

    fn delete_session(&self, session_id: &str, file_path: &str) -> Result<()> {
        crate::scanner::delete_session(session_id, file_path)
    }

    fn archive_session(&self, session_id: &str) -> Result<()> {
        crate::scanner::archive_session(session_id)
    }

    fn unarchive_session(&self, session_id: &str) -> Result<()> {
        crate::scanner::unarchive_session(session_id)
    }

    fn load_config(&self) -> Config {
        crate::config::load_config()
    }

    fn save_session_meta(&self, session_id: &str, patch: SessionMeta) -> Result<()> {
        crate::config::upsert_session_meta(session_id, patch)
    }

    fn delete_session_meta(&self, session_id: &str) -> Result<()> {
        crate::config::delete_session_meta(session_id)
    }

    fn update_settings(&self, patch: Settings) -> Result<()> {
        crate::config::update_settings(patch)
    }

    fn list_cloud_sessions(&self) -> Result<Vec<Session>> {
        crate::cloud::list_cloud_sessions()
    }

    fn set_cloud_folder(&self, root: &str) -> Result<PathBuf> {
        crate::cloud::set_cloud_root(root)
    }

    fn upload_to_cloud(&self, session: &Session) -> Result<()> {
        crate::cloud::upload_session(session)
    }

    fn checkout_session(&self, session: &Session) -> Result<String> {
        crate::cloud::checkout(session)
    }

    fn checkin_session(&self, session: &Session) -> Result<()> {
        crate::cloud::checkin(session)
    }

    fn resume_session(&self, session_id: &str, cwd: Option<&str>) -> Result<()> {
        crate::resume::resume_in_new_terminal(session_id, cwd)
    }

    fn summarize_batch(
        &self,
        items: &[(String, String)],
    ) -> Result<HashMap<String, (String, String)>> {
        crate::summary::auto_summarize_batch(items)
    }

    fn summarize_session(
        &self,
        file_path: &str,
        previous_summary: Option<&str>,
    ) -> Result<(String, String)> {
        crate::summary::auto_summarize_session(file_path, previous_summary)
    }
}
