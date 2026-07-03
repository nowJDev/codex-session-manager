// 릴리즈 버전 비교 규칙을 제공한다.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

fn parse_version(version: &str) -> [u64; 3] {
    let clean = version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V');
    let mut parts = [0_u64; 3];
    for (idx, piece) in clean.split('.').take(3).enumerate() {
        let digits = piece
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        parts[idx] = digits.parse().unwrap_or(0);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::is_newer_version;

    #[test]
    fn version_comparison_handles_v_prefixed_semver() {
        assert!(is_newer_version("0.5.1", "v0.5.2"));
        assert!(is_newer_version("0.5.1", "0.6.0"));
        assert!(!is_newer_version("0.5.2", "v0.5.2"));
        assert!(!is_newer_version("0.5.2", "v0.5.1"));
    }
}
