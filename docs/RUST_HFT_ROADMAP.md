# Rust HFT Bot Implementation Roadmap

## Overview

This roadmap is specifically designed for building a **high-frequency trading (HFT) bot** in Rust for Polymarket, leveraging SIMD optimization and lock-free concurrency.

**Why Rust for HFT:**
- **Ultra-low latency**: ~10μs detection vs TypeScript's 100μs
- **SIMD vectorization**: 4x speedup on arbitrage detection
- **Zero-cost abstractions**: No runtime overhead
- **Memory safety**: No garbage collection pauses
- **Concurrent execution**: ~150ms total latency vs TypeScript's 2.3s

**Based on Analysis:**
- terauss implementation (95/100 score - highest ranked)
- SIMD optimization patterns
- Lock-free atomic data structures
- Production-grade Rust practices

---

## Project Structure

```
polymarket-hft-bot/
├── src/
│   ├── lib.rs                      # Library root
│   ├── main.rs                     # Binary entry point
│   ├── types/                      # Type definitions
│   │   ├── mod.rs
│   │   ├── market.rs
│   │   ├── order.rs
│   │   ├── trade.rs
│   │   └── config.rs
│   ├── core/                       # Core business logic
│   │   ├── mod.rs
│   │   ├── arbitrage/
│   │   │   ├── mod.rs
│   │   │   ├── detector.rs         # SIMD-optimized detector
│   │   │   └── tests.rs
│   │   ├── execution/
│   │   │   ├── mod.rs
│   │   │   ├── executor.rs
│   │   │   └── tests.rs
│   │   └── risk/
│   │       ├── mod.rs
│   │       ├── circuit_breaker.rs
│   │       └── tests.rs
│   ├── services/                   # External integrations
│   │   ├── mod.rs
│   │   ├── polymarket/
│   │   │   ├── mod.rs
│   │   │   ├── clob_client.rs
│   │   │   └── tests.rs
│   │   └── websocket/
│   │       ├── mod.rs
│   │       ├── manager.rs
│   │       └── tests.rs
│   ├── config/                     # Configuration
│   │   └── mod.rs
│   └── utils/                      # Utilities
│       ├── mod.rs
│       ├── logger/
│       │   └── mod.rs
│       └── math/
│           └── mod.rs
├── tests/                          # Integration tests
│   ├── integration_tests.rs
│   └── common/
│       └── mod.rs
├── benches/                        # Performance benchmarks
│   └── arbitrage_bench.rs
├── docs/
│   ├── architecture/
│   │   ├── ADR-001-rust-simd.md
│   │   └── ADR-002-arbitrage-strategy.md
│   └── guides/
├── .github/workflows/
│   ├── ci.yml
│   └── release.yml
├── Cargo.toml
├── Cargo.lock
├── rustfmt.toml
├── clippy.toml
└── README.md
```

---

## PHASE 0: Rust Development Setup (Day 1)

### 0.1 Install Rust Toolchain

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly (for SIMD)
rustup toolchain install nightly
rustup component add rustfmt clippy

# Set default to stable
rustup default stable
```

### 0.2 Development Tools

```bash
# Install cargo-watch for auto-rebuild
cargo install cargo-watch

# Install cargo-edit for managing dependencies
cargo install cargo-edit

# Install cargo-tarpaulin for coverage
cargo install cargo-tarpaulin

# Install cargo-audit for security audits
cargo install cargo-audit

# Install cargo-deny for dependency checks
cargo install cargo-deny
```

### 0.3 IDE Setup (VS Code)

**Extensions:**
- rust-analyzer
- CodeLLDB (debugging)
- crates (dependency management)
- Even Better TOML

---

## PHASE 1: Foundation & Type System (Week 1)

### ✅ Completed

- [x] Project initialization with Cargo
- [x] Directory structure
- [x] Cargo.toml with dependencies
- [x] Core type system:
  - Market types (MarketId, TokenId, Market, OrderBook)
  - Order types (OrderSide, OrderType, OrderStatus)
  - Trade types (Trade, Position, ArbitrageOpportunity)
  - Config types (BotConfig with validation)

### 1.1 Testing Setup

Create `rustfmt.toml`:
```toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
edition = "2021"
```

Create `clippy.toml`:
```toml
cognitive-complexity-threshold = 30
too-many-arguments-threshold = 8
```

### 1.2 Build and Test

```bash
# Build
cargo build

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check without building
cargo check
```

---

## PHASE 2: SIMD-Optimized Arbitrage Detector (Week 2)

### 2.1 Implement Detector

**File**: `src/core/arbitrage/detector.rs`

**Key Features from terauss:**
- SIMD vectorization using `wide` crate
- Checks 4 arbitrage types simultaneously:
  1. Bid-ask spread (internal)
  2. YES/NO pricing arbitrage
  3. Cross-market arbitrage
  4. Oracle deviation

**Performance Target**: < 10μs detection latency

### 2.2 Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrage_detection() {
        // Test cases
    }

    #[test]
    fn test_empty_order_book() {
        // Edge case
    }
}
```

