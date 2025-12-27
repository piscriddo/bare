//! Circuit breaker for risk management
//!
//! Implements a thread-safe circuit breaker using atomic operations for
//! lock-free concurrency. Prevents excessive losses and manages risk limits.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use crate::types::RiskConfig;

/// Circuit breaker state for risk management
///
/// Uses lock-free atomic operations for high-performance concurrent access.
/// Tracks daily losses, position counts, and error rates.
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Whether the circuit breaker is tripped (trading halted)
    tripped: AtomicBool,

    /// Number of consecutive errors
    consecutive_errors: AtomicU32,

    /// Daily loss in cents (u64 for atomic operations)
    /// Store as cents to avoid floating-point atomics
    daily_loss_cents: AtomicU64,

    /// Number of currently open positions
    open_positions: AtomicU32,

    /// Configuration
    config: RiskConfig,

    /// Last reset time (protected by RwLock for infrequent writes)
    last_reset: RwLock<Instant>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: RiskConfig) -> Self {
        Self {
            tripped: AtomicBool::new(false),
            consecutive_errors: AtomicU32::new(0),
            daily_loss_cents: AtomicU64::new(0),
            open_positions: AtomicU32::new(0),
            config,
            last_reset: RwLock::new(Instant::now()),
        }
    }

    /// Check if trading is allowed
    ///
    /// Returns true if the circuit breaker is not tripped.
    /// This is a fast atomic read operation.
    pub fn can_execute(&self) -> bool {
        !self.tripped.load(Ordering::Acquire)
    }

    /// Trip the circuit breaker (halt trading)
    pub fn trip(&self) {
        self.tripped.store(true, Ordering::Release);
        tracing::error!("🚨 Circuit breaker TRIPPED - Trading halted!");
    }

    /// Reset the circuit breaker (resume trading)
    pub fn reset(&self) {
        self.tripped.store(false, Ordering::Release);
        tracing::info!("✅ Circuit breaker RESET - Trading resumed");
    }

    /// Check if circuit breaker should be tripped based on current state
    fn check_and_trip(&self) -> bool {
        // Check daily loss limit
        let daily_loss = self.daily_loss_cents.load(Ordering::Acquire) as f64 / 100.0;
        if daily_loss >= self.config.max_daily_loss {
            tracing::error!(
                "Daily loss limit exceeded: ${:.2} >= ${:.2}",
                daily_loss,
                self.config.max_daily_loss
            );
            self.trip();
            return true;
        }

        // Check max open positions
        let positions = self.open_positions.load(Ordering::Acquire);
        if positions as usize > self.config.max_open_positions {
            tracing::error!(
                "Max positions exceeded: {} > {}",
                positions,
                self.config.max_open_positions
            );
            self.trip();
            return true;
        }

        // Check consecutive errors
        let errors = self.consecutive_errors.load(Ordering::Acquire);
        if errors as usize >= self.config.max_consecutive_errors {
            tracing::error!("Too many consecutive errors: {}", errors);
            self.trip();
            return true;
        }

        false
    }

    /// Record a trade and update P&L
    ///
    /// Updates daily loss atomically. Positive P&L reduces loss, negative increases it.
    pub fn record_trade(&self, pnl: f64) -> Result<(), String> {
        if !self.can_execute() {
            return Err("Circuit breaker is tripped".to_string());
        }

        // Convert to cents for atomic storage
        let pnl_cents = (pnl * 100.0) as i64;

        // Atomic update of daily loss
        if pnl < 0.0 {
            // Loss: add absolute value
            self.daily_loss_cents.fetch_add(pnl_cents.unsigned_abs(), Ordering::AcqRel);
        } else {
            // Profit: subtract (but don't go below 0)
            let current = self.daily_loss_cents.load(Ordering::Acquire);
            let new_loss = current.saturating_sub(pnl_cents.unsigned_abs());
            self.daily_loss_cents.store(new_loss, Ordering::Release);
        }

        // Reset consecutive errors on successful trade
        self.consecutive_errors.store(0, Ordering::Release);

        // Check if limits exceeded
        self.check_and_trip();

        Ok(())
    }

    /// Record an error
    pub fn record_error(&self) {
        let errors = self.consecutive_errors.fetch_add(1, Ordering::AcqRel);
        tracing::warn!("Consecutive errors: {}", errors + 1);

        // Check if should trip
        self.check_and_trip();
    }

    /// Increment open positions
    pub fn open_position(&self) -> Result<(), String> {
        if !self.can_execute() {
            return Err("Circuit breaker is tripped".to_string());
        }

        let positions = self.open_positions.fetch_add(1, Ordering::AcqRel);
        tracing::debug!("Opened position (total: {})", positions + 1);

        // Check if should trip
        if self.check_and_trip() {
            // Rollback position increment if we tripped
            self.open_positions.fetch_sub(1, Ordering::AcqRel);
            return Err("Position limit would be exceeded".to_string());
        }

        Ok(())
    }

    /// Decrement open positions
    pub fn close_position(&self) {
        let positions = self.open_positions.fetch_sub(1, Ordering::AcqRel);
        tracing::debug!("Closed position (remaining: {})", positions.saturating_sub(1));
    }

    /// Get current daily loss
    pub fn daily_loss(&self) -> f64 {
        self.daily_loss_cents.load(Ordering::Acquire) as f64 / 100.0
    }

    /// Get current open positions count
    pub fn positions(&self) -> u32 {
        self.open_positions.load(Ordering::Acquire)
    }

    /// Get consecutive errors count
    pub fn errors(&self) -> u32 {
        self.consecutive_errors.load(Ordering::Acquire)
    }

    /// Reset daily counters (call at start of new trading day)
    pub fn reset_daily(&self) {
        self.daily_loss_cents.store(0, Ordering::Release);
        self.consecutive_errors.store(0, Ordering::Release);
        *self.last_reset.write() = Instant::now();
        tracing::info!("Daily counters reset");
    }

    /// Check if daily reset is needed (based on time elapsed)
    pub fn check_daily_reset(&self, reset_interval: Duration) -> bool {
        let last_reset = *self.last_reset.read();
        last_reset.elapsed() >= reset_interval
    }

    /// Auto-reset if cooldown period elapsed and conditions are met
    pub fn auto_reset(&self, cooldown: Duration) -> bool {
        if !self.can_execute() {
            let last_reset = *self.last_reset.read();
            if last_reset.elapsed() >= cooldown {
                // Check if safe to reset
                let daily_loss = self.daily_loss();
                if daily_loss < self.config.max_daily_loss * 0.9 {
                    self.reset();
                    return true;
                }
            }
        }
        false
    }
}

