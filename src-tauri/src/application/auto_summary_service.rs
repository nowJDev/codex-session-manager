// 자동 요약 배치 처리 use case를 제공한다.
use crate::application::ports::SessionPorts;
use crate::application::session_service::SessionService;

pub const AUTO_SUMMARY_BATCH_SIZE: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoSummaryBatchResult {
    Idle,
    Processed(Vec<String>),
}

pub fn process_auto_summary_batch<P>(
    service: &SessionService<P>,
    batch_size: usize,
) -> AutoSummaryBatchResult
where
    P: SessionPorts,
{
    let pending = match service.pending_auto_summary_batch(batch_size) {
        Ok(sessions) => sessions,
        Err(_) => return AutoSummaryBatchResult::Idle,
    };
    if pending.is_empty() {
        return AutoSummaryBatchResult::Idle;
    }

    let mut updated = Vec::new();
    match service.summarize_batch(&pending) {
        Ok(result) => {
            for (id, _path) in &pending {
                if let Some((name, desc)) = result.get(id) {
                    let _ = service.save_summary(id, name.clone(), desc.clone());
                    updated.push(id.clone());
                } else {
                    let _ = service.mark_summary_missing(id);
                }
            }
        }
        Err(e) => {
            if let Some((id, _)) = pending.first() {
                let _ = service.mark_summary_failed(id, &e.to_string());
            }
        }
    }
    AutoSummaryBatchResult::Processed(updated)
}