### 2.3 Benchmarking

**File**: `benches/arbitrage_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polymarket_hft_bot::core::arbitrage::ArbitrageDetector;

fn benchmark_detection(c: &mut Criterion) {
    let detector = ArbitrageDetector::new(config);
    let order_book = create_test_order_book();

    c.bench_function("arbitrage detection", |b| {
        b.iter(|| {
            detector.detect_opportunity(
                black_box(&market_id),
                black_box(&order_book)
            )
        });
    });
}

criterion_group!(benches, benchmark_detection);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
```

---

## PHASE 3: Circuit Breaker & Risk Management (Week 3)

### 3.1 Circuit Breaker Pattern

**File**: `src/core/risk/circuit_breaker.rs`

**Features:**
- Track daily losses using atomic operations
- Max positions limit
- Consecutive errors tracking
- Auto-reset after cooldown period

**Key Implementation:**
```rust
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use parking_lot::RwLock;

pub struct CircuitBreaker {
    tripped: AtomicBool,
    consecutive_errors: AtomicU32,
    daily_loss: AtomicU64,  // Store as cents (u64)
    open_positions: AtomicU32,
    config: RiskConfig,
    last_reset: RwLock<std::time::Instant>,
}

impl CircuitBreaker {
    pub fn can_execute(&self) -> bool {
        !self.tripped.load(Ordering::Acquire)
    }

    pub fn record_trade(&self, pnl: f64) {
        // Atomic updates
    }

    pub fn trip(&self) {
        self.tripped.store(true, Ordering::Release);
    }
}
```

---

## PHASE 4: Polymarket CLOB Client (Week 4)

### 4.1 HTTP Client

**File**: `src/services/polymarket/clob_client.rs`

```rust
use reqwest::Client;
use serde_json::Value;
use anyhow::Result;

pub struct ClobClient {
    client: Client,
    base_url: String,
    signer: EthereumSigner,
}

impl ClobClient {
    pub async fn get_order_book(&self, token_id: &TokenId) -> Result<OrderBook> {
        let url = format!("{}/book?token_id={}", self.base_url, token_id);
        let response = self.client.get(&url).send().await?;
        let order_book: OrderBook = response.json().await?;
        Ok(order_book)
    }

    pub async fn create_order(&self, params: &CreateOrderParams) -> Result<OrderResponse> {
        // Sign order
        let signature = self.signer.sign_order(params).await?;

        // Submit order
        let response = self.client
            .post(&format!("{}/order", self.base_url))
            .json(&params)
            .header("Authorization", format!("Bearer {}", signature))
            .send()
            .await?;

        response.json().await.map_err(Into::into)
    }
}
```

---

## PHASE 5: WebSocket Manager (Week 5)

### 5.1 WebSocket with Auto-Reconnect

**File**: `src/services/websocket/manager.rs`

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

pub struct WebSocketManager {
    url: String,
    reconnect_interval: Duration,
    message_tx: mpsc::Sender<OrderBook>,
}

