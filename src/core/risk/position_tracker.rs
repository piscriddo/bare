//! Position tracking for P&L monitoring
//!
//! Tracks open positions and calculates real-time profit/loss.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::types::{MarketId, Position, TokenId};

/// Position tracker for managing open positions
///
/// Thread-safe position tracking using RwLock for concurrent access.
/// Tracks positions per market/token and calculates aggregate P&L.
#[derive(Debug)]
pub struct PositionTracker {
    /// Map of (market_id, token_id) -> Position
    positions: RwLock<HashMap<(MarketId, TokenId), Position>>,
}

impl PositionTracker {
    /// Create a new position tracker
    pub fn new() -> Self {
        Self {
            positions: RwLock::new(HashMap::new()),
        }
    }

    /// Add or update a position
    pub fn update_position(&self, market_id: MarketId, token_id: TokenId, position: Position) {
        let mut positions = self.positions.write();
        positions.insert((market_id, token_id), position);
    }

    /// Get a position
    pub fn get_position(&self, market_id: &MarketId, token_id: &TokenId) -> Option<Position> {
        let positions = self.positions.read();
        positions.get(&(market_id.clone(), token_id.clone())).cloned()
    }

    /// Remove a position (when closed)
    pub fn remove_position(&self, market_id: &MarketId, token_id: &TokenId) -> Option<Position> {
        let mut positions = self.positions.write();
        positions.remove(&(market_id.clone(), token_id.clone()))
    }

    /// Get total number of open positions
    pub fn position_count(&self) -> usize {
        let positions = self.positions.read();
        positions.len()
    }

    /// Calculate total unrealized P&L across all positions
    pub fn total_unrealized_pnl(&self, current_prices: &HashMap<(MarketId, TokenId), f64>) -> f64 {
        let positions = self.positions.read();

        positions
            .iter()
            .map(|((market_id, token_id), position)| {
                if let Some(&current_price) = current_prices.get(&(market_id.clone(), token_id.clone())) {
                    position.calculate_unrealized_pnl(current_price)
                } else {
                    0.0 // No current price available
                }
            })
            .sum()
    }

    /// Get total position size (in dollars)
    pub fn total_exposure(&self) -> f64 {
        let positions = self.positions.read();

        positions
            .values()
            .map(|position| position.entry_price * position.abs_size())
            .sum()
    }

    /// Get all positions
    pub fn get_all_positions(&self) -> Vec<((MarketId, TokenId), Position)> {
        let positions = self.positions.read();
        positions
            .iter()
            .map(|((m, t), p)| ((m.clone(), t.clone()), p.clone()))
            .collect()
    }

    /// Clear all positions
    pub fn clear(&self) {
        let mut positions = self.positions.write();
        positions.clear();
    }

    /// Check if position exists
    pub fn has_position(&self, market_id: &MarketId, token_id: &TokenId) -> bool {
        let positions = self.positions.read();
        positions.contains_key(&(market_id.clone(), token_id.clone()))
    }

    /// Get positions for a specific market
    pub fn positions_for_market(&self, market_id: &MarketId) -> Vec<(TokenId, Position)> {
        let positions = self.positions.read();
        positions
            .iter()
            .filter(|((m, _), _)| m == market_id)
            .map(|((_, t), p)| (t.clone(), p.clone()))
            .collect()
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper for position tracker
pub type SharedPositionTracker = Arc<PositionTracker>;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_position(size: f64, entry_price: f64) -> Position {
        Position {
            market_id: MarketId("test-market".to_string()),
            token_id: TokenId("test-token".to_string()),
            size,
            entry_price,
            current_price: entry_price,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            opened_at: 1000,
            updated_at: 1000,
        }
    }

    #[test]
    fn test_add_and_get_position() {
        let tracker = PositionTracker::new();
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());
        let position = create_test_position(100.0, 0.75);

        tracker.update_position(market_id.clone(), token_id.clone(), position.clone());

        let retrieved = tracker.get_position(&market_id, &token_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().size, 100.0);
    }

    #[test]
    fn test_remove_position() {
        let tracker = PositionTracker::new();
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());
        let position = create_test_position(100.0, 0.75);

        tracker.update_position(market_id.clone(), token_id.clone(), position);

        assert_eq!(tracker.position_count(), 1);

