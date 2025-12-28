//! Trading strategies
//!
//! Different trading strategies for Polymarket.

pub mod crypto_updown;
pub mod binary_arbitrage;

pub use crypto_updown::{
    CryptoAsset, CryptoUpDownConfig, CryptoUpDownFetcher, CryptoUpDownMarket, Timeframe,
};

pub use binary_arbitrage::{
    ArbitrageSide, BinaryArbitrageConfig, BinaryArbitrageDetector, BinaryArbitrageOpportunity,
};
