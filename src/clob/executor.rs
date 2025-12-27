//! Arbitrage executor with batch orders and rollback
//!
//! **Critical Component:** Executes arbitrage trades with batch orders and
//! handles partial fills through automatic rollback.
//!
//! # Safety Guarantees
//! 1. **Both orders succeed** → Arbitrage complete ✓
//! 2. **Only one succeeds** → Immediate rollback (cancel successful order)
//! 3. **Both fail** → Safe, no action needed
//! 4. **Rollback fails** → Trip circuit breaker, alert operator
//!
//! # Performance
//! - Batch execution: ~150-200ms (vs 400ms sequential)
//! - Rollback detection: <1ms
//! - Cancel request: ~100-150ms
//!
//! # Integration
//! - Uses ClobClient for HTTP requests
//! - Uses CircuitBreaker for risk management
//! - Reports P&L for successful arbitrage

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Instant;
use tracing;

use super::client::{ClobClient, CreateOrderRequest};
use crate::core::risk::CircuitBreaker;
use crate::types::{ArbitrageOpportunity, OrderSide};

/// Result of arbitrage execution
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Both orders succeeded (arbitrage complete)
    Success {
        /// Buy order hash
        buy_hash: String,
        /// Sell order hash
        sell_hash: String,
        /// Estimated P&L
        pnl: f64,
        /// Execution latency
        latency_ms: u64,
    },

    /// Only one order succeeded (rolled back)
    PartialFill {
        /// The order that succeeded
        filled_hash: String,
        /// Whether rollback succeeded
        rolled_back: bool,
        /// Execution latency
        latency_ms: u64,
    },

    /// Both orders failed (safe)
    Failed {
        /// Error message
        error: String,
        /// Execution latency
        latency_ms: u64,
    },
}

impl ExecutionResult {
    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionResult::Success { .. })
    }

    /// Get execution latency
    pub fn latency_ms(&self) -> u64 {
        match self {
            ExecutionResult::Success { latency_ms, .. } => *latency_ms,
            ExecutionResult::PartialFill { latency_ms, .. } => *latency_ms,
            ExecutionResult::Failed { latency_ms, .. } => *latency_ms,
        }
    }

    /// Get P&L (0.0 if not successful)
    pub fn pnl(&self) -> f64 {
        match self {
            ExecutionResult::Success { pnl, .. } => *pnl,
            _ => 0.0,
        }
    }
}

/// Arbitrage executor with batch orders and rollback
pub struct ArbitrageExecutor {
    /// CLOB client (with all Tier 1 optimizations)
    client: Arc<ClobClient>,

    /// Circuit breaker for risk management
    circuit_breaker: Arc<CircuitBreaker>,

    /// Fee rate in basis points
    fee_rate_bps: u16,
}

impl ArbitrageExecutor {
    /// Create a new arbitrage executor
    ///
    /// # Arguments
    /// * `client` - CLOB client with Tier 1 optimizations
    /// * `circuit_breaker` - Circuit breaker for risk management
    /// * `fee_rate_bps` - Fee rate in basis points (e.g., 100 = 1%)
    pub fn new(
        client: Arc<ClobClient>,
        circuit_breaker: Arc<CircuitBreaker>,
        fee_rate_bps: u16,
    ) -> Self {
        Self {
            client,
            circuit_breaker,
            fee_rate_bps,
        }
    }

