// 애플리케이션 use case가 외부 시스템에 기대는 기능 계약이다.
use crate::types::{Config, Session, SessionMeta, Settings};
use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub trait SessionPorts {
    fn scan_local_sessions(&self) -> Result<Vec<Session>>;
    fn delete_session(&self, session_id: &str, file_path: &str) -> Result<()>;
    fn archive_session(&self, session_id: &str) -> Result<()>;
    fn unarchive_session(&self, session_id: &str) -> Result<()>;

    fn load_config(&self) -> Config;
    fn save_session_meta(&self, session_id: &str, patch: SessionMeta) -> Result<()>;
    fn delete_session_meta(&self, session_id: &str) -> Result<()>;
    fn update_settings(&self, patch: Settings) -> Result<()>;

    fn list_cloud_sessions(&self) -> Result<Vec<Session>>;
    fn set_cloud_folder(&self, root: &str) -> Result<PathBuf>;
    fn upload_to_cloud(&self, session: &Session) -> Result<()>;
    fn checkout_session(&self, session: &Session) -> Result<String>;
    fn checkin_session(&self, session: &Session) -> Result<()>;

    fn resume_session(&self, session_id: &str, cwd: Option<&str>) -> Result<()>;
    fn summarize_batch(
        &self,
        items: &[(String, String)],
    ) -> Result<HashMap<String, (String, String)>>;
    fn summarize_session(
        &self,
        file_path: &str,
        previous_summary: Option<&str>,
    ) -> Result<(String, String)>;
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugLogInfo {
    pub path: String,
    pub exists: bool,
    pub size: u64,
    pub tail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveDetectResult {
    pub found: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedTerminalInfo {
    pub kind: String,
    pub program: String,
    pub display_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub target_os: String,
    pub codex_cli_found: bool,
    pub codex_cli_path: Option<String>,
    pub codex_cli_version: Option<String>,
    pub terminals: Vec<DetectedTerminalInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLimitStatus {
    pub key: String,
    pub label: String,
    pub available: bool,
    pub value: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus {
    pub checked_at: String,
    pub cli_found: bool,
    pub cli_path: Option<String>,
    pub cli_version: Option<String>,
    pub model: Option<String>,
    pub model_reasoning_effort: Option<String>,
    pub status_line: Vec<String>,
    pub limits: Vec<CodexLimitStatus>,
    pub note: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_url: String,
}

pub trait SystemPorts {
    fn debug_log_info(&self) -> DebugLogInfo;
    fn open_debug_log_folder(&self) -> Result<()>;
    fn detect_google_drive(&self) -> DriveDetectResult;
    fn connect_google_drive(&self) -> Result<String>;
    fn check_environment(&self) -> EnvironmentReport;
    fn get_codex_status(&self) -> CodexStatus;
    fn check_update(&self) -> Pin<Box<dyn Future<Output = Result<UpdateInfo>> + Send + '_>>;
}
