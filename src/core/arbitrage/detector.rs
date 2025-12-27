//! Arbitrage detection module
//!
//! Implements both scalar and SIMD-optimized arbitrage detection algorithms.

use crate::types::{ArbitrageOpportunity, MarketId, OrderBook, TokenId};

/// Configuration for arbitrage detection
#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    /// Minimum profit margin required (0.0-1.0)
    pub min_profit_margin: f64,

    /// Minimum size in USDC to consider
    pub min_size: f64,

    /// Maximum spread to consider valid (sanity check)
    pub max_spread: f64,
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            min_profit_margin: 0.02, // 2%
            min_size: 10.0,          // $10 minimum
            max_spread: 0.50,        // 50% max spread (sanity check)
        }
    }
}

/// Scalar arbitrage detector (baseline implementation)
pub struct ScalarArbitrageDetector {
    config: ArbitrageConfig,
}

impl ScalarArbitrageDetector {
    /// Create a new scalar detector
    pub fn new(config: ArbitrageConfig) -> Self {
        Self { config }
    }

    /// Detect arbitrage opportunity from an order book
    ///
    /// Returns Some(opportunity) if profitable arbitrage exists, None otherwise.
    pub fn detect(
        &self,
        market_id: &MarketId,
        token_id: &TokenId,
        order_book: &OrderBook,
    ) -> Option<ArbitrageOpportunity> {
        // Get best bid and ask
        let best_bid = order_book.best_bid()?;
        let best_ask = order_book.best_ask()?;

        // Calculate maximum tradeable size
        let max_size = best_bid.size.min(best_ask.size);

        // Skip if size too small
        if max_size < self.config.min_size {
            return None;
        }

        // Check for arbitrage (bid > ask)
        if best_bid.price <= best_ask.price {
            return None;
        }

        // Calculate profit margin
        let spread = best_bid.price - best_ask.price;
        let profit_margin = spread / best_ask.price;

        // Sanity check: reject unrealistic spreads
        if spread > self.config.max_spread {
            return None;
        }

        // Check if meets minimum profit threshold
        if profit_margin < self.config.min_profit_margin {
            return None;
        }

        // Create opportunity
        ArbitrageOpportunity::new(
            market_id.clone(),
            token_id.clone(),
            best_bid.price,
            best_ask.price,
            max_size,
        )
    }

    /// Detect opportunities across multiple order books
    pub fn detect_batch(
        &self,
        markets: &[(MarketId, TokenId, OrderBook)],
    ) -> Vec<ArbitrageOpportunity> {
        markets
            .iter()
            .filter_map(|(market_id, token_id, order_book)| {
                self.detect(market_id, token_id, order_book)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderBookEntry, Outcome};

    fn create_test_order_book(bid_price: f64, ask_price: f64, size: f64) -> OrderBook {
        OrderBook {
            token_id: TokenId("test-token".to_string()),
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
    fn test_detect_valid_arbitrage() {
        let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        // Bid > Ask (arbitrage exists)
        let order_book = create_test_order_book(0.75, 0.70, 100.0);

        let opportunity = detector.detect(&market_id, &token_id, &order_book);

        assert!(opportunity.is_some());
        let opp = opportunity.unwrap();
        assert_eq!(opp.bid_price, 0.75);
        assert_eq!(opp.ask_price, 0.70);
        assert!((opp.profit_margin - 0.0714).abs() < 0.001); // ~7.14%
    }

    #[test]
    fn test_no_arbitrage_normal_market() {
        let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        // Bid < Ask (normal market, no arbitrage)
        let order_book = create_test_order_book(0.70, 0.75, 100.0);

        let opportunity = detector.detect(&market_id, &token_id, &order_book);

        assert!(opportunity.is_none());
    }

    #[test]
    fn test_below_profit_threshold() {
        let mut config = ArbitrageConfig::default();
        config.min_profit_margin = 0.10; // 10% minimum

        let detector = ScalarArbitrageDetector::new(config);
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        // 7.14% profit (below 10% threshold)
        let order_book = create_test_order_book(0.75, 0.70, 100.0);

        let opportunity = detector.detect(&market_id, &token_id, &order_book);

        assert!(opportunity.is_none());
    }

    #[test]
    fn test_size_too_small() {
        let mut config = ArbitrageConfig::default();
        config.min_size = 100.0; // Require $100 minimum

        let detector = ScalarArbitrageDetector::new(config);
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        // Only $50 available
        let order_book = create_test_order_book(0.75, 0.70, 50.0);

        let opportunity = detector.detect(&market_id, &token_id, &order_book);

        assert!(opportunity.is_none());
    }

    #[test]
    fn test_sanity_check_max_spread() {
        let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        // Unrealistic 95% spread (likely bad data)
        let order_book = create_test_order_book(1.00, 0.05, 100.0);

        let opportunity = detector.detect(&market_id, &token_id, &order_book);

        assert!(opportunity.is_none(), "Should reject unrealistic spreads");
    }

    #[test]
    fn test_empty_order_book() {
        let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        let order_book = OrderBook {
            token_id: TokenId("test".to_string()),
            bids: vec![],
            asks: vec![],
            timestamp: 1000,
        };

        let opportunity = detector.detect(&market_id, &token_id, &order_book);

        assert!(opportunity.is_none());
    }

    #[test]
    fn test_batch_detection() {
        let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());

        let markets = vec![
            // Good arbitrage
            (
                MarketId("m1".to_string()),
                TokenId("t1".to_string()),
                create_test_order_book(0.75, 0.70, 100.0),
            ),
            // No arbitrage (normal market)
            (
                MarketId("m2".to_string()),
                TokenId("t2".to_string()),
                create_test_order_book(0.70, 0.75, 100.0),
            ),
            // Good arbitrage
            (
                MarketId("m3".to_string()),
                TokenId("t3".to_string()),
                create_test_order_book(0.80, 0.75, 100.0),
            ),
        ];

        let opportunities = detector.detect_batch(&markets);

        assert_eq!(opportunities.len(), 2);
        assert_eq!(opportunities[0].market_id.0, "m1");
        assert_eq!(opportunities[1].market_id.0, "m3");
    }

    #[test]
    fn test_max_size_limited_by_both_sides() {
        let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        let order_book = OrderBook {
            token_id: TokenId("test".to_string()),
            bids: vec![OrderBookEntry {
                price: 0.75,
                size: 50.0, // Smaller bid
                timestamp: Some(1000),
            }],
            asks: vec![OrderBookEntry {
                price: 0.70,
                size: 100.0, // Larger ask
                timestamp: Some(1000),
            }],
            timestamp: 1000,
        };

        let opportunity = detector.detect(&market_id, &token_id, &order_book);

        assert!(opportunity.is_some());
        let opp = opportunity.unwrap();
        assert_eq!(opp.max_size, 50.0, "Size should be limited by smaller side");
    }
}
