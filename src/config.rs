use crate::{Exchange, Pair, Place, Platform};
use std::time::Duration;

pub struct Config {
    pub(crate) place: Place,
    pub(crate) book_cap: usize,
    pub(crate) pairs: Vec<Pair>,
    pub(crate) system: SystemConfig,
}

#[derive(Clone)]
pub(crate) struct SystemConfig {
    pub(crate) streams_per_connection: usize,
    pub(crate) reconnect_delay: Duration,
    pub(crate) log_prefix: String,
    pub(crate) update_speed: String,
    pub(crate) max_latency: Duration,
    pub(crate) latency_check_interval: Duration,
    pub(crate) max_latency_error: Duration,
}

impl Config {
    pub fn new(place: Place, book_cap: usize, pairs: Vec<Pair>) -> Self {
        let system = match place {
            (Exchange::Binance, Platform::Spot) => {
                SystemConfig {
                    // https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#websocket-limits
                    streams_per_connection: 128,
                    // https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#websocket-limits
                    reconnect_delay: Duration::from_secs(1),
                    log_prefix: String::from("[binance] [spot]"),
                    // https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams#diff-depth-stream
                    update_speed: String::from("1000ms"),
                    max_latency: Duration::from_secs(5),
                    latency_check_interval: Duration::from_secs(1),
                    max_latency_error: Duration::from_millis(100),
                }
            }
        };
        
        Self { place, book_cap, pairs, system }
    }

    #[must_use]
    pub fn streams_per_connection(mut self, streams_per_connection: usize) -> Self {
        self.system.streams_per_connection = streams_per_connection;

        self
    }

    #[must_use]
    pub fn reconnect_delay(mut self, reconnect_delay: Duration) -> Self {
        self.system.reconnect_delay = reconnect_delay;

        self
    }

    #[must_use]
    pub fn log_prefix(mut self, log_prefix: String) -> Self {
        self.system.log_prefix = log_prefix;

        self
    }

    #[must_use]
    pub fn update_speed(mut self, update_speed: String) -> Self {
        self.system.update_speed = update_speed;

        self
    }

    #[must_use]
    pub fn max_latency(mut self, max_latency: Duration) -> Self {
        self.system.max_latency = max_latency;

        self
    }

    #[must_use]
    pub fn latency_check_interval(mut self, latency_check_interval: Duration) -> Self {
        self.system.latency_check_interval = latency_check_interval;

        self
    }

    #[must_use]
    pub fn max_latency_error(mut self, max_latency_error: Duration) -> Self {
        self.system.max_latency_error = max_latency_error;

        self
    }
}
