use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionMeta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub auto_summary: Option<String>,
    #[serde(default)]
    pub storage_type: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub favorite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub cloud_path: Option<String>,
    #[serde(default)]
    pub preferred_terminal: Option<String>,
    #[serde(default)]
    pub resume_flags: Option<String>,
    #[serde(default)]
    pub custom_terminal_program: Option<String>,
    #[serde(default)]
    pub custom_terminal_args: Option<String>,
    #[serde(default)]
    pub extra_project_dirs: Option<Vec<String>>,
    #[serde(default)]
    pub wsl_auto_detect: Option<bool>,
    /// 스캔에서 제외할 경로/폴더명 substring 목록.
    /// 세션의 file_path 또는 project_dir에 substring으로 포함되면 스캔 결과에서 제외.
    /// 예: "currency-edge" → C--Git-currency-edge / D--Code-currency-edge 둘 다 매치 (PC 무관).
    /// 절대 경로(C:\Git\currency-edge)도 OK — file_path가 매치됨.
    #[serde(default)]
    pub excluded_scan_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub sessions: HashMap<String, SessionMeta>,
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSessionTarget {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub guardian_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionPlan {
    pub targets: Vec<DeleteSessionTarget>,
    pub affected_ids: Vec<String>,
    #[serde(default)]
    pub skipped_guardian_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "phase",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DeleteProgress {
    CheckingScope,
    Deleting {
        processed: usize,
        total: usize,
        elapsed_ms: u64,
    },
    Verifying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteSessionStatus {
    Deleted,
    AlreadyMissing,
    Failed,
    Unconfirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSessionResult {
    pub session_id: String,
    pub status: DeleteSessionStatus,
    pub error: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub is_subagent: bool,
    #[serde(default)]
    pub agent_nickname: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub auto_summary: Option<String>,
    pub project: String,
    pub project_dir: String,
    pub file_path: Option<String>,
    pub size: Option<u64>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub history_mode: String,
    #[serde(default)]
    pub registered: bool,
    #[serde(default)]
    pub session_kind: SessionKind,
    #[serde(default)]
    pub is_summary_run: bool,
    pub total_lines: usize,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub cwd: Option<String>,
    pub version: Option<String>,
    pub first_user_message: Option<String>,
    pub storage_type: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SessionKind {
    Main,
    Worker,
    Guardian,
    OtherInternal,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub sessions: Vec<Session>,
    pub complete: bool,
    pub missing_count: usize,
    pub errors: Vec<String>,
}

impl SessionKind {
    pub(crate) fn fallback_name(self, id: &str) -> String {
        let label = match self {
            Self::Main | Self::Unknown => "세션",
            Self::Worker => "작업자",
            Self::Guardian => "Guardian",
            Self::OtherInternal => "내부 기록",
        };
        format!("{label} · {}", id.chars().take(8).collect::<String>())
    }
    pub(crate) fn from_source(source: &serde_json::Value) -> Self {
        use crate::types::SessionKind;
        if let Some(sub) = source.get("subAgent").or_else(|| source.get("subagent")) {
            if sub.get("other").and_then(serde_json::Value::as_str) == Some("guardian") {
                SessionKind::Guardian
            } else if sub.get("thread_spawn").is_some() {
                SessionKind::Worker
            } else {
                SessionKind::OtherInternal
            }
        } else if matches!(
            source.as_str(),
            Some("cli" | "vscode" | "exec" | "appServer" | "app_server")
        ) {
            SessionKind::Main
        } else {
            SessionKind::Unknown
        }
    }
}
