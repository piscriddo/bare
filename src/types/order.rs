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
    /// Good-til-date (expires at specific timestamp)
    GTD,
    /// Fill-or-kill (market order, full execution or cancel)
    FOK,
    /// Fill-and-kill (market order, partial fills OK)
    FAK,
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

// ============================================================================
// Polymarket CLOB-specific types (Phase 4)
// ============================================================================

/// Signature type for EIP-712 orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureType {
    /// EIP-712 signature (default)
    #[serde(rename = "0")]
    EIP712 = 0,
    /// EIP-1271 signature (contract wallets)
    #[serde(rename = "1")]
    EIP1271 = 1,
    /// Polymarket proxy signature
    #[serde(rename = "2")]
    PolyProxy = 2,
}

/// Signed order for Polymarket CLOB
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedOrder {
    /// Random salt for uniqueness
    pub salt: String,

    /// Maker address (funder)
    pub maker: String,

    /// Signer address
    pub signer: String,

    /// Taker address (operator)
    pub taker: String,

    /// ERC1155 token ID
    pub token_id: String,

    /// Maximum amount maker is willing to spend (in wei)
    pub maker_amount: String,

    /// Minimum amount taker will pay (in wei)
    pub taker_amount: String,

    /// Unix expiration timestamp
    pub expiration: String,

    /// Maker's exchange nonce
    pub nonce: String,

    /// Fee rate in basis points
    pub fee_rate_bps: String,

    /// Order side (0 = BUY, 1 = SELL)
    pub side: u8,

    /// Signature type
    pub signature_type: u8,

    /// Hex-encoded signature
    pub signature: String,
}

/// Post order wrapper for batch requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostOrder {
    /// Signed order
    pub order: SignedOrder,

    /// Order type (GTC, GTD, FOK, FAK)
    #[serde(rename = "orderType")]
    pub order_type: String,

    /// API key of order owner
    pub owner: String,
}

/// Batch order response from Polymarket CLOB
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOrderResponse {
    /// Whether server-side processing succeeded
    pub success: bool,

    /// Error message if success = false
    #[serde(default)]
    pub error_msg: String,

    /// Order ID (if single order)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,

    /// Order hashes for successful orders in batch
    #[serde(default)]
    pub order_hashes: Vec<String>,

    /// Order status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl BatchOrderResponse {
    /// Check if both orders in arbitrage pair succeeded
    pub fn both_succeeded(&self) -> bool {
        self.success && self.order_hashes.len() >= 2
    }

    /// Check if partial fill occurred (only one order succeeded)
    pub fn is_partial_fill(&self) -> bool {
        self.success && self.order_hashes.len() == 1
    }

    /// Get buy order hash (first order)
    pub fn buy_hash(&self) -> Option<&String> {
        self.order_hashes.get(0)
    }

    /// Get sell order hash (second order)
    pub fn sell_hash(&self) -> Option<&String> {
        self.order_hashes.get(1)
    }
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
