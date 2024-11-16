mod book;
mod exchanges;
mod pair;
mod token_bucket;

use backon::Retryable;
use book::{Book, Order};
use pair::Pair;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use token_bucket::TokenBucket;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Exchange {
    Binance,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Platform {
    Spot,
}

type Place = (Exchange, Platform);

async fn create_books<T, F>(
    enabled_places: &[Place],
    book_sizes: &HashMap<Place, usize>,
    pairs_getters: &HashMap<Place, T>,
) -> reqwest::Result<HashMap<Place, HashMap<Pair, Arc<Mutex<Book>>>>>
where
    T: Fn() -> F,
    F: Future<Output = reqwest::Result<Vec<Pair>>>,
{
    let mut books = HashMap::with_capacity(enabled_places.len());

    for place in enabled_places {
        let pairs = (|| pairs_getters[place]())
            .retry(backon::ExponentialBuilder::default())
            .await?;

        let place_books = HashMap::from_iter(
            pairs.into_iter().map(|p| (
                p,
                Arc::new(Mutex::new(Book::new(book_sizes[place])))
            ))
        );
        books.insert(place.clone(), place_books);
    }

    Ok(books)
}

#[tokio::main]
async fn main() {
    let enabled_places = [
        (Exchange::Binance, Platform::Spot),
    ];

    let book_sizes = HashMap::<_, usize>::from([
        ((Exchange::Binance, Platform::Spot), exchanges::binance::spot::BOOK_SIZE)
    ]);
    let pairs_getters = HashMap::from([
        ((Exchange::Binance, Platform::Spot), exchanges::binance::spot::pairs::get_pairs)
    ]);

    let books = create_books(
        &enabled_places, &book_sizes, &pairs_getters,
    ).await.unwrap();

    tokio::spawn(exchanges::binance::spot::run(books[&(Exchange::Binance, Platform::Spot)].clone()));

    loop {
        let place = (Exchange::Binance, Platform::Spot);
        let pair = Pair {
            ba: String::from("btc"),
            qa: String::from("usdt"),
        };
        {
            let book = books[&place][&pair].lock().unwrap();
            if !book.bids.orders().is_empty() && !book.asks.orders().is_empty() {
                println!("{:?} {:?}", book.bids.orders()[0], book.asks.orders()[0]);
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
