///! Benchmarks for arbitrage detection
///!
///! Measures performance of scalar vs SIMD arbitrage detection.
///! Target: <10μs detection latency

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use polymarket_hft_bot::types::{MarketId, OrderBook, OrderBookEntry, TokenId};
use polymarket_hft_bot::core::arbitrage::{
    ArbitrageConfig, ScalarArbitrageDetector, SimdArbitrageDetector,
};

fn create_test_order_book(bid_price: f64, ask_price: f64, size: f64) -> OrderBook {
    OrderBook {
        token_id: TokenId("test-token".to_string()),
        bids: vec![OrderBookEntry {
            price: bid_price,
            size,
            timestamp: Some(1000),
        }],
        asks: vec![OrderBookEntry {
            price: ask_price,
            size,
            timestamp: Some(1000),
        }],
        timestamp: 1000,
    }
}

fn benchmark_scalar_single(c: &mut Criterion) {
    let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
    let market_id = MarketId("market-1".to_string());
    let token_id = TokenId("token-1".to_string());
    let order_book = create_test_order_book(0.75, 0.70, 100.0);

    c.bench_function("scalar_single_detection", |b| {
        b.iter(|| {
            detector.detect(
                black_box(&market_id),
                black_box(&token_id),
                black_box(&order_book),
            )
        });
    });
}

fn benchmark_scalar_batch(c: &mut Criterion) {
    let detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());

    for size in [10, 100, 1000].iter() {
        let markets: Vec<_> = (0..*size)
            .map(|i| {
                (
                    MarketId(format!("market-{}", i)),
                    TokenId(format!("token-{}", i)),
                    create_test_order_book(0.75, 0.70, 100.0),
                )
            })
            .collect();

        c.bench_with_input(
            BenchmarkId::new("scalar_batch", size),
            &markets,
            |b, markets| {
                b.iter(|| detector.detect_batch(black_box(markets)));
            },
        );
    }
}

fn benchmark_simd_batch(c: &mut Criterion) {
    let detector = SimdArbitrageDetector::new(ArbitrageConfig::default());

    // Test with exactly 4 (optimal for SIMD)
    let markets_4: [(MarketId, TokenId, OrderBook); 4] = [
        (
            MarketId("m1".to_string()),
            TokenId("t1".to_string()),
            create_test_order_book(0.75, 0.70, 100.0),
        ),
        (
            MarketId("m2".to_string()),
            TokenId("t2".to_string()),
            create_test_order_book(0.72, 0.68, 100.0),
        ),
        (
            MarketId("m3".to_string()),
            TokenId("t3".to_string()),
            create_test_order_book(0.80, 0.75, 100.0),
        ),
        (
            MarketId("m4".to_string()),
            TokenId("t4".to_string()),
            create_test_order_book(0.65, 0.60, 100.0),
        ),
    ];

    c.bench_function("simd_batch_4", |b| {
        b.iter(|| detector.detect_batch_simd(black_box(&markets_4)));
    });

    // Test with larger batches
    for size in [10, 100, 1000].iter() {
        let markets: Vec<_> = (0..*size)
            .map(|i| {
                (
                    MarketId(format!("market-{}", i)),
                    TokenId(format!("token-{}", i)),
                    create_test_order_book(0.75, 0.70, 100.0),
                )
            })
            .collect();

        c.bench_with_input(
            BenchmarkId::new("simd_batch", size),
            &markets,
            |b, markets| {
                b.iter(|| detector.detect_batch(black_box(markets)));
            },
        );
    }
}

fn benchmark_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_vs_simd");

    let scalar_detector = ScalarArbitrageDetector::new(ArbitrageConfig::default());
    let simd_detector = SimdArbitrageDetector::new(ArbitrageConfig::default());

    // Create test data
    let markets: Vec<_> = (0..100)
        .map(|i| {
            (
                MarketId(format!("market-{}", i)),
                TokenId(format!("token-{}", i)),
                create_test_order_book(0.75, 0.70, 100.0),
            )
        })
        .collect();

    group.bench_function("scalar_100", |b| {
        b.iter(|| scalar_detector.detect_batch(black_box(&markets)));
    });

    group.bench_function("simd_100", |b| {
        b.iter(|| simd_detector.detect_batch(black_box(&markets)));
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_scalar_single,
    benchmark_scalar_batch,
    benchmark_simd_batch,
    benchmark_comparison
);
criterion_main!(benches);
