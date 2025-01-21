use market_view::{exchanges, Book, Exchange, Pair, Place, Platform};
use std::collections::HashMap;
use std::time::Duration;

const PRINT_INTERVAL: Duration = Duration::from_secs(1);

fn print_best_orders(books: HashMap<&Place, HashMap<&Pair, Book>>) {
    let place = (Exchange::Binance, Platform::Spot);
    let pair = Pair::new(String::from("btc"), String::from("usdt"));
    let book = &books[&place][&pair];

    if book.bids().is_empty() || book.asks().is_empty() {
        println!("Starting, wait please...");
    } else {
        println!(
            "[{pair}] Best bid: {:?} Best ask: {:?}",
            book.bids()[0],
            book.asks()[0]
        );
    }
}

#[tokio::main]
async fn main() {
    println!("You can check it here: https://www.binance.com/en/trade/BTC_USDT?type=spot");

    let books = market_view::start(vec![
        market_view::Config::new(
            (Exchange::Binance, Platform::Spot),
            100,
            // vec![Pair::new(String::from("btc"), String::from("usdt"))],
            exchanges::binance::spot::get_pairs().await.unwrap(),
        )
    ]);

    loop {
        print_best_orders(market_view::copy_books(&books));
        tokio::time::sleep(PRINT_INTERVAL).await;
    }
}
