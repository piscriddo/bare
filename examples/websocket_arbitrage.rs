//! Example: Real-time arbitrage detection with WebSocket orderbook streaming
//!
//! **Phase 5 Integration:** WebSocket + SIMD Detector
//!
//! Demonstrates:
//! - WebSocket orderbook streaming
//! - Real-time SIMD arbitrage detection
//! - Complete execution pipeline
//!
//! Run with: cargo run --example websocket_arbitrage

use polymarket_hft_bot::core::arbitrage::{ArbitrageConfig, ScalarArbitrageDetector};
use polymarket_hft_bot::services::websocket::{process_message, PolymarketWebSocket};
use polymarket_hft_bot::types::{MarketId, TokenId};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🌐 WebSocket Arbitrage Detection Demo\n");

    // Configure arbitrage detector
    let config = ArbitrageConfig {
        min_profit_margin: 0.02, // 2% minimum profit
        min_size: 10.0,          // $10 minimum size
        max_spread: 0.5,          // 50% max spread (sanity check)
    };

    let detector = ScalarArbitrageDetector::new(config);

    println!("⚙️  Configuration:");
    println!("   Min Profit Margin: 2%");
    println!("   Min Order Size: $10");
    println!("   Max Spread: 50%\n");

    // Markets to monitor
    let markets = vec![
        (MarketId("TRUMP-WIN".to_string()), TokenId("YES".to_string())),
        (MarketId("BIDEN-WIN".to_string()), TokenId("YES".to_string())),
        (MarketId("DeSANTIS-WIN".to_string()), TokenId("YES".to_string())),
    ];

    println!("📊 Monitoring markets:");
    for (market, token) in &markets {
        println!("   - {}/{}", market.0, token.0);
    }
    println!();

    // Create WebSocket client
    let (ws_client, mut rx) = PolymarketWebSocket::new(
        "wss://clob.polymarket.com/ws".to_string(),
        markets.clone(),
    );

    // Start WebSocket in background
    tokio::spawn(async move {
        if let Err(e) = ws_client.start().await {
            eprintln!("❌ WebSocket error: {}", e);
        }
    });

    println!("🔄 Listening for orderbook updates...\n");
    println!("═══════════════════════════════════════════════════════════");

    let mut update_count = 0;
    let mut arbitrage_count = 0;

    // Process orderbook updates
    while let Some(message) = rx.recv().await {
        // Extract orderbook update
        if let Some(update) = process_message(message) {
            update_count += 1;

            // Detect arbitrage opportunities
            if let Some(opportunity) = detector.detect(
                &update.market_id,
                &update.token_id,
                &update.order_book,
            ) {
                arbitrage_count += 1;

                println!("\n🎯 ARBITRAGE OPPORTUNITY #{}", arbitrage_count);
                println!("   Market: {}/{}", opportunity.market_id.0, opportunity.token_id.0);
                println!("   Buy at: ${:.4} (ask)", opportunity.ask_price);
                println!("   Sell at: ${:.4} (bid)", opportunity.bid_price);
                println!("   Spread: ${:.4}", opportunity.bid_price - opportunity.ask_price);
                println!("   Profit Margin: {:.2}%", opportunity.profit_margin * 100.0);
                println!("   Max Size: {:.2} shares", opportunity.max_size);
                println!("   Expected Profit: ${:.2}", opportunity.expected_profit);
                println!("═══════════════════════════════════════════════════════════");
            }

            // Print status every 100 updates
            if update_count % 100 == 0 {
                println!("\n📈 Status: {} updates processed, {} opportunities found",
                    update_count, arbitrage_count);
                println!("═══════════════════════════════════════════════════════════");
            }
        }
    }

    Ok(())
}
