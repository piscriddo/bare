//! Crypto Up/Down Market Strategy
//!
//! Trades short-timeframe directional markets (15min, 1h, 4h) for BTC, ETH, SOL, XRP.
//! This is DIRECTIONAL trading, NOT arbitrage - full market risk!
//!
//! # Strategy
//! 1. Fetch active crypto up/down markets from Gamma API
//! 2. Filter by timeframe (15m, 1h, 4h)
//! 3. Subscribe to orderbook updates
//! 4. Place directional bets (up or down)
//! 5. Redeem winning positions after expiry
//!
//! # Risk Warning
//! - This has FULL MARKET RISK (not risk-free arbitrage!)
//! - Stop loss is CRITICAL
//! - Can lose 100% of position
//! - Need to monitor actively

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;

/// Crypto asset for up/down markets
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CryptoAsset {
    /// Bitcoin (BTC)
    Bitcoin,
    /// Ethereum (ETH)
    Ethereum,
    /// Solana (SOL)
    Solana,
    /// XRP (Ripple)
    XRP,
}

impl CryptoAsset {
    /// Get slug patterns for this asset
    pub fn slug_patterns(&self) -> Vec<&'static str> {
        match self {
            CryptoAsset::Bitcoin => vec!["btc-updown", "bitcoin-up-or-down"],
            CryptoAsset::Ethereum => vec!["eth-updown", "ethereum-up-or-down"],
            CryptoAsset::Solana => vec!["sol-updown", "solana-up-or-down"],
            CryptoAsset::XRP => vec!["xrp-updown", "xrp-up-or-down"],
        }
    }

    /// Asset name for display
    pub fn name(&self) -> &'static str {
        match self {
            CryptoAsset::Bitcoin => "Bitcoin",
            CryptoAsset::Ethereum => "Ethereum",
            CryptoAsset::Solana => "Solana",
            CryptoAsset::XRP => "XRP",
        }
    }
}

/// Market timeframe
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timeframe {
    /// 15 minute market
    FifteenMin,
    /// 1 hour market
    OneHour,
    /// 4 hour market
    FourHour,
    /// Daily market
    Daily,
}

impl Timeframe {
    /// Parse timeframe from slug
    pub fn from_slug(slug: &str) -> Option<Self> {
        if slug.contains("-15m-") {
            Some(Timeframe::FifteenMin)
        } else if slug.contains("-1h-") || slug.contains("hour") || slug.contains("am-") || slug.contains("pm-") {
            Some(Timeframe::OneHour)
        } else if slug.contains("-4h-") {
            Some(Timeframe::FourHour)
        } else if slug.contains("-daily-") || slug.contains("daily") {
            Some(Timeframe::Daily)
        } else {
            None
        }
    }

    /// Duration in minutes
    pub fn duration_minutes(&self) -> u64 {
        match self {
            Timeframe::FifteenMin => 15,
            Timeframe::OneHour => 60,
            Timeframe::FourHour => 240,
            Timeframe::Daily => 1440,
        }
    }

    /// Display name
    pub fn display(&self) -> &'static str {
        match self {
            Timeframe::FifteenMin => "15min",
            Timeframe::OneHour => "1hour",
            Timeframe::FourHour => "4hour",
            Timeframe::Daily => "Daily",
        }
    }
}

/// Gamma API event response
#[derive(Debug, Clone, Deserialize)]
pub struct GammaEvent {
    /// Event ID
    pub id: String,

    /// Event slug (URL-friendly identifier)
    pub slug: String,

    /// Event title
    pub title: String,

    /// CLOB token IDs (YES/NO tokens)
    #[serde(rename = "clobTokenIds")]
    pub clob_token_ids: Vec<String>,

    /// Whether event is active
    pub active: bool,

    /// Whether event is closed
    pub closed: bool,

    /// End date timestamp
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
}

/// Gamma API response (returns array directly)
pub type GammaResponse = Vec<GammaEvent>;

/// Crypto up/down market
#[derive(Debug, Clone)]
pub struct CryptoUpDownMarket {
    /// Asset (BTC, ETH, SOL, XRP)
    pub asset: CryptoAsset,

    /// Timeframe (15m, 1h, 4h, daily)
    pub timeframe: Timeframe,

    /// Event ID
    pub event_id: String,

    /// Market slug
    pub slug: String,

    /// Market title
    pub title: String,

    /// Token IDs (YES/NO)
    pub token_ids: Vec<String>,

    /// End date
    pub end_date: Option<String>,
}

impl CryptoUpDownMarket {
    /// Create from Gamma event
    pub fn from_gamma_event(event: GammaEvent) -> Option<Self> {
        // Determine asset
        let asset = if event.slug.contains("btc") || event.slug.contains("bitcoin") {
            CryptoAsset::Bitcoin
        } else if event.slug.contains("eth") || event.slug.contains("ethereum") {
            CryptoAsset::Ethereum
        } else if event.slug.contains("sol") || event.slug.contains("solana") {
            CryptoAsset::Solana
        } else if event.slug.contains("xrp") {
            CryptoAsset::XRP
        } else {
            return None;
        };

        // Parse timeframe
        let timeframe = Timeframe::from_slug(&event.slug)?;

        // Validate token IDs (need at least 2 for YES/NO)
        if event.clob_token_ids.len() < 2 {
            return None;
        }

        Some(Self {
            asset,
            timeframe,
            event_id: event.id,
            slug: event.slug,
            title: event.title,
            token_ids: event.clob_token_ids,
            end_date: event.end_date,
        })
    }
}

