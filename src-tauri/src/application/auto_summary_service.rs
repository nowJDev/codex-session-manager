// 자동 요약 백그라운드 작업을 애플리케이션 use case로 실행한다.
use crate::application::ports::SessionPorts;
use crate::application::session_service::SessionService;
use std::sync::atomic::{AtomicBool, Ordering};

static AUTO_SUMMARY_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start_auto_summary<P, F>(ports: P, notify_progress: F) -> bool
where
    P: SessionPorts + Copy + Send + 'static,
    F: Fn(&str) + Send + 'static,
{
    if AUTO_SUMMARY_RUNNING.swap(true, Ordering::SeqCst) {
        return false;
    }
    const BATCH_SIZE: usize = 5;
    std::thread::spawn(move || {
        loop {
            let service = SessionService::new(ports);
            let pending = match service.pending_auto_summary_batch(BATCH_SIZE) {
                Ok(sessions) => sessions,
                Err(_) => break,
            };
            if pending.is_empty() {
                break;
            }

            match service.summarize_batch(&pending) {
                Ok(result) => {
                    for (id, _path) in &pending {
                        if let Some((name, desc)) = result.get(id) {
                            let _ = service.save_summary(id, name.clone(), desc.clone());
                            notify_progress(id);
                        } else {
                            let _ = service.mark_summary_missing(id);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[auto-summary batch] 실패: {}", e);
                    if let Some((id, _)) = pending.first() {
                        let _ = service.mark_summary_failed(id, &e.to_string());
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        AUTO_SUMMARY_RUNNING.store(false, Ordering::SeqCst);
    });
    true
}