impl WebSocketManager {
    pub async fn start(&self) -> Result<()> {
        loop {
            match self.connect_and_listen().await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("WebSocket error: {:?}, reconnecting...", e);
                    tokio::time::sleep(self.reconnect_interval).await;
                }
            }
        }
    }

    async fn connect_and_listen(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Subscribe to order book updates
        write.send(Message::Text(r#"{"type":"subscribe","channel":"orderbook"}"#.to_string())).await?;

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    let order_book: OrderBook = serde_json::from_str(&text)?;
                    self.message_tx.send(order_book).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
```

---

## PHASE 6: Performance Optimization (Week 6)

### 6.1 SIMD Arbitrage Detection

**File**: `src/core/arbitrage/simd_detector.rs`

```rust
use wide::f64x4;

pub struct SimdArbitrageDetector {
    config: ArbitrageConfig,
}

impl SimdArbitrageDetector {
    /// Detect 4 arbitrage types simultaneously using SIMD
    pub fn detect_batch(&self, order_books: &[OrderBook; 4]) -> [Option<ArbitrageOpportunity>; 4] {
        // Load prices into SIMD registers
        let bid_prices = f64x4::new([
            order_books[0].best_bid().map(|b| b.price).unwrap_or(0.0),
            order_books[1].best_bid().map(|b| b.price).unwrap_or(0.0),
            order_books[2].best_bid().map(|b| b.price).unwrap_or(0.0),
            order_books[3].best_bid().map(|b| b.price).unwrap_or(0.0),
        ]);

        let ask_prices = f64x4::new([
            order_books[0].best_ask().map(|a| a.price).unwrap_or(1.0),
            order_books[1].best_ask().map(|a| a.price).unwrap_or(1.0),
            order_books[2].best_ask().map(|a| a.price).unwrap_or(1.0),
            order_books[3].best_ask().map(|a| a.price).unwrap_or(1.0),
        ]);

        // Vectorized comparison: bid > ask
        let spread = bid_prices - ask_prices;
        let profit_margin = spread / ask_prices;

        // Extract results
        let margins: [f64; 4] = profit_margin.into();

        // Create opportunities
        let mut opportunities = [None, None, None, None];
        for (i, margin) in margins.iter().enumerate() {
            if *margin >= self.config.min_profit_margin {
                opportunities[i] = self.create_opportunity(&order_books[i], *margin);
            }
        }

        opportunities
    }
}
```

### 6.2 Benchmarking Results

**Target Performance:**
- Detection latency: < 10μs (SIMD) vs 100μs (scalar)
- Total execution: < 150ms (Rust) vs 2.3s (TypeScript)
- 4x speedup with SIMD vectorization

Run benchmarks:
```bash
cargo bench --bench arbitrage_bench
```

---

## PHASE 7: CI/CD & Production (Week 7-8)

### 7.1 GitHub Actions CI

**File**: `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
          override: true

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Run tests
        run: cargo test --all-features

      - name: Run benchmarks
        run: cargo bench --no-run

      - name: Security audit
        run: |
          cargo install cargo-audit
          cargo audit
```

### 7.2 Release Profile Optimization

Already configured in `Cargo.toml`:
```toml
[profile.release]
opt-level = 3          # Maximum optimization
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization
strip = true           # Strip symbols
```

Build release:
```bash
cargo build --release
```

---

## Key Rust Features for HFT

### 1. Lock-Free Concurrency

```rust
use std::sync::Arc;
use crossbeam::queue::ArrayQueue;

pub struct OrderQueue {
    queue: Arc<ArrayQueue<Order>>,
}

impl OrderQueue {
    pub fn push(&self, order: Order) -> Result<(), Order> {
        self.queue.push(order)
    }

    pub fn pop(&self) -> Option<Order> {
        self.queue.pop()
    }
}
```

### 2. Zero-Copy Deserialization

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OrderBookUpdate<'a> {
    #[serde(borrow)]
    pub token_id: &'a str,
    pub bids: Vec<OrderBookEntry>,
    pub asks: Vec<OrderBookEntry>,
}
```

### 3. Async/Await with Tokio

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let bot = TradingBot::new(config).await?;

    // Spawn concurrent tasks
    let arbitrage_task = tokio::spawn(async move {
        bot.run_arbitrage().await
    });

    let monitoring_task = tokio::spawn(async move {
        bot.monitor_positions().await
    });

    tokio::try_join!(arbitrage_task, monitoring_task)?;

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests (90%+ coverage)

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html
```

### Integration Tests

**File**: `tests/integration_tests.rs`

```rust
#[tokio::test]
async fn test_end_to_end_arbitrage() {
    let config = BotConfig::default();
    let bot = TradingBot::new(config).await.unwrap();

    // Test full arbitrage flow
}
```

### Property-Based Testing

```toml
[dev-dependencies]
proptest = "1.4"
```

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_arbitrage_properties(bid in 0.0..1.0, ask in 0.0..1.0) {
        // Property: bid <= ask should not create opportunity
        if bid <= ask {
            assert!(ArbitrageOpportunity::new(...).is_none());
        }
    }
}
```

---

## Performance Targets

| Metric | TypeScript | Rust (Scalar) | Rust (SIMD) |
|--------|-----------|---------------|-------------|
| Detection latency | 100μs | 50μs | **10μs** |
| Execution latency | 2.3s | 300ms | **150ms** |
| Memory usage | Variable (GC) | 10MB | **10MB** |
| CPU usage | High | Medium | **Low** |
| Throughput | 10 ops/s | 100 ops/s | **400 ops/s** |

---

## Production Deployment

### Docker

**Dockerfile**:
```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/polymarket_hft_bot /usr/local/bin/

CMD ["polymarket_hft_bot"]
```

Build and run:
```bash
docker build -t polymarket-hft-bot .
docker run -e BOT__DRY_RUN=true polymarket-hft-bot
```

---

## Next Steps

1. **Implement SIMD detector** (Week 2)
2. **Add circuit breaker** (Week 3)
3. **Build CLOB client** (Week 4)
4. **WebSocket integration** (Week 5)
5. **Performance tuning** (Week 6)
6. **Production hardening** (Week 7-8)
7. **Dry-run testing** (Week 9)
8. **Live deployment** (Week 10)

---

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [SIMD Programming](https://rust-lang.github.io/packed_simd/packed_simd_2/)
- [Polymarket API Docs](https://docs.polymarket.com)
- [terauss Implementation](https://github.com/terauss/polymarket-kalshi-arb)
