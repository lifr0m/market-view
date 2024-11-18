mod book;
mod config;
pub mod exchanges;
mod hashmap_chunks;
mod latency_meter;
mod pair;
mod token_bucket;

pub use book::{Book, Order};
pub use config::Config;
use config::SystemConfig;
use hashmap_chunks::HashMapChunks;
use latency_meter::LatencyMeter;
pub use pair::Pair;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use token_bucket::TokenBucket;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Exchange {
    Binance,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Platform {
    Spot,
}

pub type Place = (Exchange, Platform);

pub fn start(configs: Vec<Config>) -> HashMap<Place, HashMap<Pair, Arc<Mutex<Book>>>> {
    HashMap::from_iter(
        configs.into_iter().map(|config| (
            config.place,
            {
                let books = HashMap::from_iter(
                    config.pairs.into_iter().map(|pair| (
                        pair,
                        Arc::new(Mutex::new(Book::new(config.book_cap)))
                    ))
                );

                let spawner = match config.place {
                    (Exchange::Binance, Platform::Spot) => exchanges::binance::spot::spawn,
                };
                tokio::spawn(spawner(config.system, books.clone()));

                books
            }
        ))
    )
}

pub fn copy_books(
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
