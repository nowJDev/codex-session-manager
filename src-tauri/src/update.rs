// GitHub 릴리즈를 조회해 앱 업데이트 가능 여부를 판단한다.
use anyhow::Result;
use serde::{Deserialize, Serialize};

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/nowJDev/codex-session-manager/releases/latest";
const RELEASES_URL: &str = "https://github.com/nowJDev/codex-session-manager/releases";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    crate::domain::update::is_newer_version(current, latest)
}

pub async fn check_latest_release() -> Result<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let release = reqwest::Client::new()
        .get(LATEST_RELEASE_API)
        .header(reqwest::header::USER_AGENT, "codex-session-manager")
        .send()
        .await?
        .error_for_status()?
        .json::<GitHubRelease>()
        .await?;
    let has_update = is_newer_version(&current, &release.tag_name);

    Ok(UpdateInfo {
        current_version: current,
        latest_version: release.tag_name,
        has_update,
        release_url: release.html_url,
    })
}

pub fn releases_url() -> &'static str {
    RELEASES_URL
}
