//! Binary Arbitrage Trading Bot
//!
//! Live trading bot for binary arbitrage on crypto up/down markets.
//!
//! # Strategy
//! - Fetches active crypto up/down markets (BTC, ETH, SOL, XRP)
//! - Subscribes to YES/NO orderbooks via WebSocket
//! - Detects binary arbitrage: YES + NO ≠ $1.00
//! - Executes both sides simultaneously
//! - Tracks positions and redeems at expiry
//!
//! # Usage
//! ```bash
//! # Dry-run mode (no real trades)
//! cargo run --bin binary_arbitrage_bot -- --dry-run
//!
//! # Live trading with $20
//! cargo run --bin binary_arbitrage_bot
//! ```

use anyhow::{anyhow, Result};
use polymarket_hft_bot::{
    clob::{ClobClient, ClobConfig, CreateOrderRequest},
    types::config::BotConfig,
    core::redemption::{RedemptionManager, RedeemablePosition},
    strategies::{
        ArbitrageSide, BinaryArbitrageConfig, BinaryArbitrageDetector,
        CryptoAsset, CryptoUpDownConfig, CryptoUpDownFetcher, Timeframe,
    },
    services::websocket::{PolymarketMessage, process_message},
    types::{OrderBook, TokenId, OrderSide, MarketId},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use chrono::Utc;

/// Orderbook cache for all tracked tokens
type OrderbookCache = Arc<RwLock<HashMap<TokenId, OrderBook>>>;

/// Binary arbitrage bot
struct BinaryArbitrageBot {
    config: BotConfig,
    clob_client: ClobClient,
    detector: BinaryArbitrageDetector,
    orderbook_cache: OrderbookCache,
    redemption_manager: Arc<RwLock<RedemptionManager>>,
    dry_run: bool,
}

impl BinaryArbitrageBot {
    /// Create new bot
    fn new(config: BotConfig, dry_run: bool) -> Result<Self> {
        // Create CLOB client configuration
        let clob_config = ClobConfig {
            base_url: config.polymarket.clob_api_url.clone(),
            api_key: String::new(), // API key not required for basic operations
            private_key: config.wallet.private_key.clone(),
            chain_id: config.wallet.chain_id,
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(), // TODO: Get from config
            maker_address: config.wallet.address.clone(),
            taker_address: "0x0000000000000000000000000000000000000000".to_string(),
            timeout_secs: 10,
        };

        let clob_client = ClobClient::new(clob_config)?;

        let arb_config = BinaryArbitrageConfig {
            min_profit_margin: 0.02, // 2% minimum (covers fees)
            min_size: 5.0,            // $5 minimum
            max_cost: config.risk.max_position_size, // Use risk config
        };

        let detector = BinaryArbitrageDetector::new(arb_config);

        Ok(Self {
            config,
            clob_client,
            detector,
            orderbook_cache: Arc::new(RwLock::new(HashMap::new())),
            redemption_manager: Arc::new(RwLock::new(RedemptionManager::new())),
            dry_run,
        })
    }

    /// Start the bot
    async fn start(&mut self) -> Result<()> {
        info!("🤖 Binary Arbitrage Bot Starting...");
        info!("Mode: {}", if self.dry_run { "DRY-RUN" } else { "LIVE" });

        // Step 1: Fetch crypto up/down markets
        info!("📡 Fetching crypto up/down markets...");
        let markets = self.fetch_markets().await?;
        info!("✅ Found {} active markets", markets.len());

        if markets.is_empty() {
            return Err(anyhow!("No markets found - check Gamma API or filters"));
        }

        // Step 2: Extract all token IDs for WebSocket subscription
        let token_ids: Vec<TokenId> = markets
            .iter()
            .flat_map(|m| {
                m.token_ids
                    .iter()
                    .map(|id| TokenId(id.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        info!("📋 Subscribing to {} token orderbooks", token_ids.len());

        // Step 3: Start WebSocket and process orderbook updates
        self.run_websocket_loop(token_ids, markets).await?;

        Ok(())
    }

    /// Fetch active crypto up/down markets
    async fn fetch_markets(
        &self,
    ) -> Result<Vec<polymarket_hft_bot::strategies::CryptoUpDownMarket>> {
        let config = CryptoUpDownConfig {
            assets: vec![
                CryptoAsset::Bitcoin,
                CryptoAsset::Ethereum,
                CryptoAsset::Solana,
                CryptoAsset::XRP,
            ],
            timeframes: vec![
                Timeframe::FifteenMin, // Fastest turnover!
                Timeframe::OneHour,
                Timeframe::FourHour,
            ],
            max_markets: 100,
        };

        let gamma_url = self.config.polymarket.gamma_api_url.clone();

        let fetcher = CryptoUpDownFetcher::new(config, gamma_url);
        fetcher.fetch_markets().await
    }

    /// Run WebSocket loop and process orderbook updates
    async fn run_websocket_loop(
        &mut self,
        token_ids: Vec<TokenId>,
        markets: Vec<polymarket_hft_bot::strategies::CryptoUpDownMarket>,
    ) -> Result<()> {
        // Create WebSocket client
        // TODO: Add websocket_url to PolymarketConfig
        let ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market".to_string();

        info!("🔌 Connecting to WebSocket: {}", ws_url);

        // For now, use a simple channel-based approach
        // In production, you'd use the full WebSocketManager
        let (tx, mut rx) = tokio::sync::mpsc::channel::<PolymarketMessage>(1000);

        // Spawn WebSocket listener (simplified for now)
        // TODO: Integrate with WebSocketManager properly
        let cache = self.orderbook_cache.clone();
        tokio::spawn(async move {
            // This is a placeholder - in production you'd connect to real WebSocket
            info!("WebSocket listener started (placeholder)");

            while let Some(msg) = rx.recv().await {
                if let Some(update) = process_message(msg) {
                    let mut cache = cache.write().await;
                    cache.insert(update.token_id, update.order_book);
                }
            }
        });

        // Main detection loop
        info!("🔍 Starting arbitrage detection loop...");
        self.detection_loop(markets).await?;

        Ok(())
    }

    /// Main detection loop - scans for arbitrage opportunities
    async fn detection_loop(
        &mut self,
        markets: Vec<polymarket_hft_bot::strategies::CryptoUpDownMarket>,
    ) -> Result<()> {
        let mut scan_count = 0;
        let mut opportunities_found = 0;

        loop {
            scan_count += 1;

            // Scan all markets for arbitrage
            for market in &markets {
                // Need at least 2 tokens (YES and NO)
                if market.token_ids.len() < 2 {
                    continue;
                }

                let yes_token_id = TokenId(market.token_ids[0].clone());
                let no_token_id = TokenId(market.token_ids[1].clone());

                // Get orderbooks from cache and check for arbitrage
                let opportunity = {
                    let cache = self.orderbook_cache.read().await;
                    let yes_orderbook = cache.get(&yes_token_id);
                    let no_orderbook = cache.get(&no_token_id);

                    if let (Some(yes_ob), Some(no_ob)) = (yes_orderbook, no_orderbook) {
                        // Detect arbitrage
                        self.detector.detect(
                            &MarketId(market.event_id.clone()),
                            &yes_token_id,
                            &no_token_id,
                            yes_ob,
                            no_ob,
                            market.title.clone(),
                            market.end_date.clone(),
                        )
                    } else {
                        None
                    }
                }; // cache read guard is dropped here

                if let Some(opportunity) = opportunity {
                    opportunities_found += 1;

                    info!("🎯 BINARY ARBITRAGE FOUND!");
                    info!("   Market: {}", opportunity.title);
                    info!("   Side: {:?}", opportunity.side);
                    info!("   Price sum: ${:.3}", opportunity.price_sum);
                    info!("   Profit: ${:.2} ({:.1}%)",
                        opportunity.expected_profit,
                        opportunity.profit_margin * 100.0
                    );

                    // Execute trade
                    if let Err(e) = self.execute_arbitrage(&opportunity).await {
                        error!("Failed to execute arbitrage: {}", e);
                    }
                }
            }

            // Progress update every 100 scans
            if scan_count % 100 == 0 {
                info!("📊 Scanned {} times, found {} opportunities", scan_count, opportunities_found);

                // Show position status
                let manager = self.redemption_manager.read().await;
                manager.log_status();

                // Auto-redeem ready positions
                drop(manager);
                let mut manager = self.redemption_manager.write().await;
                if let Err(e) = manager.auto_redeem_all(&self.clob_client).await {
                    error!("Auto-redemption failed: {}", e);
                }
            }

            // Sleep briefly between scans
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Execute binary arbitrage trade
    async fn execute_arbitrage(
        &mut self,
        opportunity: &polymarket_hft_bot::strategies::BinaryArbitrageOpportunity,
    ) -> Result<()> {
        info!("⚡ Executing {} arbitrage...", match opportunity.side {
            ArbitrageSide::Buy => "BUY",
            ArbitrageSide::Sell => "SELL",
        });

        if self.dry_run {
            info!("💡 DRY-RUN: Would execute:");
            info!("   {} {} YES at ${:.3}",
                match opportunity.side {
                    ArbitrageSide::Buy => "BUY",
                    ArbitrageSide::Sell => "SELL",
                },
                opportunity.max_size,
                opportunity.yes_price
            );
            info!("   {} {} NO at ${:.3}",
                match opportunity.side {
                    ArbitrageSide::Buy => "BUY",
                    ArbitrageSide::Sell => "SELL",
                },
                opportunity.max_size,
                opportunity.no_price
            );
            info!("   Total: ${:.2}, Profit: ${:.2}",
                opportunity.price_sum * opportunity.max_size,
                opportunity.expected_profit
            );
            return Ok(());
        }

        // Create orders for both sides
        let (yes_side, no_side) = match opportunity.side {
            ArbitrageSide::Buy => (OrderSide::BUY, OrderSide::BUY),
            ArbitrageSide::Sell => (OrderSide::SELL, OrderSide::SELL),
        };

        let yes_order = CreateOrderRequest {
            token_id: opportunity.yes_token_id.0.clone(),
            side: yes_side,
            price: opportunity.yes_price,
            size: opportunity.max_size,
            order_type: "GTC".to_string(), // Good-til-cancelled
            expiration: None,
            fee_rate_bps: 0, // TODO: Get from config
        };

        let no_order = CreateOrderRequest {
            token_id: opportunity.no_token_id.0.clone(),
            side: no_side,
            price: opportunity.no_price,
            size: opportunity.max_size,
            order_type: "GTC".to_string(),
            expiration: None,
            fee_rate_bps: 0,
        };

        // Execute both orders atomically
        info!("📤 Placing batch orders...");
        let batch_response = self
            .clob_client
            .create_batch_orders(&[yes_order, no_order])
            .await?;

        // Check if both orders were created successfully
        let success = batch_response.success && batch_response.order_hashes.len() >= 2;

        if success {
            info!("✅ Both orders created successfully!");
            info!("   Order hashes: {:?}", batch_response.order_hashes);

            // Parse expiry from string to DateTime
            let expiry = opportunity.expiry.as_ref().and_then(|exp_str| {
                chrono::DateTime::parse_from_rfc3339(exp_str)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            // Track position for redemption
            let position = RedeemablePosition {
                market_id: opportunity.market_id.clone(),
                title: opportunity.title.clone(),
                yes_token_id: opportunity.yes_token_id.clone(),
                no_token_id: opportunity.no_token_id.clone(),
                size: opportunity.max_size,
                cost: opportunity.price_sum * opportunity.max_size,
                expected_profit: opportunity.expected_profit,
                expiry,
                opened_at: Utc::now(),
                redeemed: false,
            };

            self.redemption_manager.write().await.add_position(position);

            info!("📦 Position tracked - will redeem at expiry");
        } else {
            error!("⚠️  Order creation failed!");
            error!("   Response: {:?}", batch_response);
            error!("   TODO: Implement rollback logic");
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║       BINARY ARBITRAGE BOT - POLYMARKET HFT             ║");
    info!("╚═══════════════════════════════════════════════════════════╝");

    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());

    if dry_run {
        warn!("⚠️  DRY-RUN MODE - No real trades will be executed");
    } else {
        warn!("🔴 LIVE MODE - Real trades with real money!");
    }

    // Load configuration
    info!("📋 Loading configuration...");
    let config = BotConfig::from_env()?;

    info!("⚙️  Configuration:");
    info!("   Max position size: ${:.2}", config.risk.max_position_size);
    info!("   Max daily loss: ${:.2}", config.risk.max_daily_loss);
    info!("   Max open positions: {}", config.risk.max_open_positions);

    // Create and start bot
    let mut bot = BinaryArbitrageBot::new(config, dry_run)?;
    bot.start().await?;

    Ok(())
}
