use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// 디버그 로그 파일 경로.
/// 사용자가 문제 신고할 때 첨부할 수 있게 일관된 위치에 보관.
pub fn log_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".claude-sessions").join("debug.log")
}

/// 한 줄 append (절대 panic 안 함, IO 실패는 무시)
pub fn log(category: &str, message: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = chrono::Utc::now().to_rfc3339();
    let line = format!("[{}] [{}] {}\n", ts, category, message);
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
}
