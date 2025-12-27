//! Polymarket HFT Bot
//!
//! High-frequency trading bot for Polymarket with SIMD-optimized arbitrage detection.
//!
//! ## Architecture
//!
//! The bot is structured into several modules:
//! - `types`: Core type definitions
//! - `core`: Business logic (arbitrage, execution, risk)
//! - `services`: External integrations (Polymarket API, WebSocket)
//! - `config`: Configuration management
//! - `utils`: Utility functions
//!
//! ## Performance Features
//!
//! - SIMD vectorization for arbitrage detection (4x speedup)
//! - Lock-free atomic data structures
//! - Async/await with Tokio runtime
//! - Zero-copy deserialization where possible
//!
//! ## Safety
//!
//! - Circuit breaker pattern for risk management
//! - Comprehensive error handling
//! - Dry-run mode for testing
//! - Position limits and daily loss caps

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod types;
pub mod core;
pub mod services;
pub mod config;
pub mod utils;

pub use types::*;
