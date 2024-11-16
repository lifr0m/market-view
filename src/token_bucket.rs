use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub struct TokenBucket {
    sem: Arc<Semaphore>,
    jh: tokio::task::JoinHandle<()>,
}

impl TokenBucket {
    pub fn new(cap: usize, rate: Duration) -> Self {
        let sem = Arc::new(Semaphore::new(cap));

        let jh = tokio::spawn({
            let sem = Arc::clone(&sem);
            let mut interval = tokio::time::interval(rate);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            async move {
                loop {
                    interval.tick().await;

                    if sem.available_permits() < cap {
                        sem.add_permits(1);
                    }
                }
            }
        });

        Self { sem, jh }
    }

    pub async fn acquire(&self, n: u32) {
        let permit = self.sem.acquire_many(n).await.unwrap();
        permit.forget();
    }
}

impl Drop for TokenBucket {
    fn drop(&mut self) {
        self.jh.abort();
    }
}
