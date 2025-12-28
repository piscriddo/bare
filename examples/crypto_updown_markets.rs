//! Crypto Up/Down Market Fetcher Example
//!
//! Demonstrates how to fetch and monitor short-timeframe crypto markets.
//! This is DIRECTIONAL TRADING (not arbitrage) - has full market risk!
//!
//! Run with: cargo run --example crypto_updown_markets

use polymarket_hft_bot::strategies::{
    CryptoAsset, CryptoUpDownConfig, CryptoUpDownFetcher, Timeframe,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    tracing_subscriber::fmt::init();

    println!("🎲 Crypto Up/Down Market Fetcher\n");
    println!("═══════════════════════════════════════════════════════════");
    println!("     FETCHING SHORT-TIMEFRAME CRYPTO MARKETS");
    println!("═══════════════════════════════════════════════════════════\n");

    // Configure which markets to fetch
    let config = CryptoUpDownConfig {
        assets: vec![
            CryptoAsset::Bitcoin,
            CryptoAsset::Ethereum,
            CryptoAsset::Solana,
            CryptoAsset::XRP,
        ],
        timeframes: vec![
            Timeframe::FifteenMin,  // 15 minute markets
            Timeframe::OneHour,     // 1 hour markets
            Timeframe::FourHour,    // 4 hour markets
        ],
        max_markets: 100, // Fetch up to 100 events
    };

    println!("⚙️  Configuration:");
    println!("   Assets: BTC, ETH, SOL, XRP");
    println!("   Timeframes: 15min, 1hour, 4hour");
    println!("   Max markets: {}\n", config.max_markets);

    // Create fetcher
    let gamma_api_url = "https://gamma-api.polymarket.com".to_string();
    let fetcher = CryptoUpDownFetcher::new(config, gamma_api_url);

    // Fetch markets
    println!("📡 Fetching markets from Gamma API...\n");
    let markets = fetcher.fetch_markets().await?;

    println!("\n✅ Found {} active crypto up/down markets!\n", markets.len());

    if markets.is_empty() {
        println!("⚠️  No markets found matching criteria.");
        println!("   This might mean:");
        println!("   - No active up/down markets right now");
        println!("   - Markets have different slug patterns");
        println!("   - API is down or changed\n");
        return Ok(());
    }

    // Group markets by asset and timeframe
    let grouped = CryptoUpDownFetcher::group_markets(&markets);

    println!("═══════════════════════════════════════════════════════════");
    println!("                    MARKET BREAKDOWN");
    println!("═══════════════════════════════════════════════════════════\n");

    for ((asset, timeframe), markets) in grouped.iter() {
        println!("📊 {} - {} ({} markets):",
            asset.name(),
            timeframe.display(),
            markets.len()
        );

        for market in markets {
            println!("   • {}", market.title);
            println!("     Slug: {}", market.slug);
            println!("     Tokens: {} (YES/NO)", market.token_ids.len());
            if let Some(end_date) = &market.end_date {
                println!("     Expires: {}", end_date);
            }
            println!();
        }
    }

    // Get all token IDs for WebSocket subscription
    let token_ids = CryptoUpDownFetcher::get_token_ids(&markets);

    println!("═══════════════════════════════════════════════════════════");
    println!("              WEBSOCKET SUBSCRIPTION INFO");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("📋 Total Token IDs to subscribe: {}", token_ids.len());
    println!("\nSample Token IDs:");
    for (i, token_id) in token_ids.iter().take(5).enumerate() {
        println!("   {}. {}", i + 1, token_id);
    }
    if token_ids.len() > 5 {
        println!("   ... and {} more\n", token_ids.len() - 5);
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("                 NEXT STEPS FOR TRADING");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("⚠️  IMPORTANT: This is DIRECTIONAL trading, NOT arbitrage!");
    println!();
    println!("Differences from arbitrage:");
    println!("  ❌ Full market risk (can lose 100%)");
    println!("  ❌ Need to predict direction (up or down)");
    println!("  ❌ Stop loss is CRITICAL");
    println!("  ❌ Must wait for market expiry to redeem");
    println!("  ✅ Can redeem and reinvest within hours\n");

    println!("To trade these markets:");
    println!("  1. Subscribe to WebSocket with token_ids");
    println!("  2. Monitor orderbook for good entry prices");
    println!("  3. Place BUY order on direction (YES/NO)");
    println!("  4. Wait for expiry ({}min - {}min)",
        Timeframe::FifteenMin.duration_minutes(),
        Timeframe::FourHour.duration_minutes()
    );
    println!("  5. Redeem winning positions");
    println!("  6. Reinvest proceeds\n");

    println!("Risk management recommendations:");
    println!("  • Start with 15min markets (fastest turnover)");
    println!("  • Never bet more than 5% of capital per market");
    println!("  • Set stop loss at entry price (cancel if moves against you)");
    println!("  • Track win rate (need >55% to be profitable after fees)");
    println!("  • Consider bot signals (RSI, momentum) for direction\n");

    println!("═══════════════════════════════════════════════════════════\n");

    Ok(())
}
