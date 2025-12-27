//! Trade and position type definitions
//!
//! Defines structures for tracking trades and positions.

use serde::{Deserialize, Serialize};
use super::{MarketId, TokenId, OrderSide};

/// Trade execution data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Trade ID
    pub id: String,

    /// Market ID
    pub market_id: MarketId,

    /// Token ID
    pub token_id: TokenId,

    /// Side (BUY or SELL)
    pub side: OrderSide,

    /// Execution price
    pub price: f64,

    /// Size in shares
    pub size: f64,

    /// Execution timestamp (Unix timestamp in milliseconds)
    pub timestamp: i64,

    /// Maker address
    pub maker: String,

    /// Taker address
    pub taker: String,

    /// Transaction fee
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<f64>,
}

/// Position tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Market ID
    pub market_id: MarketId,

    /// Token ID
    pub token_id: TokenId,

    /// Position size (positive for long, negative for short)
    pub size: f64,

    /// Average entry price
    pub entry_price: f64,

    /// Current market price
    pub current_price: f64,

    /// Unrealized P&L
    pub unrealized_pnl: f64,

    /// Realized P&L
    pub realized_pnl: f64,

    /// Position opened timestamp
    pub opened_at: i64,

    /// Position last updated timestamp
    pub updated_at: i64,
}

impl Position {
    /// Calculate current unrealized P&L
    pub fn calculate_unrealized_pnl(&self, current_price: f64) -> f64 {
        (current_price - self.entry_price) * self.size
    }

    /// Check if position is long
    pub fn is_long(&self) -> bool {
        self.size > 0.0
    }

    /// Check if position is short
    pub fn is_short(&self) -> bool {
        self.size < 0.0
    }

    /// Get absolute position size
    pub fn abs_size(&self) -> f64 {
        self.size.abs()
    }
}

/// Arbitrage opportunity detected
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    /// Market identifier
    pub market_id: MarketId,

    /// Token identifier
    pub token_id: TokenId,

    /// Best bid price
    pub bid_price: f64,

    /// Best ask price
    pub ask_price: f64,

    /// Profit margin percentage (0.0-1.0)
    pub profit_margin: f64,

    /// Maximum tradeable size
    pub max_size: f64,

    /// Expected profit in USDC
    pub expected_profit: f64,

    /// Detection timestamp
    pub detected_at: i64,
}

impl ArbitrageOpportunity {
    /// Create a new arbitrage opportunity
    pub fn new(
        market_id: MarketId,
        token_id: TokenId,
        bid_price: f64,
        ask_price: f64,
        max_size: f64,
    ) -> Option<Self> {
        // Verify arbitrage exists (bid > ask)
        if bid_price <= ask_price {
            return None;
        }

        // Calculate profit margin
        let profit_margin = (bid_price - ask_price) / ask_price;

        // Calculate expected profit
        let expected_profit = (bid_price - ask_price) * max_size;

        Some(Self {
            market_id,
            token_id,
            bid_price,
            ask_price,
            profit_margin,
            max_size,
            expected_profit,
            detected_at: chrono::Utc::now().timestamp_millis(),
        })
    }

    /// Check if opportunity meets minimum profit threshold
    pub fn meets_threshold(&self, min_profit_margin: f64) -> bool {
        self.profit_margin >= min_profit_margin
    }
}

/// Trade execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,

    /// Entry order ID
    pub entry_order_id: Option<String>,

    /// Take profit order ID
    pub take_profit_order_id: Option<String>,

    /// Stop loss order ID
    pub stop_loss_order_id: Option<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution timestamp
    pub executed_at: i64,
}

impl ExecutionResult {
    /// Create successful result
    pub fn success(
        entry_id: String,
        take_profit_id: Option<String>,
        stop_loss_id: Option<String>,
    ) -> Self {
        Self {
            success: true,
            entry_order_id: Some(entry_id),
            take_profit_order_id: take_profit_id,
            stop_loss_order_id: stop_loss_id,
            error: None,
            executed_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create failure result
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            entry_order_id: None,
            take_profit_order_id: None,
            stop_loss_order_id: None,
            error: Some(error),
            executed_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_unrealized_pnl() {
        let position = Position {
            market_id: MarketId("market1".to_string()),
            token_id: TokenId("token1".to_string()),
            size: 100.0,
            entry_price: 0.70,
            current_price: 0.75,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            opened_at: 0,
            updated_at: 0,
        };

        let pnl = position.calculate_unrealized_pnl(0.75);
        assert!((pnl - 5.0).abs() < 0.0001); // (0.75 - 0.70) * 100
    }

    #[test]
    fn test_arbitrage_opportunity_creation() {
        let opportunity = ArbitrageOpportunity::new(
            MarketId("market1".to_string()),
            TokenId("token1".to_string()),
            0.75,
            0.70,
            100.0,
        );

        assert!(opportunity.is_some());
        let opp = opportunity.unwrap();
        assert!((opp.expected_profit - 5.0).abs() < 0.0001);
        assert!((opp.profit_margin - 0.0714).abs() < 0.001);
    }

    #[test]
    fn test_arbitrage_opportunity_no_profit() {
        let opportunity = ArbitrageOpportunity::new(
            MarketId("market1".to_string()),
            TokenId("token1".to_string()),
            0.70,
            0.75,
            100.0,
        );

        assert!(opportunity.is_none());
    }

    #[test]
    fn test_arbitrage_meets_threshold() {
        let opportunity = ArbitrageOpportunity::new(
            MarketId("market1".to_string()),
            TokenId("token1".to_string()),
            0.75,
            0.70,
            100.0,
        ).unwrap();

        assert!(opportunity.meets_threshold(0.02)); // 2% threshold
        assert!(!opportunity.meets_threshold(0.10)); // 10% threshold
    }
}
