# ADR-001: Rust + SIMD for High-Frequency Trading

## Status

**Accepted** - 2024-12-27

## Context

We are building a high-frequency trading (HFT) bot for Polymarket prediction markets. The bot needs to:

1. Detect arbitrage opportunities in real-time
2. Execute trades with minimal latency (<200ms total)
3. Process order book updates at high frequency (100+ updates/second)
4. Maintain memory safety and prevent race conditions
5. Run 24/7 with high reliability

### Language Options Evaluated

Based on COMPARATIVE_ANALYSIS.md of 7 Polymarket trading bots:

1. **TypeScript/Node.js** (4 implementations)
   - Now-Or-Neverr: Best architecture, 85/100
   - roswelly: 80/100
   - 0xsupersimon: 75/100 (beginner-friendly)
   - Average detection: ~100μs, execution: ~2.3s

2. **Python** (1 implementation)
   - P-x-J: 70/100 (educational, incomplete)
   - Slow performance, not suitable for HFT

3. **Rust** (1 implementation)
   - terauss: **95/100 - HIGHEST SCORED**
   - Detection: ~10μs, execution: ~150ms
   - SIMD optimization, lock-free concurrency

### Performance Requirements

For HFT, we need:
- **Detection latency**: < 10μs (to compete with other bots)
- **Execution latency**: < 200ms (before opportunity disappears)
- **Throughput**: 100+ opportunities/second
- **Memory**: Predictable, no GC pauses
- **Reliability**: 99.9% uptime

## Decision

**We will use Rust with SIMD optimization for the HFT bot.**

Specifically:
- **Language**: Rust 2021 edition
- **Runtime**: Tokio async runtime
- **Optimization**: SIMD vectorization using `wide` crate
- **Concurrency**: Lock-free atomics and crossbeam channels
- **Build**: Release profile with LTO and optimizations

## Rationale

### Performance Comparison (from analysis)

| Metric | TypeScript | Rust (Scalar) | Rust (SIMD) |
|--------|-----------|---------------|-------------|
| Detection | 100μs | 50μs | **10μs** ⚡ |
| Execution | 2.3s | 300ms | **150ms** ⚡ |
| Speedup | 1x | 7.6x | **15x** ⚡ |

### SIMD Advantage

The terauss implementation (95/100) uses SIMD to check **4 arbitrage types simultaneously**:

```rust
// Pseudocode from analysis
let bid_prices = f64x4::new([bid1, bid2, bid3, bid4]);
let ask_prices = f64x4::new([ask1, ask2, ask3, ask4]);
let spreads = bid_prices - ask_prices;  // 4 operations at once
```

**Result**: 4x speedup over scalar operations

### Memory Safety

Rust guarantees:
- **No data races** (enforced at compile time)
- **No null pointer dereferences**
- **No buffer overflows**
- **No use-after-free**

This is critical for a bot handling real money 24/7.

### No Garbage Collection

TypeScript/Node.js has unpredictable GC pauses:
- Can pause for 10-100ms during GC
- Ruins HFT latency requirements

Rust has:
- **Deterministic memory management** (ownership system)
- **No GC pauses**
- **Predictable performance**

### Lock-Free Concurrency

From terauss implementation:
```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use crossbeam::queue::ArrayQueue;

// Lock-free queue for orders
let order_queue = ArrayQueue::new(1000);

// Atomic operations for risk tracking
let daily_loss = AtomicU64::new(0);
daily_loss.fetch_add(loss_cents, Ordering::Release);
```

**Benefits**:
- No mutex contention
- Better CPU cache utilization
- Higher throughput

## Consequences

### Positive

1. **10x faster detection** than TypeScript implementations
2. **15x faster execution** end-to-end
3. **Memory safety** without runtime overhead
4. **No GC pauses** - predictable latency
5. **Concurrent execution** with lock-free data structures
6. **Production-grade** - Based on highest-scored implementation (95/100)
7. **Lower cloud costs** - Less CPU usage, smaller binaries

### Negative

1. **Steeper learning curve** - Rust has ownership/borrowing concepts
2. **Longer compile times** - 30s vs TypeScript's instant
3. **Less ecosystem** for trading - Fewer libraries than JavaScript
4. **Harder debugging** - Need to understand lifetimes and borrow checker

### Neutral

1. **Development time** - Slower initial development, but fewer bugs
2. **AI assistance** - Claude/Gemini trained on Rust, but more JS examples exist
3. **Deployment** - Single binary vs Node.js + dependencies

## Mitigating Negative Consequences

### 1. Learning Curve

**Solution**: Follow RUST_HFT_ROADMAP.md which provides:
- Week-by-week progression
- Code examples from terauss
- Types-first approach (easier to understand)

### 2. Compile Times

