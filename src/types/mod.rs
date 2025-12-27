//! Core type definitions for Polymarket HFT bot
//!
//! This module contains all type definitions used throughout the application.
//! Following the types-first approach from the AI Implementation Roadmap.

pub mod market;
pub mod order;
pub mod trade;
pub mod config;

pub use market::*;
pub use order::*;
pub use trade::*;
pub use config::*;
