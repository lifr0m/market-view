use super::Update;
use crate::{Pair, TokenBucket};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct Snapshot {
    pub lastUpdateId: u64,
    pub bids: Vec<Update>,
    pub asks: Vec<Update>,
}

/// https://developers.binance.com/docs/binance-spot-api-docs/rest-api/public-api-endpoints#order-book
pub async fn get_snapshot(
    pair: &Pair,
    size: usize,
    r_tb: &Arc<TokenBucket>,
    w_tb: &Arc<TokenBucket>,
) -> reqwest::Result<Snapshot> {
    let weight =
        if size <= 100 { 5 }
        else if size <= 500 { 25 }
        else if size <= 1000 { 50 }
        else { 250 };

    r_tb.acquire(1).await;
    w_tb.acquire(weight).await;
    
    reqwest::Client::new()
        .get("https://data-api.binance.vision/api/v3/depth")
        .query(&json!({
            "symbol": pair.fused_upper(),
            "limit": size,
        }))
        .send()
        .await?
        .json::<Snapshot>()
        .await
}
