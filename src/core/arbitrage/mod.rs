//! Arbitrage detection module
//!
//! Provides both scalar and SIMD-optimized arbitrage detection.

pub mod detector;
pub mod simd_detector;

pub use detector::{ArbitrageConfig, ScalarArbitrageDetector};
pub use simd_detector::SimdArbitrageDetector;
