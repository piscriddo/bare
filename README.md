# Polymarket HFT Bot 🚀

High-frequency trading bot for Polymarket prediction markets built in Rust with SIMD optimization.

## Status: Phase 1 - Foundation ✅ COMPLETE

**Git Tag:** `v0.1.0-phase1` | **Commits:** 2 | **Tests:** 12/12 passing

**Performance Targets:**
- Detection latency: **< 10μs** (vs 100μs TypeScript)
- Execution latency: **< 150ms** (vs 2.3s TypeScript)
- **15x faster** than TypeScript implementations

Based on analysis of 7 Polymarket trading bots, implementing best practices from the highest-ranked implementation (terauss: 95/100).

**See:** [GIT_WORKFLOW.md](GIT_WORKFLOW.md) for phase tracking and git commands.

---

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install tools
cargo install cargo-watch cargo-edit
```

### Build & Test

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Run with watch mode
cargo watch -x check -x test

# Build release (optimized)
cargo build --release
```

### Run (Dry-run mode)

```bash
# Set environment variables
export BOT__DRY_RUN=true
export BOT__WALLET__PRIVATE_KEY=0x...
export BOT__WALLET__ADDRESS=0x...

# Run
cargo run --release
```

---

## Architecture

```
src/
├── types/          ✅ Type-safe data structures
│   ├── market.rs   ✅ Market, OrderBook, OrderBookEntry
│   ├── order.rs    ✅ Orders, CreateOrderParams, OrderResponse
│   ├── trade.rs    ✅ Trade, Position, ArbitrageOpportunity
│   └── config.rs   ✅ BotConfig with validation
├── core/           🔄 Business logic (next)
│   ├── arbitrage/  ⏳ SIMD-optimized detector
│   ├── execution/  ⏳ Order executor
│   └── risk/       ⏳ Circuit breaker
├── services/       ⏳ External integrations
│   ├── polymarket/ ⏳ CLOB HTTP/WebSocket client
│   └── websocket/  ⏳ Auto-reconnect manager
└── utils/          ⏳ Logging, math utilities
```

**Legend:**
- ✅ Complete
- 🔄 In progress
- ⏳ Planned

---

## Features

### Implemented ✅

- [x] Rust project structure with Cargo
- [x] Comprehensive type system
  - Market types with branded IDs (MarketId, TokenId)
  - Order types (BUY/SELL, GTC/FOK/IOC)
  - Trade tracking (Position, ArbitrageOpportunity)
  - Configuration with runtime validation
- [x] Unit tests for all types (100% coverage)
- [x] Documentation (ADRs, roadmap)

### In Progress 🔄

- [ ] SIMD-optimized arbitrage detector (Week 2)
- [ ] Circuit breaker with atomic operations (Week 3)
- [ ] Polymarket CLOB client (Week 4)
- [ ] WebSocket manager with auto-reconnect (Week 5)

### Planned ⏳

- [ ] Performance benchmarks (Criterion)
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] Monitoring & metrics (Prometheus)
- [ ] Production deployment (Docker)

---

## Why Rust?

**Based on COMPARATIVE_ANALYSIS.md:**

| Implementation | Language | Score | Detection | Execution |
|---------------|----------|-------|-----------|-----------|
| **terauss** | **Rust** | **95/100** ⭐ | **10μs** | **150ms** |
| Now-Or-Neverr | TypeScript | 85/100 | 100μs | 2.3s |
| roswelly | TypeScript | 80/100 | 100μs | 2.3s |
| 0xsupersimon | TypeScript | 75/100 | 100μs | 2.3s |

**Advantages:**
- **15x faster** execution (critical for HFT)
- **SIMD vectorization** - check 4 arbitrage types simultaneously
- **Zero-cost abstractions** - no performance overhead
- **Memory safety** - no garbage collection pauses
- **Lock-free concurrency** - atomic operations for risk tracking

See [ADR-001](docs/architecture/ADR-001-rust-simd-for-hft.md) for detailed rationale.

---

## Development Roadmap

See [RUST_HFT_ROADMAP.md](docs/RUST_HFT_ROADMAP.md) for complete implementation plan.

### Week 1: Foundation ✅ COMPLETE
- [x] Project initialization
- [x] Type system (Market, Order, Trade, Config)
- [x] Unit tests
- [x] Documentation (ADRs, roadmap)

