//! Position Redemption Module
//!
//! Handles tracking and redeeming binary arbitrage positions after market expiry.

use crate::types::{MarketId, TokenId};
use crate::clob::ClobClient;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, warn, error};

/// Position to be redeemed
#[derive(Debug, Clone)]
pub struct RedeemablePosition {
    /// Market ID
    pub market_id: MarketId,

    /// Market title
    pub title: String,

    /// YES token ID
    pub yes_token_id: TokenId,

    /// NO token ID
    pub no_token_id: TokenId,

    /// Position size
    pub size: f64,

    /// Total cost paid
    pub cost: f64,

    /// Expected profit
    pub expected_profit: f64,

    /// Market expiry time
    pub expiry: Option<DateTime<Utc>>,

    /// When position was opened
    pub opened_at: DateTime<Utc>,

    /// Whether position has been redeemed
    pub redeemed: bool,
}

impl RedeemablePosition {
    /// Check if position is ready to redeem
    pub fn is_ready_to_redeem(&self) -> bool {
        if self.redeemed {
            return false;
        }

        if let Some(expiry) = self.expiry {
            // Market has expired
            Utc::now() >= expiry
        } else {
            // No expiry time - check manually
            false
        }
    }

    /// Calculate how long until expiry
    pub fn time_until_expiry(&self) -> Option<chrono::Duration> {
        self.expiry.map(|expiry| expiry - Utc::now())
    }
}

/// Position redemption manager
pub struct RedemptionManager {
    positions: HashMap<MarketId, RedeemablePosition>,
}

impl RedemptionManager {
    /// Create new redemption manager
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }

    /// Add position to track
    pub fn add_position(&mut self, position: RedeemablePosition) {
        info!("📦 Tracking position for redemption: {}", position.title);
        if let Some(expiry) = position.expiry {
            info!("   Expires at: {}", expiry.format("%Y-%m-%d %H:%M:%S UTC"));
            if let Some(duration) = position.time_until_expiry() {
                let minutes = duration.num_minutes();
                info!("   Time until expiry: {} minutes", minutes);
            }
        }

        self.positions.insert(position.market_id.clone(), position);
    }

    /// Get all positions ready to redeem
    pub fn get_redeemable_positions(&self) -> Vec<&RedeemablePosition> {
        self.positions
            .values()
            .filter(|p| p.is_ready_to_redeem())
            .collect()
    }

    /// Mark position as redeemed
    pub fn mark_redeemed(&mut self, market_id: &MarketId) -> Result<()> {
        let position = self.positions
            .get_mut(market_id)
            .ok_or_else(|| anyhow!("Position not found: {}", market_id.0))?;

        position.redeemed = true;
        info!("✅ Position marked as redeemed: {}", position.title);

        Ok(())
    }

    /// Get position count
    pub fn position_count(&self) -> usize {
        self.positions.len()
    }

    /// Get unredeemed position count
    pub fn unredeemed_count(&self) -> usize {
        self.positions.values().filter(|p| !p.redeemed).count()
    }

    /// Redeem position via CLOB client
    pub async fn redeem_position(
        &mut self,
        market_id: &MarketId,
        clob_client: &ClobClient,
    ) -> Result<f64> {
        let position = self.positions
            .get(market_id)
            .ok_or_else(|| anyhow!("Position not found: {}", market_id.0))?;

        if position.redeemed {
            return Err(anyhow!("Position already redeemed"));
        }

        if !position.is_ready_to_redeem() {
            return Err(anyhow!("Position not ready to redeem yet"));
        }

        info!("💰 Redeeming position: {}", position.title);
        info!("   Market ID: {}", position.market_id.0);
        info!("   Size: {}", position.size);
        info!("   Cost: ${:.2}", position.cost);
        info!("   Expected profit: ${:.2}", position.expected_profit);

        // For binary arbitrage:
        // - We own both YES and NO tokens
        // - One pays $1.00, other pays $0.00
        // - We need to claim the winning token

        // TODO: Implement actual redemption via CLOB API
        // For now, simulate redemption
        let payout = position.size; // Each token pays $1.00 if it wins
        let profit = payout - position.cost;

        info!("✅ Redemption successful!");
        info!("   Payout: ${:.2}", payout);
        info!("   Profit: ${:.2}", profit);

        // Mark as redeemed
        self.mark_redeemed(market_id)?;

        Ok(profit)
    }

    /// Auto-redeem all ready positions
    pub async fn auto_redeem_all(
        &mut self,
        clob_client: &ClobClient,
    ) -> Result<f64> {
        let redeemable: Vec<MarketId> = self
            .get_redeemable_positions()
            .iter()
            .map(|p| p.market_id.clone())
            .collect();

        if redeemable.is_empty() {
            return Ok(0.0);
        }

        info!("🔄 Auto-redeeming {} positions...", redeemable.len());

        let mut total_profit = 0.0;

        for market_id in redeemable {
            match self.redeem_position(&market_id, clob_client).await {
                Ok(profit) => {
                    total_profit += profit;
                }
                Err(e) => {
                    error!("Failed to redeem {}: {}", market_id.0, e);
                }
            }
        }

        info!("✅ Auto-redemption complete: ${:.2} total profit", total_profit);

        Ok(total_profit)
    }

    /// Log status of all positions
    pub fn log_status(&self) {
        let total = self.position_count();
        let unredeemed = self.unredeemed_count();
        let redeemed = total - unredeemed;

        info!("📊 Position Status:");
        info!("   Total positions: {}", total);
        info!("   Unredeemed: {}", unredeemed);
        info!("   Redeemed: {}", redeemed);

        // Show positions expiring soon
        let ready = self.get_redeemable_positions();
        if !ready.is_empty() {
            info!("   Ready to redeem: {}", ready.len());
            for pos in ready {
                info!("     - {}: ${:.2} expected profit",
                    pos.title, pos.expected_profit);
            }
        }

        // Show upcoming expirations
        let upcoming: Vec<_> = self.positions
            .values()
            .filter(|p| !p.redeemed && p.expiry.is_some())
            .filter(|p| {
                if let Some(duration) = p.time_until_expiry() {
                    duration.num_minutes() < 60 // Expiring within 1 hour
                } else {
                    false
                }
            })
            .collect();

        if !upcoming.is_empty() {
            info!("   Expiring soon (< 1 hour): {}", upcoming.len());
            for pos in upcoming {
                if let Some(duration) = pos.time_until_expiry() {
                    info!("     - {} in {} min: ${:.2} profit",
                        pos.title,
                        duration.num_minutes(),
                        pos.expected_profit
                    );
                }
            }
        }
    }
}

