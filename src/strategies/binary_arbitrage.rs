//! Binary Market Arbitrage Detection
//!
//! Detects arbitrage in binary (YES/NO) markets when YES + NO ≠ $1.00
//!
//! # Strategy
//! Binary markets MUST sum to $1.00 at expiry (one pays $1, other pays $0).
//! When prices are inefficient, we can arbitrage BOTH sides:
//!
//! ## BUY Arbitrage (YES + NO < $1.00)
//! ```
//! YES ask: $0.45
//! NO ask:  $0.48
//! Sum:     $0.93  ← BUY BOTH!
//!
//! Cost: $0.93
//! Payout at expiry: $1.00 (guaranteed)
//! Profit: $0.07 (7.5% return in 15min-4h!)
//! ```
//!
//! ## SELL Arbitrage (YES + NO > $1.00)
//! ```
//! YES bid: $0.55
//! NO bid:  $0.52
//! Sum:     $1.07  ← SELL BOTH!
//!
//! Revenue: $1.07
//! Payout at expiry: $1.00 (guaranteed)
//! Profit: $0.07 (7% return!)
//! ```
//!
//! # Risk
//! ZERO market risk - you either own both outcomes (buy) or owe $1 (sell)!
//! Only execution risk (partial fill, fees, etc.)

use crate::types::{OrderBook, MarketId, TokenId};

/// Arbitrage side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrageSide {
    /// Buy both YES and NO (when sum < $1.00)
    Buy,
    /// Sell both YES and NO (when sum > $1.00)
    Sell,
}

/// Binary arbitrage opportunity
#[derive(Debug, Clone)]
pub struct BinaryArbitrageOpportunity {
    /// Market ID
    pub market_id: MarketId,

    /// YES token ID
    pub yes_token_id: TokenId,

    /// NO token ID
    pub no_token_id: TokenId,

    /// Arbitrage side (buy or sell)
    pub side: ArbitrageSide,

    /// YES price (ask for buy, bid for sell)
    pub yes_price: f64,

    /// NO price (ask for buy, bid for sell)
    pub no_price: f64,

    /// Sum of prices
    pub price_sum: f64,

    /// Profit margin (abs(1.00 - sum))
    pub profit_margin: f64,

    /// Maximum tradeable size (limited by smaller side)
    pub max_size: f64,

    /// Expected profit in USDC
    pub expected_profit: f64,

    /// Market title (e.g., "BTC up or down 15min")
    pub title: String,

    /// Expiry time (if known)
    pub expiry: Option<String>,
}

impl BinaryArbitrageOpportunity {
    /// Create from two orderbooks (YES and NO)
    ///
    /// Detects BOTH buy and sell arbitrage opportunities:
    /// - BUY: When YES ask + NO ask < $1.00
    /// - SELL: When YES bid + NO bid > $1.00
    pub fn from_orderbooks(
        market_id: MarketId,
        yes_token_id: TokenId,
        no_token_id: TokenId,
        yes_orderbook: &OrderBook,
        no_orderbook: &OrderBook,
        title: String,
        expiry: Option<String>,
    ) -> Option<Self> {
        // Try BUY arbitrage first (YES ask + NO ask < $1.00)
        if let Some(yes_ask) = yes_orderbook.best_ask() {
            if let Some(no_ask) = no_orderbook.best_ask() {
                let yes_price = yes_ask.price;
                let no_price = no_ask.price;
                let price_sum = yes_price + no_price;

                // BUY arbitrage: sum < $1.00
                if price_sum < 1.0 {
                    let profit_margin = 1.0 - price_sum;
                    let max_size = yes_ask.size.min(no_ask.size);
                    let expected_profit = profit_margin * max_size;

                    return Some(Self {
                        market_id,
                        yes_token_id,
                        no_token_id,
                        side: ArbitrageSide::Buy,
                        yes_price,
                        no_price,
                        price_sum,
                        profit_margin,
                        max_size,
                        expected_profit,
                        title,
                        expiry,
                    });
                }
            }
        }

        // Try SELL arbitrage (YES bid + NO bid > $1.00)
        if let Some(yes_bid) = yes_orderbook.best_bid() {
            if let Some(no_bid) = no_orderbook.best_bid() {
                let yes_price = yes_bid.price;
                let no_price = no_bid.price;
                let price_sum = yes_price + no_price;

                // SELL arbitrage: sum > $1.00
                if price_sum > 1.0 {
                    let profit_margin = price_sum - 1.0;
                    let max_size = yes_bid.size.min(no_bid.size);
                    let expected_profit = profit_margin * max_size;

                    return Some(Self {
                        market_id,
                        yes_token_id,
                        no_token_id,
                        side: ArbitrageSide::Sell,
                        yes_price,
                        no_price,
                        price_sum,
                        profit_margin,
                        max_size,
                        expected_profit,
                        title,
                        expiry,
                    });
                }
            }
        }

        // No arbitrage opportunity
        None
    }
}

