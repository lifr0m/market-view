mod book;
mod exchanges;
mod hashmap_chunks;
mod latency_meter;
mod pair;
mod token_bucket;

use backon::Retryable;
use book::{Book, Order};
use hashmap_chunks::HashMapChunks;
use latency_meter::LatencyMeter;
use pair::Pair;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use token_bucket::TokenBucket;

const CALCULATION_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Exchange {
    Binance,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Platform {
    Spot,
}

type Place = (Exchange, Platform);

async fn create_books<T, F, E>(
    enabled_places: &[Place],
    book_caps: &HashMap<Place, usize>,
    pairs_getters: &HashMap<Place, T>,
) -> Result<HashMap<Place, HashMap<Pair, Arc<Mutex<Book>>>>, E>
where
    T: Fn() -> F,
    F: Future<Output = Result<Vec<Pair>, E>>,
{
    let mut books = HashMap::with_capacity(enabled_places.len());

    for place in enabled_places {
        let pairs = (|| pairs_getters[place]())
            .retry(backon::ExponentialBuilder::default())
            .await?;

        let place_books = HashMap::from_iter(
            pairs.into_iter().map(|p| (
                p,
                Arc::new(Mutex::new(Book::new(book_caps[place])))
            ))
        );
        books.insert(place.clone(), place_books);
    }

    Ok(books)
}

fn spawn(books: &HashMap<Place, HashMap<Pair, Arc<Mutex<Book>>>>) {
    let spawners = HashMap::from([
        ((Exchange::Binance, Platform::Spot), exchanges::binance::spot::spawn)
    ]);
    for (place, spawner) in &spawners {
        tokio::spawn(spawner(books[place].clone()));
    }
}

fn copy_books(
    books: &HashMap<Place, HashMap<Pair, Arc<Mutex<Book>>>>
) -> HashMap<&Place, HashMap<&Pair, Book>> {
    HashMap::from_iter(
        books.iter().map(|(place, books)| (
            place,
            HashMap::from_iter(
                books.iter().map(|(pair, book)| (
                    pair,
                    book.lock().unwrap().clone()
                ))
            )
        ))
    )
}

fn do_some_calculations(books: HashMap<&Place, HashMap<&Pair, Book>>) {
    let place = (Exchange::Binance, Platform::Spot);
    let pair = Pair {
        ba: String::from("btc"),
        qa: String::from("usdt"),
    };

    let book = &books[&place][&pair];
    if !book.bids.orders().is_empty() && !book.asks.orders().is_empty() {
        println!("{:?} {:?}", book.bids.orders()[0], book.asks.orders()[0]);
    }
}

#[tokio::main]
async fn main() {
    let enabled_places = [
        (Exchange::Binance, Platform::Spot),
    ];
    let book_caps = HashMap::from([
        ((Exchange::Binance, Platform::Spot), 100)
    ]);
    let pairs_getters = HashMap::from([
        ((Exchange::Binance, Platform::Spot), exchanges::binance::spot::pairs::get_pairs)
    ]);
    
    let books = create_books(
        &enabled_places, &book_caps, &pairs_getters,
    ).await.unwrap();
    
    spawn(&books);

    loop {
        do_some_calculations(copy_books(&books));
        tokio::time::sleep(CALCULATION_INTERVAL).await;
    }
}
