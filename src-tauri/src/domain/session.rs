// 세션 표시와 동기화 상태에 관한 순수 규칙을 제공한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageState {
    Synced,
    LocalOnly,
    CloudOnly,
}

pub fn is_retryable_auto_summary(value: Option<&str>) -> bool {
    let text = value.unwrap_or("").trim();
    text.is_empty()
        || text.starts_with('(')
        || text.contains("자동 요약 실패")
        || text.contains("요약 누락")
        || text.contains("codex CLI")
        || text.contains("batch file arguments are invalid")
}

pub fn classify_storage_state(local_exists: bool, cloud_exists: bool) -> Option<StorageState> {
    match (local_exists, cloud_exists) {
        (true, true) => Some(StorageState::Synced),
        (true, false) => Some(StorageState::LocalOnly),
        (false, true) => Some(StorageState::CloudOnly),
        (false, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_storage_state, is_retryable_auto_summary, StorageState};

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

    #[test]
    fn storage_state_is_classified_without_session_dto() {
        assert_eq!(
            classify_storage_state(true, true),
            Some(StorageState::Synced)
        );
        assert_eq!(
            classify_storage_state(true, false),
            Some(StorageState::LocalOnly)
        );
        assert_eq!(
            classify_storage_state(false, true),
            Some(StorageState::CloudOnly)
        );
        assert_eq!(classify_storage_state(false, false), None);
    }
}
