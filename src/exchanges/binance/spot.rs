mod difference;
mod info;
pub mod pairs;
mod snapshot;

use crate::{Book, HashMapChunks, Order, Pair, TokenBucket};
use backon::Retryable;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const BOOK_SIZE: usize = 100;

/// https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#websocket-limits
const STREAMS_PER_CONNECTION: usize = 128;
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

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

async fn run_connection(
    books: HashMap<Pair, Arc<Mutex<Book>>>,
    r_tb: Arc<TokenBucket>,
    w_tb: Arc<TokenBucket>,
) {
    loop {
        match difference::run_connection(&books, &r_tb, &w_tb).await {
            Ok(()) => println!("[binance] [spot]: restarting"),
            Err(err) => eprintln!("[binance] [spot]: {err:?}"),
        };
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

pub async fn run(books: HashMap<Pair, Arc<Mutex<Book>>>) {
    let (r_tb, w_tb) = info::get_rate_limits_tbs
        .retry(backon::ExponentialBuilder::default())
        .await.unwrap();

    for books in HashMapChunks::new(books, STREAMS_PER_CONNECTION) {
        tokio::spawn(run_connection(books, Arc::clone(&r_tb), Arc::clone(&w_tb)));
    }
}
