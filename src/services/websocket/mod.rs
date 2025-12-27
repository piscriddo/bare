//! WebSocket services with auto-reconnect and Tier 2 optimizations
//!
//! Provides:
//! - Generic WebSocket manager with auto-reconnect
//! - Polymarket-specific WebSocket client
//! - Zero-copy message buffers
//! - TCP_NODELAY optimization

mod manager;
mod polymarket_ws;

pub use manager::WebSocketManager;
pub use polymarket_ws::{
    PolymarketWebSocket,
    PolymarketMessage,
    PolymarketOrderbookUpdate,
    OrderbookUpdate,
    process_message,
};
