use crate::TokenBucket;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct ExchangeInfo {
    rateLimits: Vec<RateLimit>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct RateLimit {
    interval: String,
    intervalNum: u32,
    limit: usize,
    rateLimitType: String,
}

/// https://developers.binance.com/docs/binance-spot-api-docs/rest-api/limits \
/// https://developers.binance.com/docs/binance-spot-api-docs/rest-api/public-api-endpoints#exchange-information
pub(super) async fn get_rate_limits_tbs() -> reqwest::Result<(Arc<TokenBucket>, Arc<TokenBucket>)> {
    let exchange_info = reqwest::Client::new()
        .get("https://data-api.binance.vision/api/v3/exchangeInfo")
        .send()
        .await?
        .json::<ExchangeInfo>()
        .await?;

    let intervals_map = HashMap::from([
        (String::from("SECOND"), Duration::from_secs(1)),
        (String::from("MINUTE"), Duration::from_secs(60)),
        (String::from("HOUR"), Duration::from_secs(60 * 60)),
        (String::from("DAY"), Duration::from_secs(60 * 60 * 24)),
    ]);

    let mut tbs = HashMap::<_, _>::from_iter(
        exchange_info.rateLimits.into_iter().map(|rl| (
            rl.rateLimitType,
            TokenBucket::new(rl.limit, rl.intervalNum * intervals_map[&rl.interval])
        ))
    );
    
    Ok((
        Arc::new(tbs.remove("RAW_REQUESTS").unwrap()),
        Arc::new(tbs.remove("REQUEST_WEIGHT").unwrap()),
    ))
}