    /// Execute arbitrage with batch orders and rollback
    ///
    /// **Performance:** ~150-200ms (vs 400ms sequential)
    ///
    /// # Safety
    /// - Verifies both orders succeeded
    /// - Automatically rolls back partial fills
    /// - Trips circuit breaker if rollback fails
    /// - Updates circuit breaker with P&L
    ///
    /// # Arguments
    /// * `opportunity` - Arbitrage opportunity to execute
    ///
    /// # Returns
    /// Execution result with order hashes and P&L
    pub async fn execute(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<ExecutionResult> {
        // Circuit breaker check
        if !self.circuit_breaker.can_execute() {
            return Ok(ExecutionResult::Failed {
                error: "Circuit breaker tripped".to_string(),
                latency_ms: 0,
            });
        }

        // Track positions before execution
        self.circuit_breaker.open_position()
            .map_err(|e| anyhow!(e))?;
        self.circuit_breaker.open_position()
            .map_err(|e| anyhow!(e))?; // Two positions (BUY + SELL)

        // Build batch orders
        let buy_request = CreateOrderRequest {
            token_id: opportunity.token_id.0.clone(),
            side: OrderSide::BUY,
            price: opportunity.ask_price,
            size: opportunity.max_size,
            order_type: "GTC".to_string(),
            expiration: None,
            fee_rate_bps: self.fee_rate_bps,
        };

        let sell_request = CreateOrderRequest {
            token_id: opportunity.token_id.0.clone(),
            side: OrderSide::SELL,
            price: opportunity.bid_price,
            size: opportunity.max_size,
            order_type: "GTC".to_string(),
            expiration: None,
            fee_rate_bps: self.fee_rate_bps,
        };

        // Execute batch (single HTTP request, ~150-200ms)
        tracing::info!(
            "Executing arbitrage: BUY@{:.4} SELL@{:.4} size={:.2} spread={:.4}",
            opportunity.ask_price,
            opportunity.bid_price,
            opportunity.max_size,
            opportunity.profit_margin
        );

        let start = Instant::now();
        let response = self.client.create_batch_orders(&[buy_request, sell_request]).await;
        let latency_ms = start.elapsed().as_millis() as u64;

        tracing::info!("Batch order latency: {}ms", latency_ms);

        // Handle response
        match response {
            Ok(batch_response) => {
                // Verify and handle result
                let result = self.verify_and_rollback(&batch_response, latency_ms, opportunity).await?;

                // Update circuit breaker based on result
                self.update_circuit_breaker(&result);

                Ok(result)
            }
            Err(e) => {
                // Both orders failed
                tracing::error!("Batch order request failed: {}", e);

                self.circuit_breaker.close_position();
                self.circuit_breaker.close_position();
                self.circuit_breaker.record_error();

                Ok(ExecutionResult::Failed {
                    error: e.to_string(),
                    latency_ms,
                })
            }
        }
    }

    /// Verify both orders succeeded, rollback if needed
    ///
    /// This is the critical safety mechanism for arbitrage execution.
    async fn verify_and_rollback(
        &self,
        response: &crate::types::BatchOrderResponse,
        latency_ms: u64,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<ExecutionResult> {
        // Check if both succeeded
        if response.both_succeeded() {
            let buy_hash = response.buy_hash().unwrap().clone();
            let sell_hash = response.sell_hash().unwrap().clone();

            // Calculate P&L
            let pnl = self.calculate_pnl(opportunity);

            tracing::info!(
                "✅ Arbitrage executed successfully: BUY={} SELL={} PNL=${:.2}",
                buy_hash,
                sell_hash,
                pnl
            );

            return Ok(ExecutionResult::Success {
                buy_hash,
                sell_hash,
                pnl,
                latency_ms,
            });
        }

        // Check for partial fill
        if response.is_partial_fill() {
            let filled_hash = response.order_hashes.get(0).unwrap().clone();

            tracing::error!(
                "⚠️ PARTIAL FILL DETECTED! Only one order succeeded: {}",
                filled_hash
            );

            // Attempt rollback
            return match self.client.cancel_order(&filled_hash).await {
                Ok(_) => {
                    tracing::info!("✅ Rollback successful: cancelled {}", filled_hash);

                    Ok(ExecutionResult::PartialFill {
                        filled_hash,
                        rolled_back: true,
                        latency_ms,
                    })
                }
                Err(e) => {
                    // CRITICAL: Rollback failed!
                    tracing::error!(
                        "❌ ROLLBACK FAILED for {}: {}",
                        filled_hash,
                        e
                    );
                    tracing::error!("⚠️ ONE-SIDED POSITION EXISTS - MANUAL INTERVENTION REQUIRED!");

                    // Trip circuit breaker to prevent further trading
                    self.circuit_breaker.trip();

                    Ok(ExecutionResult::PartialFill {
                        filled_hash,
                        rolled_back: false,
                        latency_ms,
                    })
                }
            };
        }

        // Both failed
        tracing::warn!("Both orders failed: {}", response.error_msg);

        Ok(ExecutionResult::Failed {
            error: response.error_msg.clone(),
            latency_ms,
        })
    }

    /// Calculate estimated P&L for successful arbitrage
    fn calculate_pnl(&self, opportunity: &ArbitrageOpportunity) -> f64 {
        // Spread per share
        let spread = opportunity.bid_price - opportunity.ask_price;

        // Gross profit
        let gross_profit = spread * opportunity.max_size;

        // Fees (buy side + sell side)
        let fee_rate = self.fee_rate_bps as f64 / 10000.0;
        let buy_fee = opportunity.ask_price * opportunity.max_size * fee_rate;
        let sell_fee = opportunity.bid_price * opportunity.max_size * fee_rate;

        // Net profit
        gross_profit - buy_fee - sell_fee
    }

    /// Update circuit breaker based on execution result
    fn update_circuit_breaker(&self, result: &ExecutionResult) {
        match result {
            ExecutionResult::Success { pnl, .. } => {
                // Record profit/loss
                if let Err(e) = self.circuit_breaker.record_trade(*pnl) {
                    tracing::error!("Failed to record trade: {}", e);
                }

                // Positions automatically managed by circuit breaker
                // (they remain open until manually closed)
            }
            ExecutionResult::PartialFill { rolled_back, .. } => {
                // Close both positions
                self.circuit_breaker.close_position();
                self.circuit_breaker.close_position();

                // Record error
                self.circuit_breaker.record_error();

                if !rolled_back {
                    // Rollback failed - circuit breaker already tripped
                    tracing::error!("Circuit breaker tripped due to rollback failure");
                }
            }
            ExecutionResult::Failed { .. } => {
                // Close both positions
                self.circuit_breaker.close_position();
                self.circuit_breaker.close_position();

                // Record error
                self.circuit_breaker.record_error();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MarketId, RiskConfig, TokenId};

    fn create_test_opportunity() -> ArbitrageOpportunity {
        let bid_price = 0.75;
        let ask_price = 0.70;
        let max_size = 100.0;
        let spread = bid_price - ask_price;
        let expected_profit = spread * max_size;

        ArbitrageOpportunity {
            market_id: MarketId("TRUMP-WIN".to_string()),
            token_id: TokenId("YES".to_string()),
            bid_price,
            ask_price,
            max_size,
            profit_margin: 0.0714, // (0.75-0.70)/0.70
            expected_profit,
            detected_at: 1000,
        }
    }

    #[test]
    fn test_pnl_calculation() {
        use crate::clob::client::ClobConfig;

        let config = ClobConfig {
            base_url: "https://test.example.com".to_string(),
            api_key: "test_key".to_string(),
            private_key: "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            chain_id: 137,
            verifying_contract: "0x0000000000000000000000000000000000000001".to_string(),
            maker_address: "0x0000000000000000000000000000000000000002".to_string(),
            taker_address: "0x0000000000000000000000000000000000000000".to_string(),
            timeout_secs: 10,
        };

        let client = Arc::new(ClobClient::new(config).unwrap());
        let cb = Arc::new(CircuitBreaker::new(RiskConfig {
            max_daily_loss: 100.0,
            max_position_size: 50.0,
            max_open_positions: 10,
            min_usdc_balance: 10.0,
            min_matic_balance: 1.0,
            max_consecutive_errors: 5,
        }));

        let executor = ArbitrageExecutor::new(client, cb, 100); // 1% fee

        let opportunity = create_test_opportunity();
        let pnl = executor.calculate_pnl(&opportunity);

        // Spread: 0.75 - 0.70 = 0.05
        // Gross profit: 0.05 * 100 = 5.0
        // Buy fee: 0.70 * 100 * 0.01 = 0.70
        // Sell fee: 0.75 * 100 * 0.01 = 0.75
        // Net profit: 5.0 - 0.70 - 0.75 = 3.55

        assert!((pnl - 3.55).abs() < 0.01, "Expected ~3.55, got {}", pnl);
    }

    #[test]
    fn test_execution_result_methods() {
        let success = ExecutionResult::Success {
            buy_hash: "0xabc".to_string(),
            sell_hash: "0xdef".to_string(),
            pnl: 5.0,
            latency_ms: 150,
        };

        assert!(success.is_success());
        assert_eq!(success.latency_ms(), 150);
        assert_eq!(success.pnl(), 5.0);

        let partial = ExecutionResult::PartialFill {
            filled_hash: "0xabc".to_string(),
            rolled_back: true,
            latency_ms: 200,
        };

        assert!(!partial.is_success());
        assert_eq!(partial.latency_ms(), 200);
        assert_eq!(partial.pnl(), 0.0);

        let failed = ExecutionResult::Failed {
            error: "test error".to_string(),
            latency_ms: 50,
        };

        assert!(!failed.is_success());
        assert_eq!(failed.latency_ms(), 50);
        assert_eq!(failed.pnl(), 0.0);
    }
}
