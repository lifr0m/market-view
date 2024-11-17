mod difference;
mod info;
pub mod pairs;
mod snapshot;

use crate::{Book, HashMapChunks, LatencyMeter, Order, Pair, TokenBucket};
use backon::Retryable;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

pub const BOOK_SIZE: usize = 100;

/// https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#websocket-limits
const STREAMS_PER_CONNECTION: usize = 128;
/// https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#websocket-limits
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

const LOG_PREFIX: &str = "[binance] [spot]";

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
    books: HashMap<Pair, Arc<Mutex<Book>>>,
    r_tb: Arc<TokenBucket>,
    w_tb: Arc<TokenBucket>,
    lat_tx: mpsc::UnboundedSender<Duration>,
    _lat_meter: Arc<LatencyMeter>,
) {
    loop {
        match difference::run_connection(&books, &r_tb, &w_tb, &lat_tx).await {
            Ok(()) => println!("{LOG_PREFIX}: restarting"),
            Err(err) => eprintln!("{LOG_PREFIX}: {err:?}"),
        };
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

pub async fn spawn(books: HashMap<Pair, Arc<Mutex<Book>>>) {
    let (r_tb, w_tb) = info::get_rate_limits_tbs
        .retry(backon::ExponentialBuilder::default())
        .await.unwrap();

    let (lat_tx, lat_rx) = mpsc::unbounded_channel();
    let lat_meter = Arc::new(LatencyMeter::new(
        String::from(LOG_PREFIX), difference::LATENCY_CHECK_INTERVAL, lat_rx,
    ));

    for books in HashMapChunks::new(books, STREAMS_PER_CONNECTION) {
        tokio::spawn(loop_connection(
            books, Arc::clone(&r_tb), Arc::clone(&w_tb), lat_tx.clone(), Arc::clone(&lat_meter),
        ));
    }
}