/// Thread-safe wrapper for circuit breaker
pub type SharedCircuitBreaker = Arc<CircuitBreaker>;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> RiskConfig {
        RiskConfig {
            max_daily_loss: 100.0,
            max_position_size: 50.0,
            max_open_positions: 5,
            min_usdc_balance: 10.0,
            min_matic_balance: 1.0,
            max_consecutive_errors: 10,
        }
    }

    #[test]
    fn test_can_execute_initial() {
        let cb = CircuitBreaker::new(create_test_config());
        assert!(cb.can_execute());
    }

    #[test]
    fn test_trip_and_reset() {
        let cb = CircuitBreaker::new(create_test_config());

        assert!(cb.can_execute());

        cb.trip();
        assert!(!cb.can_execute());

        cb.reset();
        assert!(cb.can_execute());
    }

    #[test]
    fn test_record_loss() {
        let cb = CircuitBreaker::new(create_test_config());

        // Record a $50 loss
        cb.record_trade(-50.0).unwrap();
        assert_eq!(cb.daily_loss(), 50.0);

        // Record another $30 loss
        cb.record_trade(-30.0).unwrap();
        assert_eq!(cb.daily_loss(), 80.0);
    }

    #[test]
    fn test_record_profit() {
        let cb = CircuitBreaker::new(create_test_config());

        // Record a $50 loss
        cb.record_trade(-50.0).unwrap();
        assert_eq!(cb.daily_loss(), 50.0);

        // Record a $20 profit
        cb.record_trade(20.0).unwrap();
        assert_eq!(cb.daily_loss(), 30.0);
    }

    #[test]
    fn test_daily_loss_limit() {
        let cb = CircuitBreaker::new(create_test_config());

        // Record $90 loss (below limit)
        cb.record_trade(-90.0).unwrap();
        assert!(cb.can_execute());

        // Record $20 loss (exceeds $100 limit)
        cb.record_trade(-20.0).unwrap();
        assert!(!cb.can_execute(), "Circuit breaker should trip on loss limit");
    }

    #[test]
    fn test_position_tracking() {
        let cb = CircuitBreaker::new(create_test_config());

        assert_eq!(cb.positions(), 0);

        cb.open_position().unwrap();
        assert_eq!(cb.positions(), 1);

        cb.open_position().unwrap();
        assert_eq!(cb.positions(), 2);

        cb.close_position();
        assert_eq!(cb.positions(), 1);

        cb.close_position();
        assert_eq!(cb.positions(), 0);
    }

    #[test]
    fn test_max_positions_limit() {
        let cb = CircuitBreaker::new(create_test_config());

        // Open 5 positions (at limit)
        for _ in 0..5 {
            cb.open_position().unwrap();
        }

        assert_eq!(cb.positions(), 5);
        assert!(cb.can_execute(), "Should still be able to execute at limit");

        // Try to open 6th position (should fail and trip)
        let result = cb.open_position();
        assert!(result.is_err(), "Should not allow exceeding max positions");
        assert!(!cb.can_execute(), "Circuit breaker should trip");
        assert_eq!(cb.positions(), 5, "Position count should not increase after failure");
    }

    #[test]
    fn test_consecutive_errors() {
        let cb = CircuitBreaker::new(create_test_config());

        // Record 9 errors (below limit)
        for _ in 0..9 {
            cb.record_error();
        }
        assert!(cb.can_execute());

        // 10th error should trip
        cb.record_error();
        assert!(!cb.can_execute(), "Circuit breaker should trip after 10 errors");
    }

    #[test]
    fn test_error_reset_on_trade() {
        let cb = CircuitBreaker::new(create_test_config());

        // Record some errors
        cb.record_error();
        cb.record_error();
        assert_eq!(cb.errors(), 2);

        // Successful trade resets errors
        cb.record_trade(10.0).unwrap();
        assert_eq!(cb.errors(), 0);
    }

    #[test]
    fn test_daily_reset() {
        let cb = CircuitBreaker::new(create_test_config());

        // Record loss and errors
        cb.record_trade(-50.0).unwrap();
        cb.record_error();

        assert_eq!(cb.daily_loss(), 50.0);
        assert_eq!(cb.errors(), 1);

        // Reset daily counters
        cb.reset_daily();

        assert_eq!(cb.daily_loss(), 0.0);
        assert_eq!(cb.errors(), 0);
    }

    #[test]
    fn test_profit_doesnt_go_negative() {
        let cb = CircuitBreaker::new(create_test_config());

        // Record $20 loss
        cb.record_trade(-20.0).unwrap();
        assert_eq!(cb.daily_loss(), 20.0);

        // Record $50 profit (more than loss)
        cb.record_trade(50.0).unwrap();
        assert_eq!(cb.daily_loss(), 0.0, "Daily loss should not go negative");
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let cb = Arc::new(CircuitBreaker::new(create_test_config()));

        let mut handles = vec![];

        // Spawn 10 threads that each record 5 losing trades
        for i in 0..10 {
            let cb_clone = Arc::clone(&cb);
            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    let _ = cb_clone.record_trade(-1.0);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // All 50 losing trades processed (50 * $1 = $50 loss)
        assert_eq!(cb.daily_loss(), 50.0);
    }
}
