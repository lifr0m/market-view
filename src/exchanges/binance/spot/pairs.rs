use crate::Pair;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct ExchangeInfo {
    symbols: Vec<Symbol>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct Symbol {
    baseAsset: String,
    quoteAsset: String,
}

/// <https://developers.binance.com/docs/binance-spot-api-docs/rest-api/general-endpoints#exchange-information>
pub async fn get_pairs() -> reqwest::Result<Vec<Pair>> {
    let exchange_info = reqwest::Client::new()
        .get("https://data-api.binance.vision/api/v3/exchangeInfo")
        .query(&json!({
            "permissions": "SPOT",
            "symbolStatus": "TRADING",
        }))
        .send()
        .await?
        .json::<ExchangeInfo>()
        .await?;

    Ok(exchange_info.symbols
        .into_iter()
        .map(|s| Pair::new(
            s.baseAsset.to_lowercase(),
            s.quoteAsset.to_lowercase(),
        ))
        .collect())
}
