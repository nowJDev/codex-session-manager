// 전체 세션 스냅샷 조립 단계가 공유하는 시간 예산을 관리한다.
use anyhow::Result;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct RefreshBudget {
    deadline: Instant,
    now: fn() -> Instant,
}
impl RefreshBudget {
    pub fn new(now: fn() -> Instant) -> Self {
        Self {
            deadline: now() + Duration::from_secs(60),
            now,
        }
    }
    pub fn deadline(self) -> Instant {
        self.deadline
    }
    pub fn check(self) -> Result<()> {
        if (self.now)() >= self.deadline {
            anyhow::bail!("전체 세션 조회 60초 예산 초과");
        }
        Ok(())
    }
    pub fn record_timeout(self, snapshot: &mut crate::types::SessionSnapshot) -> bool {
        if let Err(error) = self.check() {
            snapshot.complete = false;
            let message = error.to_string();
            if !snapshot.errors.contains(&message) {
                snapshot.errors.push(message);
            }
            false
        } else {
            true
        }
    }
}
