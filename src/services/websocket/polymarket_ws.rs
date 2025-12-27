//! Polymarket-specific WebSocket client
//!
//! Handles Polymarket CLOB WebSocket protocol including:
//! - Orderbook subscriptions
//! - Incremental updates
//! - Market-specific channels
//!
//! # Message Format
//! Polymarket uses JSON messages with the following structure:
//! ```json
//! {
//!   "type": "orderbook",
//!   "market_id": "TRUMP-WIN",
//!   "token_id": "YES",
//!   "bids": [[price, size], ...],
//!   "asks": [[price, size], ...]
//! }
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tokio::net::TcpStream;
use futures_util::SinkExt;
use tracing;

use crate::types::{MarketId, TokenId, OrderBook, OrderBookEntry};
use super::manager::WebSocketManager;

/// Polymarket WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolymarketMessage {
    /// Orderbook snapshot or update
    Orderbook(OrderbookUpdate),

    /// Trade execution
    Trade(TradeUpdate),

    /// Subscription confirmation
    Subscribed(SubscriptionConfirm),

    /// Error message
    Error(ErrorMessage),
}

/// Orderbook update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookUpdate {
    /// Market identifier
    pub market_id: String,

    /// Token identifier (YES/NO)
    pub token_id: String,

    /// Bids: [price, size]
    pub bids: Vec<(f64, f64)>,

    /// Asks: [price, size]
    pub asks: Vec<(f64, f64)>,

    /// Timestamp
    #[serde(default)]
    pub timestamp: i64,
}

impl OrderbookUpdate {
    /// Convert to internal OrderBook type
    pub fn to_order_book(&self) -> OrderBook {
        OrderBook {
            token_id: TokenId(self.token_id.clone()),
            bids: self.bids
                .iter()
                .map(|(price, size)| OrderBookEntry {
                    price: *price,
                    size: *size,
                    timestamp: Some(self.timestamp),
                })
                .collect(),
            asks: self.asks
                .iter()
                .map(|(price, size)| OrderBookEntry {
                    price: *price,
                    size: *size,
                    timestamp: Some(self.timestamp),
                })
                .collect(),
            timestamp: self.timestamp,
        }
    }
}

/// Trade update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeUpdate {
    pub market_id: String,
    pub token_id: String,
    pub price: f64,
    pub size: f64,
    pub side: String, // "BUY" or "SELL"
    pub timestamp: i64,
}

/// Subscription confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionConfirm {
    pub channel: String,
    pub market_id: Option<String>,
}

/// Error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
}

/// Polymarket-specific orderbook update
#[derive(Debug, Clone)]
pub struct PolymarketOrderbookUpdate {
    /// Market identifier
    pub market_id: MarketId,
    /// Token identifier
    pub token_id: TokenId,
    /// Order book snapshot
    pub order_book: OrderBook,
    /// Update timestamp
    pub timestamp: i64,
}

/// Polymarket WebSocket client
pub struct PolymarketWebSocket {
    manager: WebSocketManager<PolymarketMessage>,
    subscriptions: Vec<(MarketId, TokenId)>,
}

impl PolymarketWebSocket {
    /// Create a new Polymarket WebSocket client
    ///
    /// # Arguments
    /// * `url` - WebSocket URL (e.g., "wss://clob.polymarket.com/ws")
    /// * `markets` - Markets to subscribe to
    /// * `update_tx` - Channel to send orderbook updates
    pub fn new(
        url: String,
        markets: Vec<(MarketId, TokenId)>,
    ) -> (Self, mpsc::Receiver<PolymarketMessage>) {
        let (tx, rx) = mpsc::channel(1000);

        let manager = WebSocketManager::new(url, tx);

        (
            Self {
                manager,
                subscriptions: markets,
            },
            rx,
        )
    }

    /// Start the WebSocket client
    pub async fn start(self) -> Result<()> {
        self.manager.start().await
    }

    /// Send subscription messages for configured markets
    async fn send_subscriptions(
        &self,
        stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<()> {
        for (market_id, token_id) in &self.subscriptions {
            let subscription = serde_json::json!({
                "type": "subscribe",
                "channel": "orderbook",
                "market_id": market_id.0,
                "token_id": token_id.0,
            });

            let msg = Message::Text(subscription.to_string());
            stream.send(msg).await?;

            tracing::info!("Subscribed to {}/{}", market_id.0, token_id.0);
        }

        Ok(())
    }
}

/// Process Polymarket messages and extract orderbook updates
pub fn process_message(msg: PolymarketMessage) -> Option<PolymarketOrderbookUpdate> {
    match msg {
        PolymarketMessage::Orderbook(update) => {
            Some(PolymarketOrderbookUpdate {
                market_id: MarketId(update.market_id.clone()),
                token_id: TokenId(update.token_id.clone()),
                order_book: update.to_order_book(),
                timestamp: update.timestamp,
            })
        }
        PolymarketMessage::Subscribed(confirm) => {
            tracing::info!("Subscription confirmed: {:?}", confirm);
            None
        }
        PolymarketMessage::Error(error) => {
            tracing::error!("WebSocket error: {} - {}", error.code, error.message);
            None
        }
        PolymarketMessage::Trade(trade) => {
            tracing::debug!("Trade: {}/{} @ {} size {}",
                trade.market_id, trade.token_id, trade.price, trade.size);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_update_conversion() {
        let update = OrderbookUpdate {
            market_id: "TRUMP-WIN".to_string(),
            token_id: "YES".to_string(),
            bids: vec![(0.75, 100.0), (0.74, 200.0)],
            asks: vec![(0.76, 150.0), (0.77, 250.0)],
            timestamp: 1000,
        };

        let order_book = update.to_order_book();

        assert_eq!(order_book.bids.len(), 2);
        assert_eq!(order_book.asks.len(), 2);
        assert_eq!(order_book.bids[0].price, 0.75);
        assert_eq!(order_book.bids[0].size, 100.0);
        assert_eq!(order_book.asks[0].price, 0.76);
        assert_eq!(order_book.asks[0].size, 150.0);
    }

    #[test]
    fn test_process_orderbook_message() {
        let msg = PolymarketMessage::Orderbook(OrderbookUpdate {
            market_id: "TRUMP-WIN".to_string(),
            token_id: "YES".to_string(),
            bids: vec![(0.75, 100.0)],
            asks: vec![(0.76, 150.0)],
            timestamp: 1000,
        });

        let result = process_message(msg);
        assert!(result.is_some());

        let update = result.unwrap();
        assert_eq!(update.market_id.0, "TRUMP-WIN");
        assert_eq!(update.token_id.0, "YES");
        assert_eq!(update.order_book.bids.len(), 1);
    }

    #[test]
    fn test_process_subscription_confirm() {
        let msg = PolymarketMessage::Subscribed(SubscriptionConfirm {
            channel: "orderbook".to_string(),
            market_id: Some("TRUMP-WIN".to_string()),
        });

        let result = process_message(msg);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_error_message() {
        let msg = PolymarketMessage::Error(ErrorMessage {
            code: "INVALID_SUBSCRIPTION".to_string(),
            message: "Market not found".to_string(),
        });

        let result = process_message(msg);
        assert!(result.is_none());
    }
}
