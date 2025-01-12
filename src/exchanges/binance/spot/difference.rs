use super::{snapshot::get_snapshot, Update};
use crate::{Book, Order, Pair, SystemConfig, TokenBucket};
use backon::Retryable;
use futures::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::mpsc;

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

fn check_latency(
    config: &SystemConfig,
    pair: &Pair,
    event: &EventPayload,
    tx: &mpsc::UnboundedSender<Duration>
) {
    let event_time = UNIX_EPOCH + Duration::from_millis(event.E);

    match event_time.elapsed() {
        Ok(latency) => if latency > config.max_latency {
            tx.send(latency).unwrap();
        }
        Err(err) => if err.duration() > config.max_latency_error {
            eprintln!("{} [{pair}]: latency error - {:?}", config.log_prefix, err.duration());
        }
    }
}

fn apply_event(book: &Arc<Mutex<Book>>, event: EventPayload) {
    let mut book = book.lock().unwrap();

    for update in event.b {
        book.bids.diff_update(Order::from(update))
    }
    for update in event.a {
        book.asks.diff_update(Order::from(update))
    }
}

/// <https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#how-to-manage-a-local-order-book-correctly>
///
/// ### Snapshot and Event Flow
///
/// ```text
///    U-------u
///    | Event |
///    +-------+
///            U-------u
///            | Event |
///            +-------+
///              U--------------u
///              |    Event     |
///              +--------------+
///                U----------u
///                | Snapshot |
///                +----------+
///                   U-----u
///                   |Event|
///                   +-----+
///                       U-------u
///                       | Event |
///                       +-------+
///                               U-------u
///                               | Event |
///                               +-------+
/// ```
async fn run_pair(
    config: SystemConfig,
    pair: Pair,
    book: Arc<Mutex<Book>>,
    mut rx: mpsc::UnboundedReceiver<EventPayload>,
    r_tb: Arc<TokenBucket>,
    w_tb: Arc<TokenBucket>,
    lat_tx: mpsc::UnboundedSender<Duration>,
) {
    // Wait until first event appears in case websocket server
    // will start sending events too late (later than we get snapshot).
    while rx.is_empty() {
        if rx.is_closed() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let book_cap = book.lock().unwrap().capacity();

    'from_snapshot: loop {
        let snapshot = (|| get_snapshot(&pair, book_cap, &r_tb, &w_tb))
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

        let mut prev_u;

        loop {
            match rx.recv().await {
                Some(event) => {
                    if event.u <= snapshot.lastUpdateId {
                        // Snapshot covers this event.
                        continue;
                    }
                    if event.U > snapshot.lastUpdateId + 1 {
                        // We missed some event.
                        eprintln!(
                            "{} [{pair}]: U ({}) > lastUpdateId ({}) + 1",
                            config.log_prefix, event.U, snapshot.lastUpdateId
                        );
                        continue 'from_snapshot;
                    }
                    prev_u = event.u;

                    check_latency(&config, &pair, &event, &lat_tx);
                    apply_event(&book, event);
                    break;
                }
                None => break 'from_snapshot
            }
        }

        loop {
            match rx.recv().await {
                Some(event) => {
                    if event.U != prev_u + 1 {
                        // We missed some event.
                        eprintln!(
                            "{} [{pair}]: U ({}) != prev_u ({prev_u}) + 1",
                            config.log_prefix, event.U
                        );
                        continue 'from_snapshot;
                    }
                    prev_u = event.u;

                    check_latency(&config, &pair, &event, &lat_tx);
                    apply_event(&book, event);
                }
                None => break 'from_snapshot
            }
        }
    }
}

/// <https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams> \
/// <https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#diff-depth-stream>
pub(super) async fn run_connection(
    config: &SystemConfig,
    books: &HashMap<Pair, Arc<Mutex<Book>>>,
    r_tb: &Arc<TokenBucket>,
    w_tb: &Arc<TokenBucket>,
    lat_tx: &mpsc::UnboundedSender<Duration>,
) -> Result<(), tokio_websockets::Error> {
    let uri = http::Uri::from_str(&format!(
        "wss://data-stream.binance.vision/stream?streams={}",
        books.keys()
            .map(|p| format!("{}@depth@{}", p.fused(), config.update_speed))
            .collect::<Vec<String>>()
            .join("/")
    )).unwrap();
    let (mut client, _) =
        tokio_websockets::ClientBuilder::from_uri(uri).connect().await?;

    let txs = HashMap::<_, _>::from_iter(
        books.iter().map(|(p, b)| (
            p.fused_upper(),
            {
                let (tx, rx) = mpsc::unbounded_channel();
                tokio::spawn(run_pair(
                    config.clone(), p.clone(), Arc::clone(b), rx, Arc::clone(r_tb), Arc::clone(w_tb),
                    lat_tx.clone()
                ));
                tx
            }
        ))
    );
    while let Some(msg) = client.next().await {
        let msg = msg?;
        
        if msg.is_text() || msg.is_binary() {
            let body = msg.as_payload();
            let event = serde_json::from_slice::<Event>(body).unwrap().data;
            txs[&event.s].send(event).unwrap();
        }
    }

    Ok(())
}
