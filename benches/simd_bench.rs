//! Benchmark: SIMD Arbitrage Detection Performance
//!
//! Compares f64x4 vs u64x4 fixed-point SIMD performance.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polymarket_hft_bot::core::arbitrage::{ArbitrageConfig, SimdArbitrageDetector};
use polymarket_hft_bot::types::{MarketId, OrderBook, OrderBookEntry, TokenId};

/// Create test orderbook with arbitrage opportunity
fn create_arbitrage_orderbook(token_id: &str) -> OrderBook {
    OrderBook {
        token_id: TokenId(token_id.to_string()),
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
fn create_normal_orderbook(token_id: &str) -> OrderBook {
    OrderBook {
        token_id: TokenId(token_id.to_string()),
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

fn bench_simd_fixed_batch(c: &mut Criterion) {
    let detector = SimdArbitrageDetector::new(ArbitrageConfig::default());

    let markets: [(MarketId, TokenId, OrderBook); 4] = [
        (
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_arbitrage_orderbook("t1"),
        ),
        (
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_normal_orderbook("t2"),
        ),
        (
            MarketId("m3".to_string()),
            TokenId("t3".to_string()),
            create_arbitrage_orderbook("t3"),
        ),
        (
            MarketId("m4".to_string()),
            TokenId("t4".to_string()),
            create_normal_orderbook("t4"),
        ),
    ];

    c.bench_function("simd_fixed_batch_4_markets", |bencher| {
        bencher.iter(|| black_box(detector.detect_batch_simd_fixed(black_box(&markets))))
    });
}

fn bench_simd_f64_batch(c: &mut Criterion) {
    let detector = SimdArbitrageDetector::new(ArbitrageConfig::default());

    let markets: [(MarketId, TokenId, OrderBook); 4] = [
        (
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_arbitrage_orderbook("t1"),
        ),
        (
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_normal_orderbook("t2"),
        ),
        (
            MarketId("m3".to_string()),
            TokenId("t3".to_string()),
            create_arbitrage_orderbook("t3"),
        ),
        (
            MarketId("m4".to_string()),
            TokenId("t4".to_string()),
            create_normal_orderbook("t4"),
        ),
    ];

    c.bench_function("simd_f64_batch_4_markets", |bencher| {
        bencher.iter(|| black_box(detector.detect_batch_simd(black_box(&markets))))
    });
}

fn bench_simd_fixed_all_arbitrage(c: &mut Criterion) {
    let detector = SimdArbitrageDetector::new(ArbitrageConfig::default());

    // All 4 markets have arbitrage
    let markets: [(MarketId, TokenId, OrderBook); 4] = [
        (
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_arbitrage_orderbook("t1"),
        ),
        (
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_arbitrage_orderbook("t2"),
        ),
        (
            MarketId("m3".to_string()),
            TokenId("t3".to_string()),
            create_arbitrage_orderbook("t3"),
        ),
        (
            MarketId("m4".to_string()),
            TokenId("t4".to_string()),
            create_arbitrage_orderbook("t4"),
        ),
    ];

    c.bench_function("simd_fixed_all_arbitrage", |bencher| {
        bencher.iter(|| black_box(detector.detect_batch_simd_fixed(black_box(&markets))))
    });
}

fn bench_simd_fixed_no_arbitrage(c: &mut Criterion) {
    let detector = SimdArbitrageDetector::new(ArbitrageConfig::default());

    // None have arbitrage (fast path)
    let markets: [(MarketId, TokenId, OrderBook); 4] = [
        (
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_normal_orderbook("t1"),
        ),
        (
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_normal_orderbook("t2"),
        ),
        (
            MarketId("m3".to_string()),
            TokenId("t3".to_string()),
            create_normal_orderbook("t3"),
        ),
        (
            MarketId("m4".to_string()),
            TokenId("t4".to_string()),
            create_normal_orderbook("t4"),
        ),
    ];

    c.bench_function("simd_fixed_no_arbitrage", |bencher| {
        bencher.iter(|| black_box(detector.detect_batch_simd_fixed(black_box(&markets))))
    });
}

criterion_group!(
    benches,
    bench_simd_fixed_batch,
    bench_simd_f64_batch,
    bench_simd_fixed_all_arbitrage,
    bench_simd_fixed_no_arbitrage
);
criterion_main!(benches);
