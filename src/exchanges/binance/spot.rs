mod difference;
mod info;
mod pairs;
mod snapshot;

use crate::{Book, HashMapChunks, LatencyMeter, Order, Pair, SystemConfig, TokenBucket};
use backon::Retryable;
pub use pairs::get_pairs;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize)]
struct Update(Decimal, Decimal);

impl From<Update> for Order {
    fn from(value: Update) -> Self {
        Order {
            price: value.0,
            size: value.1,
        }
    }
}

async fn loop_connection(
    id: usize,
    config: SystemConfig,
    books: HashMap<Pair, Arc<Mutex<Book>>>,
    r_tb: Arc<TokenBucket>,
    w_tb: Arc<TokenBucket>,
    lat_tx: mpsc::UnboundedSender<Duration>,
    _lat_meter: Arc<LatencyMeter>,
) {
    loop {
        match difference::run_connection(&config, &books, &r_tb, &w_tb, &lat_tx).await {
            Ok(()) => log::info!("{} connection {id}: restarting", config.log_prefix),
            Err(err) => log::error!("{} connection {id}: {err:?}", config.log_prefix),
        };
        tokio::time::sleep(config.reconnect_delay).await;
    }
}

pub(crate) async fn spawn(config: SystemConfig, books: HashMap<Pair, Arc<Mutex<Book>>>) {
    let (r_tb, w_tb) = info::get_rate_limits_tbs
        .retry(backon::ExponentialBuilder::default())
        .await.unwrap();

    let (lat_tx, lat_rx) = mpsc::unbounded_channel();
    let lat_meter = Arc::new(LatencyMeter::new(
        config.log_prefix.clone(), config.latency_check_interval, lat_rx,
    ));

    for (idx, books) in HashMapChunks::new(books, config.streams_per_connection).enumerate() {
        tokio::spawn(loop_connection(
            idx + 1, config.clone(), books, Arc::clone(&r_tb), Arc::clone(&w_tb), lat_tx.clone(),
            Arc::clone(&lat_meter),
        ));
    }
}
