//! Optimistic nonce manager for Polymarket CLOB
//!
//! **Tier 1 Optimization:** Eliminates 100ms API call per order by tracking nonce locally.
//!
//! # Performance Impact
//! - **Before:** 100ms API call to fetch current nonce
//! - **After:** <1μs atomic increment
//! - **Savings:** ~100ms per order (200ms for 2-order arbitrage)
//!
//! # How It Works
//! 1. Initialize with current on-chain nonce (one-time API call)
//! 2. Atomically increment local nonce for each order (no API call)
//! 3. Handle nonce conflicts by resetting to server value + 1
//!
//! # Thread Safety
//! Uses AtomicU64 for lock-free concurrent access.

use std::sync::atomic::{AtomicU64, Ordering};
use tracing;

/// Optimistic nonce manager with atomic operations
///
/// Tracks the next nonce to use without requiring API calls.
/// Thread-safe for concurrent order creation.
#[derive(Debug)]
pub struct NonceManager {
    /// Current nonce (atomic for thread-safety)
    current_nonce: AtomicU64,
}

impl NonceManager {
    /// Create a new nonce manager (starts at 0)
    ///
    /// Must call `initialize()` with actual on-chain nonce before use.
    pub fn new() -> Self {
        Self {
            current_nonce: AtomicU64::new(0),
        }
    }

    /// Create nonce manager with starting value
    pub fn with_nonce(starting_nonce: u64) -> Self {
        Self {
            current_nonce: AtomicU64::new(starting_nonce),
        }
    }

    /// Initialize with current on-chain nonce
    ///
    /// This should be called once at startup after fetching the
    /// current nonce from the CLOB API.
    pub fn initialize(&self, nonce: u64) {
        self.current_nonce.store(nonce, Ordering::SeqCst);
        tracing::info!("Nonce manager initialized at {}", nonce);
    }

    /// Get next nonce (optimistic increment, no API call)
    ///
    /// **Performance:** <1μs vs 100ms API call
    ///
    /// This is the main optimization: instead of calling the API
    /// to get the current nonce, we atomically increment our local counter.
    ///
    /// # Example
    /// ```rust,ignore
    /// let nonce = manager.next_nonce(); // ~1 nanosecond
    /// // vs
    /// let nonce = api.fetch_nonce().await?; // ~100 milliseconds
    /// ```
    pub fn next_nonce(&self) -> u64 {
        self.current_nonce.fetch_add(1, Ordering::SeqCst)
    }

    /// Handle nonce conflict (reset to server value + 1)
    ///
    /// If an order is rejected due to nonce conflict, the server will
    /// return the expected nonce. Call this method to reset our local
    /// counter to avoid future conflicts.
    ///
    /// # Arguments
    /// * `server_nonce` - The nonce value the server expects
    pub fn handle_conflict(&self, server_nonce: u64) {
        let current = self.current_nonce.load(Ordering::Acquire);

        if server_nonce >= current {
            // Server is ahead, update to server value + 1
            self.current_nonce.store(server_nonce + 1, Ordering::SeqCst);
            tracing::warn!(
                "Nonce conflict detected: local={}, server={}, reset to {}",
                current,
                server_nonce,
                server_nonce + 1
            );
        } else {
            // We're ahead of server (shouldn't happen often)
            tracing::debug!(
                "Nonce conflict: local={} ahead of server={}",
                current,
                server_nonce
            );
        }
    }

    /// Get current nonce without incrementing
    ///
    /// Useful for debugging and monitoring.
    pub fn current(&self) -> u64 {
        self.current_nonce.load(Ordering::Acquire)
    }

    /// Manually set nonce (use with caution)
    ///
    /// This is primarily for testing or recovery scenarios.
    pub fn set_nonce(&self, nonce: u64) {
        self.current_nonce.store(nonce, Ordering::SeqCst);
        tracing::warn!("Nonce manually set to {}", nonce);
    }

    /// Reset nonce to 0 (for testing)
    #[cfg(test)]
    pub fn reset(&self) {
        self.current_nonce.store(0, Ordering::SeqCst);
    }
}

impl Default for NonceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_increment() {
        let manager = NonceManager::with_nonce(100);

        assert_eq!(manager.next_nonce(), 100);
        assert_eq!(manager.next_nonce(), 101);
        assert_eq!(manager.next_nonce(), 102);
        assert_eq!(manager.current(), 103);
    }

    #[test]
    fn test_initialize() {
        let manager = NonceManager::new();
        assert_eq!(manager.current(), 0);

        manager.initialize(42);
        assert_eq!(manager.current(), 42);
        assert_eq!(manager.next_nonce(), 42);
        assert_eq!(manager.current(), 43);
    }

    #[test]
    fn test_conflict_handling_server_ahead() {
        let manager = NonceManager::with_nonce(100);

        // Use a few nonces
        manager.next_nonce(); // 100
        manager.next_nonce(); // 101

        // Server says we should be at 105
        manager.handle_conflict(105);

        // Should reset to 106
        assert_eq!(manager.current(), 106);
        assert_eq!(manager.next_nonce(), 106);
    }

    #[test]
    fn test_conflict_handling_local_ahead() {
        let manager = NonceManager::with_nonce(100);

        // Use many nonces
        for _ in 0..10 {
            manager.next_nonce();
        }

        // Server is behind (shouldn't reset)
        manager.handle_conflict(95);

        // Should stay at current value
        assert_eq!(manager.current(), 110);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(NonceManager::with_nonce(0));
        let mut handles = vec![];

        // Spawn 10 threads that each increment 100 times
        for _ in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    manager_clone.next_nonce();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have incremented 1000 times total
        assert_eq!(manager.current(), 1000);
    }

    #[test]
    fn test_set_nonce() {
        let manager = NonceManager::with_nonce(100);

        manager.set_nonce(500);
        assert_eq!(manager.current(), 500);
        assert_eq!(manager.next_nonce(), 500);
    }
}
