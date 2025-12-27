//! Order type definitions
//!
//! Defines all order-related data structures for trading.

use serde::{Deserialize, Serialize};
use super::TokenId;

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    /// Buy order
    BUY,
    /// Sell order
    SELL,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    /// Good-til-cancelled limit order
    GTC,
    /// Fill-or-kill
    FOK,
    /// Immediate-or-cancel
    IOC,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderStatus {
    /// Order is open and not yet filled
    OPEN,
    /// Order is partially filled
    PARTIAL,
    /// Order is completely filled
    FILLED,
    /// Order was cancelled
    CANCELLED,
    /// Order was rejected
    REJECTED,
}

/// Parameters for creating a new order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderParams {
    /// Token ID to trade
    pub token_id: TokenId,

    /// Buy or sell
    pub side: OrderSide,

    /// Order type
    #[serde(rename = "type")]
    pub order_type: OrderType,

    /// Limit price (0.0-1.0)
    pub price: f64,

    /// Size in shares
    pub size: f64,

    /// Expiration timestamp (Unix timestamp in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<i64>,

    /// Nonce for unique order identification
    pub nonce: u64,

    /// Client order ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

/// Response after order creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    /// Unique order ID assigned by exchange
    pub order_id: String,

    /// Current order status
    pub status: OrderStatus,

    /// Token ID
    pub token_id: TokenId,

    /// Order side
    pub side: OrderSide,

    /// Limit price
    pub price: f64,

    /// Original size
    pub size: f64,

    /// Filled size
    pub filled_size: f64,

    /// Remaining size
    pub remaining_size: f64,

    /// Creation timestamp
    pub created_at: i64,

    /// Last update timestamp
    pub updated_at: i64,
}

impl OrderResponse {
    /// Check if order is completely filled
    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::FILLED
    }

    /// Check if order is still active
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::OPEN | OrderStatus::PARTIAL)
    }

    /// Get fill percentage (0.0-1.0)
    pub fn fill_percentage(&self) -> f64 {
        if self.size == 0.0 {
            0.0
        } else {
            self.filled_size / self.size
        }
    }
}

/// Three-order execution strategy (entry + take profit + stop loss)
#[derive(Debug, Clone)]
pub struct ThreeOrderStrategy {
    /// Entry order parameters
    pub entry: CreateOrderParams,

    /// Take profit order parameters
    pub take_profit: CreateOrderParams,

    /// Stop loss order parameters
    pub stop_loss: CreateOrderParams,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_is_filled() {
        let order = OrderResponse {
            order_id: "123".to_string(),
            status: OrderStatus::FILLED,
            token_id: TokenId("token1".to_string()),
            side: OrderSide::BUY,
            price: 0.75,
            size: 100.0,
            filled_size: 100.0,
            remaining_size: 0.0,
            created_at: 0,
            updated_at: 0,
        };

        assert!(order.is_filled());
        assert!(!order.is_active());
    }

    #[test]
    fn test_order_fill_percentage() {
        let order = OrderResponse {
            order_id: "123".to_string(),
            status: OrderStatus::PARTIAL,
            token_id: TokenId("token1".to_string()),
            side: OrderSide::BUY,
            price: 0.75,
            size: 100.0,
            filled_size: 75.0,
            remaining_size: 25.0,
            created_at: 0,
            updated_at: 0,
        };

        assert_eq!(order.fill_percentage(), 0.75);
        assert!(order.is_active());
    }
}