/// Configuration for crypto up/down market fetching
#[derive(Debug, Clone)]
pub struct CryptoUpDownConfig {
    /// Assets to track
    pub assets: Vec<CryptoAsset>,

    /// Timeframes to include
    pub timeframes: Vec<Timeframe>,

    /// Maximum markets to fetch
    pub max_markets: usize,
}

impl Default for CryptoUpDownConfig {
    fn default() -> Self {
        Self {
            assets: vec![
                CryptoAsset::Bitcoin,
                CryptoAsset::Ethereum,
                CryptoAsset::Solana,
                CryptoAsset::XRP,
            ],
            timeframes: vec![
                Timeframe::FifteenMin,
                Timeframe::OneHour,
                Timeframe::FourHour,
            ],
            max_markets: 50,
        }
    }
}

/// Crypto up/down market fetcher
pub struct CryptoUpDownFetcher {
    /// HTTP client
    client: reqwest::Client,

    /// Configuration
    config: CryptoUpDownConfig,

    /// Gamma API base URL
    gamma_api_url: String,
}

impl CryptoUpDownFetcher {
    /// Create new fetcher
    pub fn new(config: CryptoUpDownConfig, gamma_api_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            gamma_api_url,
        }
    }

    /// Fetch active crypto up/down markets
    ///
    /// # Returns
    /// List of active markets matching configuration
    pub async fn fetch_markets(&self) -> Result<Vec<CryptoUpDownMarket>> {
        // Build API URL
        let url = format!(
            "{}/events?closed=false&archived=false&limit={}&offset=0&order=id&ascending=false",
            self.gamma_api_url,
            self.config.max_markets
        );

        tracing::info!("Fetching crypto up/down markets from: {}", url);

        // Fetch events
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch events: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("API returned error: {}", response.status()));
        }

        let gamma_response: GammaResponse = response.json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let total_events = gamma_response.len();
        tracing::info!("Fetched {} events from API", total_events);

        // Filter and convert
        let markets: Vec<CryptoUpDownMarket> = gamma_response
            .into_iter()
            .filter_map(|event| {
                // Check if event matches any asset pattern
                let matches_asset = self.config.assets.iter().any(|asset| {
                    asset.slug_patterns().iter().any(|pattern| {
                        event.slug.contains(pattern)
                    })
                });

                if !matches_asset {
                    return None;
                }

                // Parse market
                let market = CryptoUpDownMarket::from_gamma_event(event)?;

                // Check if timeframe is enabled
                if !self.config.timeframes.contains(&market.timeframe) {
                    return None;
                }

                Some(market)
            })
            .collect();

        tracing::info!(
            "Found {} crypto up/down markets (filtered from {} events)",
            markets.len(),
            total_events
        );

        // Log summary
        self.log_market_summary(&markets);

        Ok(markets)
    }

    /// Get token IDs from markets
    pub fn get_token_ids(markets: &[CryptoUpDownMarket]) -> Vec<String> {
        markets.iter()
            .flat_map(|m| m.token_ids.iter().cloned())
            .collect()
    }

    /// Group markets by asset and timeframe
    pub fn group_markets(markets: &[CryptoUpDownMarket]) -> HashMap<(CryptoAsset, Timeframe), Vec<CryptoUpDownMarket>> {
        let mut grouped: HashMap<(CryptoAsset, Timeframe), Vec<CryptoUpDownMarket>> = HashMap::new();

        for market in markets {
            let key = (market.asset.clone(), market.timeframe);
            grouped.entry(key).or_default().push(market.clone());
        }

        grouped
    }

    /// Log market summary
    fn log_market_summary(&self, markets: &[CryptoUpDownMarket]) {
        let grouped = Self::group_markets(markets);

        tracing::info!("Market Summary:");
        for ((asset, timeframe), markets) in grouped.iter() {
            tracing::info!(
                "  {} {}: {} markets",
                asset.name(),
                timeframe.display(),
                markets.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeframe_from_slug() {
        assert_eq!(Timeframe::from_slug("btc-updown-15m-est"), Some(Timeframe::FifteenMin));
        assert_eq!(Timeframe::from_slug("eth-up-or-down-1h-utc"), Some(Timeframe::OneHour));
        assert_eq!(Timeframe::from_slug("sol-updown-4h-est"), Some(Timeframe::FourHour));
        assert_eq!(Timeframe::from_slug("btc-daily-updown"), Some(Timeframe::Daily));
    }

    #[test]
    fn test_asset_slug_patterns() {
        let btc = CryptoAsset::Bitcoin;
        assert!(btc.slug_patterns().contains(&"btc-updown"));
        assert!(btc.slug_patterns().contains(&"bitcoin-up-or-down"));
    }
}