impl Default for RedemptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_position(expired: bool) -> RedeemablePosition {
        let expiry = if expired {
            Some(Utc::now() - chrono::Duration::minutes(1)) // 1 minute ago
        } else {
            Some(Utc::now() + chrono::Duration::minutes(15)) // 15 minutes from now
        };

        RedeemablePosition {
            market_id: MarketId("test-market".to_string()),
            title: "Test Market".to_string(),
            yes_token_id: TokenId("yes-token".to_string()),
            no_token_id: TokenId("no-token".to_string()),
            size: 100.0,
            cost: 93.0,
            expected_profit: 7.0,
            expiry,
            opened_at: Utc::now() - chrono::Duration::minutes(10),
            redeemed: false,
        }
    }

    #[test]
    fn test_position_ready_to_redeem() {
        let expired_pos = create_test_position(true);
        assert!(expired_pos.is_ready_to_redeem());

        let active_pos = create_test_position(false);
        assert!(!active_pos.is_ready_to_redeem());
    }

    #[test]
    fn test_redemption_manager() {
        let mut manager = RedemptionManager::new();

        // Add expired position
        let pos1 = create_test_position(true);
        manager.add_position(pos1);

        // Add active position
        let pos2 = create_test_position(false);
        manager.add_position(pos2);

        assert_eq!(manager.position_count(), 2);
        assert_eq!(manager.unredeemed_count(), 2);

        let redeemable = manager.get_redeemable_positions();
        assert_eq!(redeemable.len(), 1);
    }

    #[test]
    fn test_mark_redeemed() {
        let mut manager = RedemptionManager::new();

        let pos = create_test_position(true);
        let market_id = pos.market_id.clone();
        manager.add_position(pos);

        assert_eq!(manager.unredeemed_count(), 1);

        manager.mark_redeemed(&market_id).unwrap();

        assert_eq!(manager.unredeemed_count(), 0);
    }
}
