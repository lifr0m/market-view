use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct LatencyMeter {
    recv_jh: tokio::task::JoinHandle<()>,
    check_jh: tokio::task::JoinHandle<()>,
}

impl LatencyMeter {
    pub fn new(prefix: String, interval: Duration, mut rx: mpsc::UnboundedReceiver<Duration>) -> Self {
        let vec = Arc::new(Mutex::new(Vec::new()));
        
        let recv_jh = tokio::spawn({
            let vec = Arc::clone(&vec);
            
            async move {
                while let Some(latency) = rx.recv().await {
                    vec.lock().unwrap().push(latency);
                }
            }
        });
        
        let check_jh = tokio::spawn({
            let vec = Arc::clone(&vec);
            let mut interval = tokio::time::interval(interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            
            async move {
                loop {
                    interval.tick().await;

                    {
                        let mut vec = vec.lock().unwrap();
                        
                        if !vec.is_empty() {
                            let mean = vec.iter().sum::<Duration>() / vec.len() as u32;
                            eprintln!("{prefix}: high latency - {mean:?}");

                            vec.clear();
                        }
                    }
                }
            }
        });
        
        Self { recv_jh, check_jh }
    }
}

impl Drop for LatencyMeter {
    fn drop(&mut self) {
        self.recv_jh.abort();
        self.check_jh.abort();
    }
}
