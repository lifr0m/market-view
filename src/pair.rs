#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pair {
    /// Base asset
    pub ba: String,
    /// Quote asset
    pub qa: String,
}

impl Pair {
    pub fn new(ba: String, qa: String) -> Self {
        Self { ba, qa }
    }
    
    pub fn fused(&self) -> String {
        format!("{}{}", self.ba, self.qa)
    }

    pub fn fused_upper(&self) -> String {
        self.fused().to_uppercase()
    }
}

impl std::fmt::Display for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}/{}", self.ba.to_uppercase(), self.qa.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair() {
        let pair = Pair::new(String::from("btc"), String::from("usdt"));

        assert_eq!(pair.fused(), "btcusdt");
        assert_eq!(pair.fused_upper(), "BTCUSDT");
        assert_eq!(format!("{pair}"), "BTC/USDT");
    }
}