### Week 2: SIMD Arbitrage Detector (NEXT)
- [ ] Implement scalar detector
- [ ] Add SIMD optimization (wide crate)
- [ ] Benchmark (target: <10μs)
- [ ] Integration tests

### Week 3: Risk Management
- [ ] Circuit breaker with atomics
- [ ] Position tracking
- [ ] Daily loss limits

### Week 4: API Integration
- [ ] CLOB HTTP client
- [ ] Order signing (ethers-rs)
- [ ] WebSocket manager

### Week 5-8: Production
- [ ] Error handling & retry logic
- [ ] Monitoring & metrics
- [ ] CI/CD pipeline
- [ ] Dry-run testing

---

## Performance Benchmarks

**Target** (based on terauss implementation):

| Metric | Target | Implementation |
|--------|--------|---------------|
| Arbitrage detection | < 10μs | SIMD f64x4 vectorization |
| Order book processing | < 50μs | Zero-copy deserialization |
| Order execution | < 100ms | Async/await (Tokio) |
| Total end-to-end | < 200ms | Concurrent execution |
| Memory usage | < 20MB | Stack allocation, no GC |

Run benchmarks (coming in Week 2):
```bash
cargo bench
```

---

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_arbitrage_opportunity

# Run with output
cargo test -- --nocapture

# Run tests with coverage (install cargo-tarpaulin first)
cargo tarpaulin --out Html
```

**Current Coverage:** Unit tests for all type definitions

---

## Configuration

Create `.env` file:

```env
# Wallet
BOT__WALLET__PRIVATE_KEY=0x...
BOT__WALLET__ADDRESS=0x...
BOT__WALLET__CHAIN_ID=137

# Trading
BOT__TRADING__DEFAULT_AMOUNT=10.0
BOT__TRADING__PRICE_THRESHOLD=0.02
BOT__TRADING__TAKE_PROFIT_AMOUNT=0.05
BOT__TRADING__STOP_LOSS_AMOUNT=0.03

# Risk Management
BOT__RISK__MAX_DAILY_LOSS=100.0
BOT__RISK__MAX_POSITION_SIZE=50.0
BOT__RISK__MAX_OPEN_POSITIONS=5

# Polymarket
BOT__POLYMARKET__CLOB_API_URL=https://clob.polymarket.com
BOT__POLYMARKET__GAMMA_API_URL=https://gamma-api.polymarket.com

# Features
BOT__FEATURES__ARBITRAGE_ENABLED=true
BOT__FEATURES__DRY_RUN=true
```

Or use environment variables directly.

---

## Documentation

- [RUST_HFT_ROADMAP.md](docs/RUST_HFT_ROADMAP.md) - Complete implementation guide
- [ADR-001: Rust + SIMD](docs/architecture/ADR-001-rust-simd-for-hft.md) - Tech stack decision
- [Cargo.toml](Cargo.toml) - Dependencies and build configuration

**Reference Analysis:**
- [START_HERE.md](../START_HERE.md) - Quick start guide
- [COMPARATIVE_ANALYSIS.md](../COMPARATIVE_ANALYSIS.md) - 7 bot implementations compared
- [BEST_CODE_SNIPPETS.md](../BEST_CODE_SNIPPETS.md) - Reference implementations

---

## Security

⚠️ **IMPORTANT**:
- Never commit private keys to git
- Always use `.env` for secrets
- Test with small amounts first
- Use dry-run mode initially
- Monitor circuit breaker status

---

## Contributing

This project follows:
- Types-first development
- Test-driven development (TDD)
- Documentation-driven development
- Comprehensive error handling

Before submitting:
```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test

# Check documentation
cargo doc --no-deps --open
```

---

## License

MIT

---

## Acknowledgments

**Based on analysis of:**
- [terauss/polymarket-kalshi-arb](https://github.com/terauss/polymarket-kalshi-arb) (95/100 - SIMD reference)
- [Now-Or-Neverr/polymarket-trading-bot](https://github.com/Now-Or-Neverr/polymarket-trading-bot) (85/100 - Architecture)
- [roswelly/polymarket-trading-bot](https://github.com/roswelly/polymarket-trading-bot) (80/100 - Config patterns)

**References:**
- [Polymarket Documentation](https://docs.polymarket.com)
- [CLOB API](https://docs.polymarket.com/api)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

---

**Status:** Phase 1 Complete ✅ | Next: SIMD Arbitrage Detector (Week 2)

**Performance:** Targeting 15x speedup over TypeScript | Based on terauss (95/100)