        let removed = tracker.remove_position(&market_id, &token_id);
        assert!(removed.is_some());
        assert_eq!(tracker.position_count(), 0);
    }

    #[test]
    fn test_position_count() {
        let tracker = PositionTracker::new();

        assert_eq!(tracker.position_count(), 0);

        tracker.update_position(
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_test_position(100.0, 0.75),
        );

        assert_eq!(tracker.position_count(), 1);

        tracker.update_position(
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_test_position(50.0, 0.60),
        );

        assert_eq!(tracker.position_count(), 2);
    }

    #[test]
    fn test_total_exposure() {
        let tracker = PositionTracker::new();

        // Position 1: 100 shares @ $0.75 = $75
        tracker.update_position(
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_test_position(100.0, 0.75),
        );

        // Position 2: 50 shares @ $0.60 = $30
        tracker.update_position(
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_test_position(50.0, 0.60),
        );

        // Total exposure: $75 + $30 = $105
        assert_eq!(tracker.total_exposure(), 105.0);
    }

    #[test]
    fn test_total_unrealized_pnl() {
        let tracker = PositionTracker::new();

        let market1 = MarketId("m1".to_string());
        let token1 = TokenId("t1".to_string());
        let market2 = MarketId("m2".to_string());
        let token2 = TokenId("t2".to_string());

        // Position 1: Long 100 @ $0.70
        tracker.update_position(market1.clone(), token1.clone(), create_test_position(100.0, 0.70));

        // Position 2: Short 50 @ $0.60
        tracker.update_position(market2.clone(), token2.clone(), create_test_position(-50.0, 0.60));

        // Current prices
        let mut prices = HashMap::new();
        prices.insert((market1.clone(), token1.clone()), 0.75); // Long position up $5
        prices.insert((market2.clone(), token2.clone()), 0.55); // Short position up $2.50

        let total_pnl = tracker.total_unrealized_pnl(&prices);

        // Position 1: (0.75 - 0.70) * 100 = $5
        // Position 2: (0.60 - 0.55) * 50 = $2.50
        // Total: $7.50
        assert!((total_pnl - 7.5).abs() < 0.01, "Expected ~7.5, got {}", total_pnl);
    }

    #[test]
    fn test_has_position() {
        let tracker = PositionTracker::new();
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        assert!(!tracker.has_position(&market_id, &token_id));

        tracker.update_position(
            market_id.clone(),
            token_id.clone(),
            create_test_position(100.0, 0.75),
        );

        assert!(tracker.has_position(&market_id, &token_id));
    }

    #[test]
    fn test_positions_for_market() {
        let tracker = PositionTracker::new();
        let market_id = MarketId("market-1".to_string());

        // Add 2 positions for same market
        tracker.update_position(
            market_id.clone(),
            TokenId("token-1".to_string()),
            create_test_position(100.0, 0.75),
        );

        tracker.update_position(
            market_id.clone(),
            TokenId("token-2".to_string()),
            create_test_position(50.0, 0.60),
        );

        // Add position for different market
        tracker.update_position(
            MarketId("market-2".to_string()),
            TokenId("token-3".to_string()),
            create_test_position(75.0, 0.80),
        );

        let positions = tracker.positions_for_market(&market_id);
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_clear() {
        let tracker = PositionTracker::new();

        tracker.update_position(
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_test_position(100.0, 0.75),
        );

        assert_eq!(tracker.position_count(), 1);

        tracker.clear();

        assert_eq!(tracker.position_count(), 0);
    }

    #[test]
    fn test_update_existing_position() {
        let tracker = PositionTracker::new();
        let market_id = MarketId("market-1".to_string());
        let token_id = TokenId("token-1".to_string());

        // Add initial position
        tracker.update_position(
            market_id.clone(),
            token_id.clone(),
            create_test_position(100.0, 0.75),
        );

        assert_eq!(tracker.position_count(), 1);

        // Update same position
        tracker.update_position(
            market_id.clone(),
            token_id.clone(),
            create_test_position(150.0, 0.80),
        );

        // Should still be 1 position
        assert_eq!(tracker.position_count(), 1);

        // Position should be updated
        let position = tracker.get_position(&market_id, &token_id).unwrap();
        assert_eq!(position.size, 150.0);
        assert_eq!(position.entry_price, 0.80);
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let tracker = Arc::new(PositionTracker::new());

        let mut handles = vec![];

        // Spawn 10 threads that each add positions
        for i in 0..10 {
            let tracker_clone = Arc::clone(&tracker);
            let handle = thread::spawn(move || {
                tracker_clone.update_position(
                    MarketId(format!("market-{}", i)),
                    TokenId(format!("token-{}", i)),
                    create_test_position(100.0, 0.75),
                );
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have 10 positions
        assert_eq!(tracker.position_count(), 10);
    }
}
