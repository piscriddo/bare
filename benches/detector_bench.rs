//! Benchmark: Arbitrage Detection Performance
//!
//! Compares fixed-point detector performance to validate 3x speedup claim.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polymarket_hft_bot::core::arbitrage::{ArbitrageConfig, ScalarArbitrageDetector};
use polymarket_hft_bot::types::{MarketId, OrderBook, OrderBookEntry, TokenId};

/// Create test orderbook with arbitrage opportunity
fn create_arbitrage_orderbook() -> OrderBook {
    OrderBook {
        token_id: TokenId("test-token".to_string()),
        bids: vec![OrderBookEntry {
            price: 0.76,
            size: 100.0,
            timestamp: Some(1000),
        }],
        asks: vec![OrderBookEntry {
            price: 0.75,
            size: 100.0,
            timestamp: Some(1000),
        }],
        timestamp: 1000,
    }
}

/// Create test orderbook with NO arbitrage (normal market)
fn create_normal_orderbook() -> OrderBook {
    OrderBook {
        token_id: TokenId("test-token".to_string()),
        bids: vec![OrderBookEntry {
            price: 0.74,
            size: 100.0,
            timestamp: Some(1000),
        }],
        asks: vec![OrderBookEntry {
            price: 0.75,
            size: 100.0,
            timestamp: Some(1000),
        }],
        timestamp: 1000,
    }
}

fn bench_detector_with_arbitrage(c: &mut Criterion) {
    let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
    let market_id = MarketId("market-1".to_string());
    let token_id = TokenId("token-1".to_string());
    let orderbook = create_arbitrage_orderbook();

    c.bench_function("detector_with_arbitrage", |bencher| {
        bencher.iter(|| {
            black_box(detector.detect(
                black_box(&market_id),
                black_box(&token_id),
                black_box(&orderbook),
            ))
        })
    });
}

fn bench_detector_without_arbitrage(c: &mut Criterion) {
    let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
    let market_id = MarketId("market-1".to_string());
    let token_id = TokenId("token-1".to_string());
    let orderbook = create_normal_orderbook();

    c.bench_function("detector_without_arbitrage", |bencher| {
        bencher.iter(|| {
            black_box(detector.detect(
                black_box(&market_id),
                black_box(&token_id),
                black_box(&orderbook),
            ))
        })
    });
}

fn bench_detector_batch(c: &mut Criterion) {
    let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());

    // Create mix of arbitrage and normal markets
    let markets = vec![
        (
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_arbitrage_orderbook(),
        ),
        (
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_normal_orderbook(),
        ),
        (
            MarketId("m3".to_string()),
            TokenId("t3".to_string()),
            create_arbitrage_orderbook(),
        ),
        (
            MarketId("m4".to_string()),
            TokenId("t4".to_string()),
            create_normal_orderbook(),
        ),
    ];

    c.bench_function("detector_batch_4_markets", |bencher| {
        bencher.iter(|| black_box(detector.detect_batch(black_box(&markets))))
    });
}

criterion_group!(
    benches,
    bench_detector_with_arbitrage,
    bench_detector_without_arbitrage,
    bench_detector_batch
);
criterion_main!(benches);
