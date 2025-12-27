//! Market data type definitions
//!
//! Defines all market-related data structures including markets, order books, and outcomes.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a market
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(pub String);

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a token (used in CLOB trading)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenId(pub String);

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Market outcome (YES or NO in binary markets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    YES,
    NO,
}

/// Market status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketStatus {
    Active,
    Closed,
    Resolved,
}

/// Market metadata and current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    /// Unique market identifier
    pub id: MarketId,

    /// Human-readable market question
    pub question: String,

    /// URL-friendly market slug
    pub slug: String,

    /// Possible outcomes
    pub outcomes: Vec<Outcome>,

    /// Current prices for each outcome (0.0-1.0)
    pub outcome_prices: Vec<f64>,

    /// Token IDs for CLOB trading
    pub clob_token_ids: Vec<TokenId>,

    /// Market status
    pub status: MarketStatus,

    /// Market expiration timestamp (Unix timestamp)
    pub expiration_timestamp: i64,

    /// Total volume traded (USDC)
    pub volume: f64,

    /// Total liquidity available (USDC)
    pub liquidity: f64,
}

/// Order book entry (bid or ask)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrderBookEntry {
    /// Price (0.0-1.0)
    pub price: f64,

    /// Size in shares
    pub size: f64,

    /// Timestamp of entry (Unix timestamp in milliseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Complete order book for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    /// Token ID
    pub token_id: TokenId,

    /// Buy orders (sorted highest to lowest)
    pub bids: Vec<OrderBookEntry>,

    /// Sell orders (sorted lowest to highest)
    pub asks: Vec<OrderBookEntry>,

    /// Last update timestamp (Unix timestamp in milliseconds)
    pub timestamp: i64,
}

impl OrderBook {
    /// Get the best bid (highest buy price)
    pub fn best_bid(&self) -> Option<&OrderBookEntry> {
        self.bids.first()
    }

    /// Get the best ask (lowest sell price)
    pub fn best_ask(&self) -> Option<&OrderBookEntry> {
        self.asks.first()
    }

    /// Check if order book has sufficient depth
    pub fn has_depth(&self) -> bool {
        !self.bids.is_empty() && !self.asks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_book_best_bid() {
        let order_book = OrderBook {
            token_id: TokenId("test".to_string()),
            bids: vec![
                OrderBookEntry { price: 0.75, size: 100.0, timestamp: None },
                OrderBookEntry { price: 0.70, size: 50.0, timestamp: None },
            ],
            asks: vec![],
            timestamp: 0,
        };

        assert_eq!(order_book.best_bid().unwrap().price, 0.75);
    }

    #[test]
    fn test_order_book_best_ask() {
        let order_book = OrderBook {
            token_id: TokenId("test".to_string()),
            bids: vec![],
            asks: vec![
                OrderBookEntry { price: 0.70, size: 100.0, timestamp: None },
                OrderBookEntry { price: 0.75, size: 50.0, timestamp: None },
            ],
            timestamp: 0,
        };

        assert_eq!(order_book.best_ask().unwrap().price, 0.70);
    }

    #[test]
    fn test_order_book_has_depth() {
        let order_book = OrderBook {
            token_id: TokenId("test".to_string()),
            bids: vec![OrderBookEntry { price: 0.75, size: 100.0, timestamp: None }],
            asks: vec![OrderBookEntry { price: 0.70, size: 100.0, timestamp: None }],
            timestamp: 0,
        };

        assert!(order_book.has_depth());
    }

    #[test]
    fn test_order_book_no_depth_empty_bids() {
        let order_book = OrderBook {
            token_id: TokenId("test".to_string()),
            bids: vec![],
            asks: vec![OrderBookEntry { price: 0.70, size: 100.0, timestamp: None }],
            timestamp: 0,
        };

        assert!(!order_book.has_depth());
    }
}
