//! Benchmark: Fixed-Point vs Floating-Point Performance
//!
//! **Target:** Demonstrate 5-10x speedup for price calculations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polymarket_hft_bot::utils::fixed_point::FixedPrice;

fn benchmark_fixed_point_operations(c: &mut Criterion) {
    c.bench_function("fixed_add", |bencher| {
        let a = FixedPrice::from_f64(0.75);
        let b = FixedPrice::from_f64(0.25);
        bencher.iter(|| black_box(a + b))
    });

    c.bench_function("f64_add", |bencher| {
        let a = 0.75f64;
        let b = 0.25f64;
        bencher.iter(|| black_box(a + b))
    });

    c.bench_function("fixed_multiply", |bencher| {
        let a = FixedPrice::from_f64(0.75);
        let b_val = FixedPrice::from_f64(2.0);
        bencher.iter(|| black_box(a.mul_price(b_val)))
    });

    c.bench_function("f64_multiply", |bencher| {
        let a = 0.75f64;
        let b_val = 2.0f64;
        bencher.iter(|| black_box(a * b_val))
    });

    c.bench_function("fixed_divide", |bencher| {
        let a = FixedPrice::from_f64(1.0);
        let b_val = FixedPrice::from_f64(2.0);
        bencher.iter(|| black_box(a.div_price(b_val)))
    });

    c.bench_function("f64_divide", |bencher| {
        let a = 1.0f64;
        let b_val = 2.0f64;
        bencher.iter(|| black_box(a / b_val))
    });

    c.bench_function("fixed_profit_margin", |bencher| {
        let bid = FixedPrice::from_f64(0.76);
        let ask = FixedPrice::from_f64(0.75);
        bencher.iter(|| black_box(FixedPrice::profit_margin(bid, ask)))
    });

    c.bench_function("f64_profit_margin", |bencher| {
        let bid = 0.76f64;
        let ask = 0.75f64;
        bencher.iter(|| {
            let spread = bid - ask;
            black_box(spread / ask)
        })
    });

    c.bench_function("fixed_arbitrage_check", |bencher| {
        let bid = FixedPrice::from_f64(0.76);
        let ask = FixedPrice::from_f64(0.75);
        let min_margin = FixedPrice::from_f64(0.02); // 2%
        let min_size = FixedPrice::from_f64(10.0);
        let size = FixedPrice::from_f64(100.0);

        bencher.iter(|| {
            if let Some(margin) = FixedPrice::profit_margin(bid, ask) {
                black_box(margin >= min_margin && size >= min_size)
            } else {
                black_box(false)
            }
        })
    });

    c.bench_function("f64_arbitrage_check", |bencher| {
        let bid = 0.76f64;
        let ask = 0.75f64;
        let min_margin = 0.02f64;
        let min_size = 10.0f64;
        let size = 100.0f64;

        bencher.iter(|| {
            let spread = bid - ask;
            if spread > 0.0 {
                let margin = spread / ask;
                black_box(margin >= min_margin && size >= min_size)
            } else {
                black_box(false)
            }
        })
    });
}

criterion_group!(benches, benchmark_fixed_point_operations);
criterion_main!(benches);
