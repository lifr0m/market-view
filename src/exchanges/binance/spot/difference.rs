use super::{snapshot::get_snapshot, Update};
use crate::{Book, Order, Pair, TokenBucket};
use backon::Retryable;
use futures::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::mpsc;

const MAX_LATENCY: Duration = Duration::from_secs(5);
const MAX_LATENCY_ERROR: Duration = Duration::from_millis(100);

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct Event {
    data: EventPayload,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct EventPayload {
    E: u64,
    s: String,
    U: u64,
    u: u64,
    b: Vec<Update>,
    a: Vec<Update>,
}

fn process_event(
    event: EventPayload,
    book: &Arc<Mutex<Book>>,
) {
    let mut book = book.lock().unwrap();

    for update in event.b {
        book.bids.diff_update(Order::from(update))
    }
    for update in event.a {
        book.asks.diff_update(Order::from(update))
    }
}

fn ensure_latency(pair: &Pair, event: &EventPayload) {
    let event_time = UNIX_EPOCH + Duration::from_millis(event.E);
    match event_time.elapsed() {
        Ok(latency) => if latency > MAX_LATENCY {
            eprintln!("[binance] [spot] [{pair}]: high latency - {latency:?}");
        }
        Err(err) => if err.duration() > MAX_LATENCY_ERROR {
            eprintln!("[binance] [spot] [{pair}]: latency error - {:?}", err.duration());
        }
    }
}

/// https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#how-to-manage-a-local-order-book-correctly
async fn run_pair(
    pair: Pair,
    book: Arc<Mutex<Book>>,
    mut rx: mpsc::UnboundedReceiver<EventPayload>,
    r_tb: Arc<TokenBucket>,
    w_tb: Arc<TokenBucket>,
) {
    while rx.is_empty() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    'from_snapshot: loop {
        let snapshot = (|| get_snapshot(&pair, &r_tb, &w_tb))
            .retry(backon::ExponentialBuilder::default())
            .await.unwrap();

        {
            let mut book = book.lock().unwrap();

            let orders = snapshot.bids
                .into_iter()
                .map(Order::from)
                .collect();
            book.bids.shot_update(orders);

            let orders = snapshot.asks
                .into_iter()
                .map(Order::from)
                .collect();
            book.asks.shot_update(orders);
        }

        let mut prev_u: u64;

        loop {
            let event = rx.recv().await.unwrap();

            if event.u <= snapshot.lastUpdateId {
                continue;
            }
            if !(event.U <= snapshot.lastUpdateId + 1 && event.u > snapshot.lastUpdateId) {
                eprintln!(
                    "[binance] [spot] [{pair}]: !(U ({}) <= lastUpdateId ({}) + 1 && \
                    u ({}) >= lastUpdateId ({}) + 1)",
                    event.U, snapshot.lastUpdateId, event.u, snapshot.lastUpdateId
                );
                continue 'from_snapshot;
            }
            prev_u = event.u;

            ensure_latency(&pair, &event);
            process_event(event, &book);
            break;
        }

        loop {
            match rx.recv().await {
                Some(event) => {
                    if event.U != prev_u + 1 {
                        eprintln!("[binance] [spot] [{pair}]: U ({}) != prev_u ({prev_u}) + 1",
                                  event.U);
                        continue 'from_snapshot;
                    }
                    prev_u = event.u;

                    ensure_latency(&pair, &event);
                    process_event(event, &book);
                }
                None => break 'from_snapshot
            }
        }
    }
}

/// https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams
pub async fn run_connection(
    books: &HashMap<Pair, Arc<Mutex<Book>>>,
    r_tb: &Arc<TokenBucket>,
    w_tb: &Arc<TokenBucket>,
) -> Result<(), tokio_websockets::Error> {
    let uri = http::Uri::from_str(&format!(
        "wss://data-stream.binance.vision/stream?streams={}",
        books.keys()
            .map(|p| format!("{}@depth@100ms", p.fused()))
            .collect::<Vec<String>>()
            .join("/")
    )).unwrap();
    let (mut client, _) =
        tokio_websockets::ClientBuilder::from_uri(uri).connect().await?;

    let mut tasks = tokio::task::JoinSet::new();
    let txs = HashMap::<String, mpsc::UnboundedSender<EventPayload>>::from_iter(
        books.iter().map(|(p, b)| (
            p.fused_upper(),
            {
                let (tx, rx) = mpsc::unbounded_channel();
                tasks.spawn(run_pair(p.clone(), Arc::clone(b), rx, Arc::clone(r_tb), Arc::clone(w_tb)));
                tx
            }
        ))
    );
    while let Some(msg) = client.next().await {
        let msg = msg?;
        let body = msg.as_payload();

        if !msg.is_ping() {
            let event = serde_json::from_slice::<Event>(body).unwrap().data;
            txs[&event.s].send(event).unwrap();
        }
    }

    Ok(())
}
