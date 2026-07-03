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

pub fn is_internal_summary_run(cwd: Option<&str>) -> bool {
    cwd.is_some_and(|c| c.contains(".summary-runs") || c.contains("summary-runs"))
}

pub fn encode_scan_match_text(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

pub fn scan_exclusion_matches(
    file_path: &str,
    cwd: Option<&str>,
    excluded_raw: &[String],
    excluded_encoded: &[String],
) -> bool {
    excluded_raw
        .iter()
        .any(|p| file_path.contains(p) || cwd.is_some_and(|c| c.contains(p)))
        || excluded_encoded
            .iter()
            .any(|p| file_path.contains(p) || cwd.is_some_and(|c| c.contains(p)))
}

pub fn should_include_scanned_session(
    name: Option<&str>,
    description: Option<&str>,
    auto_summary: Option<&str>,
    first_user_message: Option<&str>,
) -> bool {
    let has_saved_display = [name, description, auto_summary]
        .into_iter()
        .flatten()
        .any(|s| !s.trim().is_empty());
    let has_real_user_message = first_user_message.is_some_and(|s| !s.trim().is_empty());
    has_saved_display || has_real_user_message
}

#[cfg(test)]
mod tests {
    use super::{
        classify_storage_state, encode_scan_match_text, is_internal_summary_run,
        is_retryable_auto_summary, scan_exclusion_matches, should_include_scanned_session,
        StorageState,
    };

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

    #[test]
    fn scanner_policy_detects_internal_summary_runs() {
        assert!(is_internal_summary_run(Some(
            "C:/Users/me/.codex-sessions/.summary-runs"
        )));
        assert!(is_internal_summary_run(Some("C:/tmp/summary-runs/batch")));
        assert!(!is_internal_summary_run(Some("C:/Git/product")));
        assert!(!is_internal_summary_run(None));
    }

    #[test]
    fn scanner_policy_matches_raw_and_encoded_exclusions() {
        let raw = vec!["currency-edge".to_string(), "C:\\Git\\other".to_string()];
        let encoded: Vec<String> = raw.iter().map(|p| encode_scan_match_text(p)).collect();

        assert!(scan_exclusion_matches(
            "C:/Users/me/.codex/sessions/C--Git-currency-edge/rollout.jsonl",
            Some("C:/Git/currency-edge"),
            &raw,
            &encoded,
        ));
        assert!(scan_exclusion_matches(
            "C:/Users/me/.codex/sessions/C--Git-other/rollout.jsonl",
            None,
            &raw,
            &encoded,
        ));
        assert!(!scan_exclusion_matches(
            "C:/Users/me/.codex/sessions/C--Git-keep/rollout.jsonl",
            Some("C:/Git/keep"),
            &raw,
            &encoded,
        ));
    }

    #[test]
    fn scanner_policy_requires_saved_display_or_real_user_message() {
        assert!(should_include_scanned_session(
            Some("name"),
            None,
            None,
            None
        ));
        assert!(should_include_scanned_session(
            None,
            Some("desc"),
            None,
            None
        ));
        assert!(should_include_scanned_session(
            None,
            None,
            Some("summary"),
            None
        ));
        assert!(should_include_scanned_session(
            None,
            None,
            None,
            Some("real question")
        ));
        assert!(!should_include_scanned_session(None, None, None, None));
        assert!(!should_include_scanned_session(
            Some(" "),
            None,
            None,
            Some(" ")
        ));
    }
}
