// 자동 요약 백그라운드 실행을 애플리케이션 use case에 연결한다.
use crate::application::auto_summary_service::{
    process_auto_summary_batch, AutoSummaryBatchResult, AUTO_SUMMARY_BATCH_SIZE,
};
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

    std::thread::spawn(move || {
        loop {
            let service = SessionService::new(ports);
            match process_auto_summary_batch(&service, AUTO_SUMMARY_BATCH_SIZE) {
                AutoSummaryBatchResult::Idle => break,
                AutoSummaryBatchResult::Processed(ids) => {
                    for id in ids {
                        notify_progress(&id);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        AUTO_SUMMARY_RUNNING.store(false, Ordering::SeqCst);
    });
    true
}
