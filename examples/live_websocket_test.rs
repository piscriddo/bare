//! Live WebSocket Test - Connect to Real Polymarket Data
//!
//! This connects to the actual Polymarket WebSocket and shows live orderbook updates.
//!
//! Run with: cargo run --example live_websocket_test

use polymarket_hft_bot::core::arbitrage::{ArbitrageConfig, ScalarArbitrageDetector};
use serde_json::json;
use tokio;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 LIVE Polymarket WebSocket Test\n");
    println!("═══════════════════════════════════════════════════════════");
    println!("           CONNECTING TO REAL POLYMARKET DATA");
    println!("═══════════════════════════════════════════════════════════\n");

    // Real Polymarket WebSocket URL (market channel for public orderbook data)
    let ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

    println!("📡 Connecting to: {}", ws_url);

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(ws_url).await?;
    println!("✅ Connected successfully!\n");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to market channel (no auth needed for public market data)
    // Using ACTIVE markets from Gamma API (Dec 2025)
    let token_ids = vec![
        "52037280206803127284847057476966472509485061756266626447432615405338644734115", // Tim Cook - Apple CEO (Yes)
        "22907777524422234275710483491392890278714003990753035158958272245651067670038", // Tim Cook - Apple CEO (No)
    ];

    let subscribe_msg = json!({
        "type": "market",
        "assets_ids": token_ids,
    });

    println!("📤 Sending subscription request...");
    println!("   Subscribing to {} token IDs", token_ids.len());

    write.send(Message::Text(subscribe_msg.to_string())).await?;
    println!("✅ Subscription sent!\n");

    println!("═══════════════════════════════════════════════════════════");
    println!("              LISTENING FOR LIVE UPDATES");
    println!("═══════════════════════════════════════════════════════════\n");

    // Configure arbitrage detector
    let config = ArbitrageConfig {
        min_profit_margin: 0.01, // 1% minimum profit (lower for live data)
        min_size: 5.0,            // $5 minimum size
        max_spread: 0.5,          // 50% max spread
    };
    let detector = ScalarArbitrageDetector::new(config);

    let mut message_count = 0;
    let mut ping_count = 0;

    // Spawn ping task (required every 10 seconds)
    let write = std::sync::Arc::new(tokio::sync::Mutex::new(write));
    let write_clone = write.clone();

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            let mut w = write_clone.lock().await;
            if let Err(e) = w.send(Message::Text("PING".to_string())).await {
                eprintln!("Failed to send ping: {}", e);
                break;
            }
        }
    });

    // Listen for messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                message_count += 1;

                println!("\n📨 Message #{}: ", message_count);

                // Try to parse as JSON
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    println!("{}", serde_json::to_string_pretty(&parsed)?);

                    // Check if it's an orderbook update
                    if let Some(msg_type) = parsed.get("type") {
                        if msg_type == "book" {
                            println!("\n🎯 ORDERBOOK UPDATE DETECTED!");
                            // TODO: Parse and detect arbitrage
                        }
                    }
                } else {
                    println!("{}", text);
                }

                println!("─────────────────────────────────────────────────────────");

                // Wait for some real data before stopping
                if message_count >= 30 {
                    println!("\n✅ Received {} messages successfully!", message_count);
                    println!("💡 WebSocket connection working!");
                    break;
                }
            }
            Ok(Message::Ping(_)) => {
                println!("📍 Received PING");
            }
            Ok(Message::Pong(_)) => {
                ping_count += 1;
                println!("✅ PONG #{} received", ping_count);
            }
            Ok(Message::Close(frame)) => {
                println!("🔌 Connection closed: {:?}", frame);
                break;
            }
            Ok(_) => {
                println!("❓ Received other message type");
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
                break;
            }
        }
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("                 CONNECTION SUMMARY");
    println!("═══════════════════════════════════════════════════════════");
    println!("Messages received: {}", message_count);
    println!("Pings/Pongs: {}", ping_count);
    println!("Status: Connection successful! ✅");
    println!("═══════════════════════════════════════════════════════════\n");

    Ok(())
}
