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
    pub anthropic_api_key: Option<String>,
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
pub struct Session {
    pub session_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub auto_summary: Option<String>,
    pub project: String,
    pub project_dir: String,
    pub file_path: String,
    pub size: u64,
    pub total_lines: usize,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub cwd: Option<String>,
    pub version: Option<String>,
    pub first_user_message: Option<String>,
    pub storage_type: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_by: Option<String>,
}
