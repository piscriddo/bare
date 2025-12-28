//! Binary Arbitrage Scanner Example
//!
//! Scans crypto up/down markets for binary arbitrage opportunities.
//!
//! Binary arbitrage occurs when YES + NO ≠ $1.00:
//! - When YES + NO < $1.00: BUY both for guaranteed profit
//! - When YES + NO > $1.00: SELL both for guaranteed profit
//!
//! # Strategy
//! Buy BOTH outcomes when sum < $1.00, guaranteed $1.00 payout at expiry.
//! Zero market risk - you own both outcomes!
//!
//! Run with: cargo run --example binary_arbitrage_scanner

use polymarket_hft_bot::{
    strategies::{
        BinaryArbitrageConfig, BinaryArbitrageDetector,
        CryptoAsset, CryptoUpDownConfig, CryptoUpDownFetcher, Timeframe,
    },
    types::{OrderBook, OrderBookEntry, TokenId},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    tracing_subscriber::fmt::init();

    println!("🎯 Binary Arbitrage Scanner\n");
    println!("═══════════════════════════════════════════════════════════");
    println!("     SCANNING FOR BINARY ARBITRAGE OPPORTUNITIES");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("Strategy:");
    println!("  • When YES + NO < $1.00: BUY both (profit at expiry)");
    println!("  • When YES + NO > $1.00: SELL both (profit at expiry)");
    println!("  • Zero market risk - you own both outcomes!\n");

    // Configure market fetcher
    let market_config = CryptoUpDownConfig {
        assets: vec![
            CryptoAsset::Bitcoin,
            CryptoAsset::Ethereum,
            CryptoAsset::Solana,
            CryptoAsset::XRP,
        ],
        timeframes: vec![
            Timeframe::FifteenMin,  // 15 minute markets (fastest turnover)
            Timeframe::OneHour,     // 1 hour markets
            Timeframe::FourHour,    // 4 hour markets
        ],
        max_markets: 100,
    };

    println!("⚙️  Market Configuration:");
    println!("   Assets: BTC, ETH, SOL, XRP");
    println!("   Timeframes: 15min, 1hour, 4hour");
    println!("   Max markets: {}\n", market_config.max_markets);

    // Configure arbitrage detector
    let arb_config = BinaryArbitrageConfig {
        min_profit_margin: 0.02, // 2% minimum (covers fees + buffer)
        min_size: 5.0,           // $5 minimum size
        max_cost: 100.0,         // Max $100 per trade
    };

    println!("⚙️  Arbitrage Configuration:");
    println!("   Min profit margin: {:.1}%", arb_config.min_profit_margin * 100.0);
    println!("   Min size: ${:.2}", arb_config.min_size);
    println!("   Max cost: ${:.2}\n", arb_config.max_cost);

    // Create fetcher and detector
    let gamma_api_url = "https://gamma-api.polymarket.com".to_string();
    let fetcher = CryptoUpDownFetcher::new(market_config, gamma_api_url);
    let detector = BinaryArbitrageDetector::new(arb_config);

    // Step 1: Fetch crypto up/down markets
    println!("═══════════════════════════════════════════════════════════");
    println!("              STEP 1: FETCH MARKETS");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("📡 Fetching markets from Gamma API...\n");
    let markets = fetcher.fetch_markets().await?;

    println!("✅ Found {} active crypto up/down markets!\n", markets.len());

    if markets.is_empty() {
        println!("⚠️  No markets found matching criteria.");
        println!("   This might mean:");
        println!("   - No active up/down markets right now");
        println!("   - Markets have different slug patterns");
        println!("   - API is down or changed\n");
        return Ok(());
    }

    // Step 2: Scan for arbitrage opportunities
    println!("═══════════════════════════════════════════════════════════");
    println!("       STEP 2: SCAN FOR ARBITRAGE OPPORTUNITIES");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("📊 Scanning {} markets for binary arbitrage...\n", markets.len());

    // NOTE: In a real implementation, you would:
    // 1. Subscribe to WebSocket for real-time orderbook updates
    // 2. Fetch orderbooks for each market's YES and NO tokens
    // 3. Detect arbitrage opportunities in real-time
    //
    // For this example, we'll demonstrate with mock data

    let mut opportunities_found = 0;

    println!("🔍 Demonstrating arbitrage detection with example data:\n");

    // Example 1: BUY arbitrage (YES ask + NO ask < $1.00)
    let yes_orderbook_1 = create_mock_orderbook_full(0.45, 0.43, 50.0);
    let no_orderbook_1 = create_mock_orderbook_full(0.48, 0.46, 50.0);

    println!("Example 1: BTC Up/Down 15min");
    println!("  YES ask: $0.45, bid: $0.43");
    println!("  NO ask:  $0.48, bid: $0.46");
    println!("  Ask sum: $0.93 ← BUY ARBITRAGE!");

    if let Some(market) = markets.first() {
        if let Some(opp) = detector.detect(
            &market.event_id.clone().into(),
            &market.token_ids[0].clone().into(),
            &market.token_ids.get(1).unwrap_or(&market.token_ids[0]).clone().into(),
            &yes_orderbook_1,
            &no_orderbook_1,
            market.title.clone(),
            market.end_date.clone(),
        ) {
            opportunities_found += 1;
            println!("\n🎯 BUY ARBITRAGE FOUND!\n");
            print_opportunity(&opp);
        }
    }

    // Example 2: SELL arbitrage (YES bid + NO bid > $1.00)
    let yes_orderbook_2 = create_mock_orderbook_full(0.57, 0.55, 50.0);
    let no_orderbook_2 = create_mock_orderbook_full(0.54, 0.52, 50.0);

    println!("\n\nExample 2: ETH Up/Down 1hour");
    println!("  YES ask: $0.57, bid: $0.55");
    println!("  NO ask:  $0.54, bid: $0.52");
    println!("  Bid sum: $1.07 ← SELL ARBITRAGE!");

    if let Some(market) = markets.get(1).or(markets.first()) {
        if let Some(opp) = detector.detect(
            &market.event_id.clone().into(),
            &market.token_ids[0].clone().into(),
            &market.token_ids.get(1).unwrap_or(&market.token_ids[0]).clone().into(),
            &yes_orderbook_2,
            &no_orderbook_2,
            market.title.clone(),
            market.end_date.clone(),
        ) {
            opportunities_found += 1;
            println!("\n🎯 SELL ARBITRAGE FOUND!\n");
            print_opportunity(&opp);
        }
    }

    // Example 3: No arbitrage (YES + NO = $1.00)
    let yes_orderbook_3 = create_mock_orderbook_full(0.51, 0.50, 50.0);
    let no_orderbook_3 = create_mock_orderbook_full(0.49, 0.50, 50.0);

    println!("\n\nExample 3: SOL Up/Down 4hour");
    println!("  YES ask: $0.51, bid: $0.50");
    println!("  NO ask:  $0.49, bid: $0.50");
    println!("  Ask sum: $1.00, Bid sum: $1.00");
    println!("  ❌ No arbitrage (prices are efficient)\n");

    // Example 4: Small profit (filtered out)
    let yes_orderbook_4 = create_mock_orderbook_full(0.49, 0.47, 50.0);
    let no_orderbook_4 = create_mock_orderbook_full(0.50, 0.48, 50.0);

    println!("\nExample 4: XRP Up/Down 15min");
    println!("  YES ask: $0.49, bid: $0.47");
    println!("  NO ask:  $0.50, bid: $0.48");
    println!("  Ask sum: $0.99 (1% profit)");
    println!("  ❌ Filtered out (below 2% minimum threshold)\n");

    // Summary
    println!("\n═══════════════════════════════════════════════════════════");
    println!("                    SCAN SUMMARY");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("📊 Results:");
    println!("   Markets scanned: {}", markets.len());
    println!("   Opportunities found: {}", opportunities_found);
    println!("   Success rate: {:.1}%\n",
        (opportunities_found as f64 / markets.len() as f64) * 100.0);

    // Next steps
    println!("═══════════════════════════════════════════════════════════");
    println!("                  NEXT STEPS");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("To implement live trading:");
    println!("  1. Subscribe to WebSocket for token_ids");
    println!("  2. Receive real-time orderbook updates");
    println!("  3. Run binary arbitrage detector on each update");
    println!("  4. When opportunity found:");
    println!("     a. BUY YES token at ask price");
    println!("     b. BUY NO token at ask price");
    println!("     c. Wait for market expiry ({}min - {}min)",
        Timeframe::FifteenMin.duration_minutes(),
        Timeframe::FourHour.duration_minutes()
    );
    println!("     d. Redeem winning position for $1.00");
    println!("     e. Profit = $1.00 - (YES + NO)\n");

    println!("Risk management:");
    println!("  ✅ Zero market risk (own both outcomes)");
    println!("  ⚠️  Execution risk:");
    println!("     - Partial fills (may not get both sides)");
    println!("     - Failed rollback (one side fills, other fails)");
    println!("     - Fees eating into profit");
    println!("     - Price moves between detection and execution\n");

    println!("Capital efficiency:");
    println!("  • 15min markets: ~96 cycles/day (fast capital turnover!)");
    println!("  • 1hour markets: ~24 cycles/day");
    println!("  • 4hour markets: ~6 cycles/day");
    println!("  • With $20 capital + 2% profit + 15min markets:");
    println!("    → Theoretical: $0.40/cycle × 96 = $38.40/day");
    println!("    → Realistic: ~10-20 opportunities/day = $4-8/day\n");

    println!("═══════════════════════════════════════════════════════════\n");

    Ok(())
}

