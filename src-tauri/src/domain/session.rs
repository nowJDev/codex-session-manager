// 세션 표시와 동기화 상태에 관한 순수 규칙을 제공한다.
use crate::types::Session;
use std::collections::HashSet;

pub fn is_retryable_auto_summary(value: Option<&str>) -> bool {
    let text = value.unwrap_or("").trim();
    text.is_empty()
        || text.starts_with('(')
        || text.contains("자동 요약 실패")
        || text.contains("요약 누락")
        || text.contains("codex CLI")
        || text.contains("batch file arguments are invalid")
}

pub fn merge_local_and_cloud_sessions(
    mut local: Vec<Session>,
    cloud_all: Vec<Session>,
) -> Vec<Session> {
    let cloud_ids: HashSet<String> = cloud_all.iter().map(|c| c.session_id.clone()).collect();
    for session in local.iter_mut() {
        if cloud_ids.contains(&session.session_id) {
            session.storage_type = "synced".into();
        } else {
            session.storage_type = "local-only".into();
        }
    }

    let local_ids: HashSet<String> = local.iter().map(|s| s.session_id.clone()).collect();
    let cloud_only = cloud_all
        .into_iter()
        .filter(|session| !local_ids.contains(&session.session_id))
        .map(|mut session| {
            session.storage_type = "cloud-only".into();
            session
        });
    local.extend(cloud_only);
    local
}

#[cfg(test)]
mod tests {
    use super::is_retryable_auto_summary;

    #[test]
    fn auto_summary_failure_markers_are_retryable() {
        assert!(is_retryable_auto_summary(None));
        assert!(is_retryable_auto_summary(Some("")));
        assert!(is_retryable_auto_summary(Some(
            "(자동 요약 실패: codex CLI 실행 실패)"
        )));
        assert!(is_retryable_auto_summary(Some("(요약 누락 - 재시도 예정)")));
        assert!(is_retryable_auto_summary(Some(
            "(auto summary failed: batch file arguments are invalid)"
        )));
        assert!(!is_retryable_auto_summary(Some(
            "세션 매니저 릴리즈를 점검했다."
        )));
    }
}