/// Binary arbitrage detector configuration
#[derive(Debug, Clone)]
pub struct BinaryArbitrageConfig {
    /// Minimum profit margin (e.g., 0.02 = 2%)
    pub min_profit_margin: f64,

    /// Minimum size in USDC
    pub min_size: f64,

    /// Maximum total cost per trade
    pub max_cost: f64,
}

impl Default for BinaryArbitrageConfig {
    fn default() -> Self {
        Self {
            min_profit_margin: 0.02, // 2% minimum (to cover fees)
            min_size: 5.0,           // $5 minimum
            max_cost: 100.0,         // Max $100 total cost
        }
    }
}

/// Binary arbitrage detector
pub struct BinaryArbitrageDetector {
    config: BinaryArbitrageConfig,
}

impl BinaryArbitrageDetector {
    /// Create new detector
    pub fn new(config: BinaryArbitrageConfig) -> Self {
        Self { config }
    }

    /// Detect arbitrage in binary market pair
    pub fn detect(
        &self,
        market_id: &MarketId,
        yes_token_id: &TokenId,
        no_token_id: &TokenId,
        yes_orderbook: &OrderBook,
        no_orderbook: &OrderBook,
        title: String,
        expiry: Option<String>,
    ) -> Option<BinaryArbitrageOpportunity> {
        // Try to find opportunity
        let opportunity = BinaryArbitrageOpportunity::from_orderbooks(
            market_id.clone(),
            yes_token_id.clone(),
            no_token_id.clone(),
            yes_orderbook,
            no_orderbook,
            title,
            expiry,
        )?;

        // Check minimum profit margin
        if opportunity.profit_margin < self.config.min_profit_margin {
            return None;
        }

        // Check minimum size
        if opportunity.max_size < self.config.min_size {
            return None;
        }

        // Check maximum cost
        let total_cost = opportunity.price_sum * opportunity.max_size;
        if total_cost > self.config.max_cost {
            return None;
        }

        Some(opportunity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderBookEntry;

    fn create_orderbook(ask_price: f64, bid_price: f64, size: f64) -> OrderBook {
        OrderBook {
            token_id: TokenId("test".to_string()),
            bids: vec![OrderBookEntry {
                price: bid_price,
                size,
                timestamp: Some(1000),
            }],
            asks: vec![OrderBookEntry {
                price: ask_price,
                size,
                timestamp: Some(1000),
            }],
            timestamp: 1000,
        }
    }

    #[test]
    fn test_buy_arbitrage_detection() {
        let market_id = MarketId("btc-15min".to_string());
        let yes_token = TokenId("yes-token".to_string());
        let no_token = TokenId("no-token".to_string());

        // YES ask: $0.45, NO ask: $0.48, Sum: $0.93 (7% profit!)
        let yes_orderbook = create_orderbook(0.45, 0.43, 100.0);
        let no_orderbook = create_orderbook(0.48, 0.46, 100.0);

        let opportunity = BinaryArbitrageOpportunity::from_orderbooks(
            market_id,
            yes_token,
            no_token,
            &yes_orderbook,
            &no_orderbook,
            "BTC Up/Down 15min".to_string(),
            None,
        );

        assert!(opportunity.is_some());
        let opp = opportunity.unwrap();

        assert_eq!(opp.side, ArbitrageSide::Buy);
        assert!((opp.yes_price - 0.45).abs() < 0.001);
        assert!((opp.no_price - 0.48).abs() < 0.001);
        assert!((opp.price_sum - 0.93).abs() < 0.001);
        assert!((opp.profit_margin - 0.07).abs() < 0.001);
        assert!((opp.max_size - 100.0).abs() < 0.001);
        assert!((opp.expected_profit - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_sell_arbitrage_detection() {
        let market_id = MarketId("eth-1h".to_string());
        let yes_token = TokenId("yes-token".to_string());
        let no_token = TokenId("no-token".to_string());

        // YES bid: $0.55, NO bid: $0.52, Sum: $1.07 (7% profit!)
        let yes_orderbook = create_orderbook(0.57, 0.55, 100.0);
        let no_orderbook = create_orderbook(0.54, 0.52, 100.0);

        let opportunity = BinaryArbitrageOpportunity::from_orderbooks(
            market_id,
            yes_token,
            no_token,
            &yes_orderbook,
            &no_orderbook,
            "ETH Up/Down 1hour".to_string(),
            None,
        );

        assert!(opportunity.is_some());
        let opp = opportunity.unwrap();

        assert_eq!(opp.side, ArbitrageSide::Sell);
        assert!((opp.yes_price - 0.55).abs() < 0.001);
        assert!((opp.no_price - 0.52).abs() < 0.001);
        assert!((opp.price_sum - 1.07).abs() < 0.001);
        assert!((opp.profit_margin - 0.07).abs() < 0.001);
        assert!((opp.max_size - 100.0).abs() < 0.001);
        assert!((opp.expected_profit - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_no_arbitrage_when_sum_equals_one() {
        let market_id = MarketId("btc-15min".to_string());
        let yes_token = TokenId("yes-token".to_string());
        let no_token = TokenId("no-token".to_string());

        // Ask sum = $1.00, Bid sum = $1.00 (no arbitrage)
        let yes_orderbook = create_orderbook(0.51, 0.50, 100.0);
        let no_orderbook = create_orderbook(0.49, 0.50, 100.0);

        let opportunity = BinaryArbitrageOpportunity::from_orderbooks(
            market_id,
            yes_token,
            no_token,
            &yes_orderbook,
            &no_orderbook,
            "BTC Up/Down 15min".to_string(),
            None,
        );

        assert!(opportunity.is_none());
    }

    #[test]
    fn test_buy_arbitrage_prefers_buy_over_sell() {
        let market_id = MarketId("sol-4h".to_string());
        let yes_token = TokenId("yes-token".to_string());
        let no_token = TokenId("no-token".to_string());

        // Ask sum < $1.00 (buy arbitrage)
        // Bid sum > $1.00 (sell arbitrage)
        // Should prefer BUY (checked first)
        let yes_orderbook = create_orderbook(0.46, 0.54, 100.0);
        let no_orderbook = create_orderbook(0.48, 0.52, 100.0);

        let opportunity = BinaryArbitrageOpportunity::from_orderbooks(
            market_id,
            yes_token,
            no_token,
            &yes_orderbook,
            &no_orderbook,
            "SOL Up/Down 4hour".to_string(),
            None,
        );

        assert!(opportunity.is_some());
        let opp = opportunity.unwrap();

        // Should choose BUY arbitrage (checked first)
        assert_eq!(opp.side, ArbitrageSide::Buy);
        assert!((opp.price_sum - 0.94).abs() < 0.001);
        assert!((opp.profit_margin - 0.06).abs() < 0.001);
    }

    #[test]
    fn test_detector_filters_by_config() {
        let config = BinaryArbitrageConfig {
            min_profit_margin: 0.05, // 5% minimum
            min_size: 10.0,
            max_cost: 50.0,
        };
        let detector = BinaryArbitrageDetector::new(config);

        let market_id = MarketId("btc-15min".to_string());
        let yes_token = TokenId("yes-token".to_string());
        let no_token = TokenId("no-token".to_string());

        // YES: $0.47, NO: $0.50, Sum: $0.97 (3% profit - below 5% threshold)
        let yes_orderbook = create_orderbook(0.47, 0.45, 100.0);
        let no_orderbook = create_orderbook(0.50, 0.48, 100.0);

        let result = detector.detect(
            &market_id,
            &yes_token,
            &no_token,
            &yes_orderbook,
            &no_orderbook,
            "BTC Up/Down 15min".to_string(),
            None,
        );

        assert!(result.is_none(), "Should filter out 3% profit when min is 5%");
    }
}
