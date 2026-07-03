// 시스템 진단, 업데이트, 디버그 로그 use case를 조율한다.
use crate::application::ports::{
    CodexStatus, DebugLogInfo, DriveDetectResult, EnvironmentReport, PathsInfo, SystemPorts,
    UpdateInfo,
};
use anyhow::Result;

pub struct SystemService<P> {
    ports: P,
}

impl<P: SystemPorts> SystemService<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub fn debug_log_info(&self) -> DebugLogInfo {
        self.ports.debug_log_info()
    }

    pub fn open_debug_log_folder(&self) -> Result<()> {
        self.ports.open_debug_log_folder()
    }

    pub fn detect_google_drive(&self) -> DriveDetectResult {
        self.ports.detect_google_drive()
    }

    pub fn connect_google_drive(&self) -> Result<String> {
        self.ports.connect_google_drive()
    }

    pub fn check_environment(&self) -> EnvironmentReport {
        self.ports.check_environment()
    }

    pub fn get_codex_status(&self) -> CodexStatus {
        self.ports.get_codex_status()
    }

    pub async fn check_update(&self) -> Result<UpdateInfo> {
        self.ports.check_update().await
    }

    pub fn paths_info(&self) -> PathsInfo {
        self.ports.paths_info()
    }
}
