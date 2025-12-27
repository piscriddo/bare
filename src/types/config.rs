//! Configuration type definitions
//!
//! Defines all configuration structures for the bot.

use serde::{Deserialize, Serialize};

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Ethereum private key (0x-prefixed hex)
    pub private_key: String,

    /// Polygon wallet address (0x-prefixed hex)
    pub address: String,

    /// Chain ID (Polygon mainnet = 137)
    pub chain_id: u64,
}

/// Trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Default USDC amount per trade
    pub default_amount: f64,

    /// Minimum price difference to trigger trade (0.0-1.0)
    pub price_threshold: f64,

    /// Price increase for take profit order (0.0-1.0)
    pub take_profit_amount: f64,

    /// Price decrease for stop loss order (0.0-1.0)
    pub stop_loss_amount: f64,

    /// Milliseconds between trades
    pub cooldown_ms: u64,
}

impl TradingConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.default_amount <= 0.0 {
            return Err("default_amount must be positive".to_string());
        }
        if self.default_amount > 10000.0 {
            return Err("default_amount too large".to_string());
        }
        if self.price_threshold < 0.0 || self.price_threshold > 1.0 {
            return Err("price_threshold must be 0.0-1.0".to_string());
        }
        if self.take_profit_amount <= self.stop_loss_amount {
            return Err("take_profit must be greater than stop_loss".to_string());
        }
        Ok(())
    }
}

/// Risk management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum daily loss in USDC
    pub max_daily_loss: f64,

    /// Maximum size per position in USDC
    pub max_position_size: f64,

    /// Maximum concurrent open positions
    pub max_open_positions: usize,

    /// Minimum USDC balance required
    pub min_usdc_balance: f64,

    /// Minimum MATIC balance for gas
    pub min_matic_balance: f64,

    /// Maximum consecutive errors before circuit breaker trips
    pub max_consecutive_errors: usize,
}

impl RiskConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_daily_loss <= 0.0 {
            return Err("max_daily_loss must be positive".to_string());
        }
        if self.max_position_size <= 0.0 {
            return Err("max_position_size must be positive".to_string());
        }
        if self.max_open_positions == 0 {
            return Err("max_open_positions must be positive".to_string());
        }
        if self.max_open_positions > 100 {
            return Err("max_open_positions too large".to_string());
        }
        Ok(())
    }
}

/// Polymarket API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketConfig {
    /// CLOB API URL
    pub clob_api_url: String,

    /// Gamma API URL
    pub gamma_api_url: String,

    /// Chain ID
    pub chain_id: u64,

    /// RPC URL
    pub rpc_url: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (debug, info, warn, error)
    pub level: String,

    /// Whether to log to file
    pub to_file: bool,

    /// File path for logs (optional)
    pub file_path: Option<String>,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Enable arbitrage detection
    pub arbitrage_enabled: bool,

    /// Enable copy trading
    pub copy_trading_enabled: bool,

    /// Dry run mode (don't execute real trades)
    pub dry_run: bool,
}

/// Complete bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub wallet: WalletConfig,
    pub trading: TradingConfig,
    pub risk: RiskConfig,
    pub polymarket: PolymarketConfig,
    pub logging: LoggingConfig,
    pub features: FeatureConfig,
}

impl BotConfig {
    /// Validate entire configuration
    pub fn validate(&self) -> Result<(), String> {
        self.trading.validate()?;
        self.risk.validate()?;
        Ok(())
    }

    /// Load configuration from environment and file
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::Environment::with_prefix("BOT"))
            .build()?;

        settings.try_deserialize()
    }
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            wallet: WalletConfig {
                private_key: String::new(),
                address: String::new(),
                chain_id: 137, // Polygon mainnet
            },
            trading: TradingConfig {
                default_amount: 10.0,
                price_threshold: 0.02,
                take_profit_amount: 0.05,
                stop_loss_amount: 0.03,
                cooldown_ms: 1000,
            },
            risk: RiskConfig {
                max_daily_loss: 100.0,
                max_position_size: 50.0,
                max_open_positions: 5,
                min_usdc_balance: 10.0,
                min_matic_balance: 0.1,
                max_consecutive_errors: 3,
            },
            polymarket: PolymarketConfig {
                clob_api_url: "https://clob.polymarket.com".to_string(),
                gamma_api_url: "https://gamma-api.polymarket.com".to_string(),
                chain_id: 137,
                rpc_url: "https://polygon-rpc.com".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                to_file: true,
                file_path: Some("bot.log".to_string()),
            },
            features: FeatureConfig {
                arbitrage_enabled: true,
                copy_trading_enabled: false,
                dry_run: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_config_validation() {
        let mut config = TradingConfig {
            default_amount: 10.0,
            price_threshold: 0.02,
            take_profit_amount: 0.05,
            stop_loss_amount: 0.03,
            cooldown_ms: 1000,
        };

        assert!(config.validate().is_ok());

        // Test invalid amount
        config.default_amount = -1.0;
        assert!(config.validate().is_err());

        config.default_amount = 10.0;

        // Test invalid take profit
        config.take_profit_amount = 0.01;
        config.stop_loss_amount = 0.03;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_risk_config_validation() {
        let mut config = RiskConfig {
            max_daily_loss: 100.0,
            max_position_size: 50.0,
            max_open_positions: 5,
            min_usdc_balance: 10.0,
            min_matic_balance: 0.1,
            max_consecutive_errors: 3,
        };

        assert!(config.validate().is_ok());

        // Test invalid max positions
        config.max_open_positions = 0;
        assert!(config.validate().is_err());

        config.max_open_positions = 150;
        assert!(config.validate().is_err());
    }
}
