///! Integration tests for configuration loading
///!
///! Tests that configuration can be loaded from environment variables

use polymarket_hft_bot::types::config::BotConfig;

#[test]
fn test_config_has_defaults() {
    let config = BotConfig::default();

    // Verify default trading config
    assert_eq!(config.trading.default_amount, 10.0);
    assert_eq!(config.trading.price_threshold, 0.02);

    // Verify default risk config
    assert_eq!(config.risk.max_daily_loss, 100.0);
    assert_eq!(config.risk.max_open_positions, 5);

    // Verify default features
    assert!(config.features.dry_run, "Dry run should be enabled by default");
    assert!(config.features.arbitrage_enabled);
}

#[test]
fn test_config_validation() {
    let mut config = BotConfig::default();

    // Valid config should pass
    assert!(config.validate().is_ok());

    // Invalid trading config (take profit < stop loss)
    config.trading.take_profit_amount = 0.01;
    config.trading.stop_loss_amount = 0.05;
    assert!(config.validate().is_err());

    // Fix it
    config.trading.take_profit_amount = 0.05;
    config.trading.stop_loss_amount = 0.03;
    assert!(config.validate().is_ok());

    // Invalid risk config (0 positions)
    config.risk.max_open_positions = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_wallet_config_structure() {
    let config = BotConfig::default();

    // Wallet should have empty credentials by default
    assert_eq!(config.wallet.private_key, "");
    assert_eq!(config.wallet.address, "");
    assert_eq!(config.wallet.chain_id, 137); // Polygon mainnet
}

#[test]
fn test_polymarket_config() {
    let config = BotConfig::default();

    // Verify Polymarket endpoints
    assert_eq!(config.polymarket.clob_api_url, "https://clob.polymarket.com");
    assert_eq!(config.polymarket.gamma_api_url, "https://gamma-api.polymarket.com");
    assert_eq!(config.polymarket.chain_id, 137);
}

#[test]
fn test_feature_flags() {
    let config = BotConfig::default();

    // Dry run should be on by default (safety)
    assert!(config.features.dry_run);

    // Arbitrage should be enabled
    assert!(config.features.arbitrage_enabled);

    // Copy trading disabled by default
    assert!(!config.features.copy_trading_enabled);
}