/// Create mock orderbook for demonstration (with both bids and asks)
fn create_mock_orderbook_full(ask_price: f64, bid_price: f64, size: f64) -> OrderBook {
    OrderBook {
        token_id: TokenId("mock-token".to_string()),
        bids: vec![OrderBookEntry {
            price: bid_price,
            size,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
        }],
        asks: vec![OrderBookEntry {
            price: ask_price,
            size,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
        }],
        timestamp: chrono::Utc::now().timestamp() as u64,
    }
}

/// Print opportunity details
fn print_opportunity(opp: &polymarket_hft_bot::strategies::BinaryArbitrageOpportunity) {
    use polymarket_hft_bot::strategies::ArbitrageSide;

    println!("  Market: {}", opp.title);
    println!("  Side:   {:?}", opp.side);
    println!("  YES Price: ${:.3}", opp.yes_price);
    println!("  NO Price:  ${:.3}", opp.no_price);
    println!("  Sum:       ${:.3}", opp.price_sum);
    println!("  Profit:    ${:.3} ({:.1}%)",
        opp.profit_margin, opp.profit_margin * 100.0);
    println!("  Max Size:  ${:.2}", opp.max_size);
    println!("  Expected:  ${:.2} profit", opp.expected_profit);
    if let Some(expiry) = &opp.expiry {
        println!("  Expires:   {}", expiry);
    }

    println!("\n  Trade Plan:");
    match opp.side {
        ArbitrageSide::Buy => {
            println!("    1. BUY {:.2} YES tokens at ${:.3}", opp.max_size, opp.yes_price);
            println!("    2. BUY {:.2} NO tokens at ${:.3}", opp.max_size, opp.no_price);
            println!("    3. Total cost: ${:.2}", opp.price_sum * opp.max_size);
            println!("    4. Wait for expiry...");
            println!("    5. Redeem winning side for ${:.2}", opp.max_size);
            println!("    6. Profit: ${:.2} ({:.1}% return)",
                opp.expected_profit,
                (opp.profit_margin / opp.price_sum) * 100.0);
        }
        ArbitrageSide::Sell => {
            println!("    1. SELL {:.2} YES tokens at ${:.3}", opp.max_size, opp.yes_price);
            println!("    2. SELL {:.2} NO tokens at ${:.3}", opp.max_size, opp.no_price);
            println!("    3. Total revenue: ${:.2}", opp.price_sum * opp.max_size);
            println!("    4. Wait for expiry...");
            println!("    5. Pay ${:.2} for losing side", opp.max_size);
            println!("    6. Profit: ${:.2} ({:.1}% return)",
                opp.expected_profit,
                (opp.profit_margin / opp.price_sum) * 100.0);
        }
    }
}
