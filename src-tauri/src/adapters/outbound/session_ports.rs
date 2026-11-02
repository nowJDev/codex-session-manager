// 기존 인프라 모듈을 애플리케이션 outbound port 구현으로 감싼다.
use crate::application::ports::{
    CloudSyncPort, ResumePlan, ResumePort, SessionCommandPort, SessionMessagePort,
    SessionMetadataPort, SessionScanPort, SummaryPort,
};
use crate::types::{
    Config, DeleteSessionResult, DeleteSessionTarget, DeletionPlan, Session, SessionMeta, Settings,
};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultSessionPorts;

fn cloud_path() -> Option<PathBuf> {
    crate::config::load_config()
        .settings
        .cloud_path
        .map(PathBuf::from)
}

impl SessionScanPort for DefaultSessionPorts {
    fn scan_local_sessions(&self) -> Result<Vec<Session>> {
        super::registered_sessions::list_sessions()
    }
}

impl SessionMessagePort for DefaultSessionPorts {
    fn get_session_messages(&self, file_path: &str, max_messages: usize) -> Result<Vec<String>> {
        crate::scanner::get_session_messages(file_path, max_messages)
    }
}

impl SessionCommandPort for DefaultSessionPorts {
    fn prepare_session_deletion(&self, targets: &[DeleteSessionTarget]) -> Result<DeletionPlan> {
        super::registered_sessions::prepare_deletion(targets)
    }
    fn delete_sessions(
        &self,
        targets: &[DeleteSessionTarget],
        approved_ids: &[String],
    ) -> Result<Vec<DeleteSessionResult>> {
        super::registered_sessions::delete_sessions(targets, approved_ids)
    }

    fn archive_session(&self, session_id: &str) -> Result<()> {
        crate::scanner::archive_session(session_id)
    }

    fn unarchive_session(&self, session_id: &str) -> Result<()> {
        crate::scanner::unarchive_session(session_id)
    }
}

impl SessionMetadataPort for DefaultSessionPorts {
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
}

impl CloudSyncPort for DefaultSessionPorts {
    fn list_cloud_sessions(&self) -> Result<Vec<Session>> {
        crate::cloud::list_cloud_sessions(cloud_path())
    }

    fn set_cloud_folder(&self, root: &str) -> Result<PathBuf> {
        crate::cloud::set_cloud_root(root)
    }

    fn upload_to_cloud(&self, session: &Session) -> Result<()> {
        let cloud = cloud_path().ok_or_else(|| anyhow::anyhow!("cloud not configured"))?;
        crate::cloud::upload_session(&cloud, session)
    }

    fn checkout_session(&self, session: &Session) -> Result<String> {
        let cloud = cloud_path().ok_or_else(|| anyhow::anyhow!("cloud not configured"))?;
        let mut api = super::codex_app_server::AppServer::connect()?;
        let before = api.list_threads()?;
        if before.iter().any(|t| t.id == session.session_id) {
            anyhow::bail!("이미 로컬에 등록된 세션입니다. 새로고침 후 재개해 주세요.");
        }
        let path = crate::cloud::checkout(&cloud, session)?;
        let verify = (|| -> Result<()> {
            api.request("thread/list", serde_json::json!({
                "useStateDbOnly":false, "limit":1, "modelProviders":[],
                "sourceKinds":["cli","vscode","exec","appServer","subAgent","subAgentReview","subAgentCompact","subAgentThreadSpawn","subAgentOther","unknown"]
            }))?;
            if !api
                .list_threads()?
                .iter()
                .any(|t| t.id == session.session_id)
            {
                anyhow::bail!("파일 복사는 완료했지만 Codex 세션 등록을 확인하지 못했습니다.");
            }
            Ok(())
        })();
        if let Err(error) = verify {
            let _ = crate::cloud::release_lock(&cloud, &session.session_id);
            return Err(error);
        }
        Ok(path)
    }

    fn checkin_session(&self, session: &Session) -> Result<()> {
        crate::cloud::checkin(cloud_path(), session)
    }
}

impl ResumePort for DefaultSessionPorts {
    fn resume_session(&self, session_id: &str, cwd: Option<&str>) -> Result<()> {
        crate::resume::resume_in_new_terminal(session_id, cwd)
    }

    fn build_resume_plan(&self, session_id: &str, cwd: Option<&str>) -> Result<ResumePlan> {
        let target_os = crate::resume::current_target_os();
        let plan = crate::resume::build_resume_plan(session_id, cwd, target_os);
        Ok(ResumePlan {
            program: plan.program,
            args: plan.args,
        })
    }
}

impl SummaryPort for DefaultSessionPorts {
    fn summarize_batch(
        &self,
        items: &[(String, String)],
    ) -> Result<HashMap<String, (String, String)>> {
        crate::summary::auto_summarize_batch(items)
    }

    fn summarize_session(
        &self,
        session_id: &str,
        previous_summary: Option<&str>,
    ) -> Result<(String, String)> {
        let mut api = super::codex_app_server::AppServer::connect()?;
        let value = api.request(
            "thread/read",
            serde_json::json!({"threadId":session_id,"includeTurns":false}),
        )?;
        let thread: super::codex_app_server::RegisteredThread = serde_json::from_value(
            value
                .get("thread")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("세션 응답에 thread가 없습니다."))?,
        )?;
        if thread.history_mode == "legacy" {
            if let Some(path) = thread
                .path
                .as_deref()
                .filter(|p| std::path::Path::new(p).is_file())
            {
                return crate::summary::auto_summarize_session(path, previous_summary);
            }
        }
        let excerpt = api.read_excerpt(&thread.id)?;
        crate::summary::summarize_excerpt(&excerpt, previous_summary)
    }
}