**Solution**:
```toml
# Use cargo-watch for incremental rebuilds
[profile.dev]
incremental = true

# Separate compilation units
[profile.dev]
codegen-units = 16  # Faster dev builds
```

```bash
# Watch mode for development
cargo watch -x check -x test
```

### 3. Ecosystem Gaps

**Solution**:
- Use `ethers-rs` for Ethereum (mature library)
- Use `reqwest` for HTTP (equivalent to axios)
- Use `tokio-tungstenite` for WebSocket
- Build custom CLOB client (only ~200 LOC based on terauss)

### 4. Debugging

**Solution**:
- Use `tracing` crate for structured logging
- Use `rust-analyzer` in VS Code for inline errors
- Write comprehensive tests first (TDD approach)

## Implementation Plan

### Phase 1: Types & Foundation (Week 1) ✅ COMPLETED
- [x] Project structure
- [x] Type definitions (Market, Order, Trade, Config)
- [x] Unit tests for types

### Phase 2: SIMD Arbitrage Detector (Week 2)
- [ ] Implement scalar detector first
- [ ] Add SIMD optimization
- [ ] Benchmark (target: <10μs)

### Phase 3: Risk Management (Week 3)
- [ ] Circuit breaker with atomics
- [ ] Position tracking
- [ ] Daily loss limits

### Phase 4: API Integration (Week 4)
- [ ] CLOB HTTP client
- [ ] Order signing with ethers-rs
- [ ] WebSocket manager

### Phase 5: Production (Week 5-8)
- [ ] Error handling & retry logic
- [ ] Monitoring & metrics
- [ ] CI/CD pipeline
- [ ] Dry-run testing

## Alternatives Considered

### Alternative 1: TypeScript (Now-Or-Neverr architecture)

**Pros**:
- Best TypeScript implementation (85/100)
- Clean architecture with abstract base classes
- Multi-strategy support
- Good for AI code generation

**Cons**:
- 10x slower detection (100μs vs 10μs)
- 15x slower execution (2.3s vs 150ms)
- Unpredictable GC pauses
- Not suitable for HFT

**Rejected because**: Performance requirements cannot be met. HFT demands sub-millisecond latency.

### Alternative 2: Rust without SIMD

**Pros**:
- Memory safety
- No GC pauses
- 50μs detection (5x faster than TypeScript)

**Cons**:
- Missing 4x SIMD speedup
- 50μs detection vs terauss's 10μs

**Rejected because**: SIMD provides competitive advantage. Implementation complexity is similar.

### Alternative 3: C++ with SIMD

**Pros**:
- Similar performance to Rust
- SIMD support
- Mature ecosystem

**Cons**:
- **No memory safety** - Easy to write bugs
- Manual memory management
- Undefined behavior possible
- Hard to maintain

**Rejected because**: Rust provides same performance with memory safety guarantees.

### Alternative 4: Hybrid (Rust core + TypeScript tooling)

**Pros**:
- Best of both worlds
- Rust for hot path (arbitrage detection)
- TypeScript for monitoring/dashboard

**Cons**:
- Complex build system
- FFI overhead
- Two languages to maintain

**Rejected because**: Rust can handle everything. Not worth the complexity.

## Performance Targets

Based on terauss (95/100) benchmark:

| Component | Target | Rust Implementation |
|-----------|--------|---------------------|
| Arbitrage detection | < 10μs | SIMD f64x4 vectorization |
| Order book processing | < 50μs | Zero-copy deserialization |
| Order execution | < 100ms | Async/await with Tokio |
| Total end-to-end | < 200ms | Concurrent execution |
| Memory usage | < 20MB | Stack allocation, no GC |
| Uptime | 99.9% | Circuit breaker, auto-recovery |

## Monitoring

We will track:
- Detection latency (P50, P95, P99)
- Execution latency (P50, P95, P99)
- Memory usage (resident set size)
- CPU usage
- Order fill rate
- Profit/loss

**Success Criteria**: Meet or exceed terauss performance (95/100 standard)

## References

- [COMPARATIVE_ANALYSIS.md](../../COMPARATIVE_ANALYSIS.md) - Analysis of 7 implementations
- [ANALYSIS_1_terauss_polymarket_kalshi.md](../../ANALYSIS_1_terauss_polymarket_kalshi.md) - Detailed terauss breakdown
- [BEST_CODE_SNIPPETS.md](../../BEST_CODE_SNIPPETS.md) - SIMD implementation examples
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Documentation](https://tokio.rs/)
- [SIMD in Rust](https://rust-lang.github.io/packed_simd/)

## Review

This ADR will be reviewed after Phase 2 (SIMD implementation) to validate performance targets are achievable.

**Expected Review Date**: Week 2 completion

---

**Decision Made By**: Development Team
**Date**: 2024-12-27
**Supersedes**: None
**Superseded By**: None (active)
