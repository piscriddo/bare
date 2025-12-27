# Phase 4 Implementation Plan: CLOB Integration + Tier 1 Optimizations

**Goal:** Build production-ready CLOB client with batch orders and all Tier 1 HFT optimizations baked in from day 1.

**Target Latency:** <200ms per arbitrage execution (detection + order submission)

---

## Overview

Phase 4 integrates with Polymarket CLOB API and implements all critical Tier 1 optimizations:

1. ✅ **Batch orders** (50% latency reduction: 400ms → 200ms)
2. ✅ **TCP_NODELAY** (40-200ms saved per request)
3. ✅ **Connection pooling** (eliminates TCP handshake overhead)
4. ✅ **Optimistic nonce** (100ms → 0ms for nonce lookup)
5. ✅ **Pre-computed EIP-712 hashes** (10-20μs saved per order)
6. ✅ **Comprehensive error handling** with rollback

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  CLOB Client (Phase 4)                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ HTTP Client │  │ Nonce Manager│  │ EIP-712 Hash │  │
│  │ (pooled +   │  │ (optimistic) │  │ (pre-compute)│  │
│  │ TCP_NODELAY)│  └──────────────┘  └──────────────┘  │
│  └─────────────┘                                       │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │         Batch Order Executor                    │  │
│  │  • Submit 2 orders in single HTTP request       │  │
│  │  • Check both succeeded                          │  │
│  │  • Rollback if partial fill                      │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │         Order Signer (EIP-712)                   │  │
│  │  • Pre-compute domain separator                  │  │
│  │  • Optimized signing (async)                     │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Files to Create

### 1. `src/clob/client.rs` (Core HTTP client)

**Responsibilities:**
- HTTP client with TCP_NODELAY and connection pooling
- Single order creation (fallback)
- Batch order creation (primary)
- Order cancellation (for rollback)
- Rate limiting and retry logic

**Key optimizations:**
```rust
use reqwest::Client;
use std::time::Duration;

pub struct ClobClient {
    client: Client,
    base_url: String,
    api_key: String,
    nonce_manager: NonceManager,
    signer: OrderSigner,
}

impl ClobClient {
    pub fn new(config: ClobConfig) -> Result<Self> {
        // TIER 1 OPTIMIZATION: TCP_NODELAY + connection pooling
        let client = Client::builder()
            .pool_max_idle_per_host(10)           // Keep 10 connections warm
            .pool_idle_timeout(Duration::from_secs(90))  // 90s keep-alive
            .tcp_nodelay(true)                    // CRITICAL: 40-200ms saved
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            base_url: config.base_url,
            api_key: config.api_key,
            nonce_manager: NonceManager::new(),
            signer: OrderSigner::new(config.private_key)?,
        })
    }

    /// Create batch orders (up to 15)
    /// Returns order hashes for successful orders
    pub async fn create_batch_orders(
        &self,
        params: &[CreateOrderParams],
    ) -> Result<BatchOrderResponse> {
        // Build batch request
        let orders: Vec<_> = params
            .iter()
            .map(|p| self.build_signed_order(p))
            .collect::<Result<Vec<_>>>()?;

        // Single HTTP request (200ms vs 400ms sequential)
        let response = self.client
            .post(&format!("{}/orders", self.base_url))
            .header("Authorization", &self.api_key)
            .json(&orders)
            .send()
            .await?;

        let result: BatchOrderResponse = response.json().await?;
        Ok(result)
    }

    /// Cancel an order (used for rollback)
    pub async fn cancel_order(&self, order_hash: &str) -> Result<()> {
        let response = self.client
            .delete(&format!("{}/orders/{}", self.base_url, order_hash))
            .header("Authorization", &self.api_key)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to cancel order: {}", response.status()))
        }
    }

    fn build_signed_order(&self, params: &CreateOrderParams) -> Result<PostOrder> {
        // TIER 1 OPTIMIZATION: Optimistic nonce (no API call)
        let nonce = self.nonce_manager.next_nonce();

        // TIER 1 OPTIMIZATION: Pre-computed EIP-712 hash
        let order = Order {
            salt: generate_salt(),
            maker: params.maker.clone(),
            signer: params.signer.clone(),
            taker: params.taker.clone(),
            token_id: params.token_id.clone(),
            maker_amount: params.maker_amount,
            taker_amount: params.taker_amount,
            expiration: params.expiration,
            nonce,
            fee_rate_bps: params.fee_rate_bps,
            side: params.side,
            signature_type: SignatureType::EIP712,
        };

        // Sign order (uses pre-computed domain separator)
        let signature = self.signer.sign_order(&order)?;

        Ok(PostOrder {
            order: order.with_signature(signature),
            order_type: params.order_type.clone(),
            owner: self.api_key.clone(),
        })
    }
}
```

**Performance targets:**
- HTTP request with pooling: ~100-150ms (vs 200-300ms cold)
- TCP_NODELAY benefit: 40-200ms saved
- Total single request: ~200ms

---

### 2. `src/clob/nonce_manager.rs` (Optimistic nonce)

**Responsibilities:**
- Track local nonce atomically
- Optimistic increment (no API call)
- Handle nonce conflicts with retry

**Key optimization:**
```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// Optimistic nonce manager
///
/// TIER 1 OPTIMIZATION: Eliminates 100ms API call per order
pub struct NonceManager {
    /// Current nonce (atomic for thread-safety)
    current_nonce: AtomicU64,

    /// Starting nonce (set on initialization)
    starting_nonce: u64,
}

impl NonceManager {
    pub fn new() -> Self {
        Self {
            current_nonce: AtomicU64::new(0),
            starting_nonce: 0,
        }
    }

    /// Initialize with current on-chain nonce
    pub async fn initialize(&self, client: &ClobClient) -> Result<()> {
        let nonce = client.fetch_current_nonce().await?;
        self.current_nonce.store(nonce, Ordering::SeqCst);
        tracing::info!("Nonce manager initialized at {}", nonce);
        Ok(())
    }

    /// Get next nonce (optimistic increment, no API call)
    ///
    /// PERFORMANCE: 0ms vs 100ms API call
    pub fn next_nonce(&self) -> u64 {
        self.current_nonce.fetch_add(1, Ordering::SeqCst)
    }

    /// Handle nonce conflict (reset to server value + 1)
    pub fn handle_conflict(&self, server_nonce: u64) {
        let current = self.current_nonce.load(Ordering::Acquire);
        if server_nonce >= current {
            self.current_nonce.store(server_nonce + 1, Ordering::SeqCst);
            tracing::warn!("Nonce conflict detected, reset to {}", server_nonce + 1);
        }
    }

    /// Get current nonce (non-incrementing read)
    pub fn current(&self) -> u64 {
        self.current_nonce.load(Ordering::Acquire)
    }
}
```

**Performance impact:**
- **Before:** 100ms API call per order
- **After:** 0ms (atomic increment)
- **Savings:** 100ms per order (200ms for 2-order arbitrage)

---

### 3. `src/clob/eip712.rs` (Pre-computed hashes)

**Responsibilities:**
- Pre-compute EIP-712 domain separator at startup
- Fast order hash computation
- Signature generation

**Key optimization:**
```rust
use ethers::types::{H256, U256};
use ethers::utils::keccak256;

/// EIP-712 order signer with pre-computed domain separator
///
/// TIER 1 OPTIMIZATION: Pre-compute domain separator (10-20μs saved)
pub struct OrderSigner {
    private_key: SigningKey,

    /// Pre-computed domain separator (computed once at startup)
    domain_separator: H256,
}

impl OrderSigner {
    pub fn new(private_key: String) -> Result<Self> {
        let key = SigningKey::from_str(&private_key)?;

        // OPTIMIZATION: Pre-compute domain separator once
        let domain_separator = Self::compute_domain_separator()?;

        tracing::info!("EIP-712 domain separator pre-computed: {:?}", domain_separator);

        Ok(Self {
            private_key: key,
            domain_separator,
        })
    }

    fn compute_domain_separator() -> Result<H256> {
        // EIP-712 domain separator for Polymarket CLOB
        let domain = keccak256(&encode(&[
            Token::String("EIP712Domain".to_string()),
            Token::String("Polymarket CLOB".to_string()),
            Token::String("1".to_string()),
            Token::Uint(U256::from(137)), // Polygon chain ID
            Token::Address(CLOB_CONTRACT_ADDRESS.parse()?),
        ]));

        Ok(H256::from_slice(&domain))
    }

    /// Sign order (uses pre-computed domain separator)
    pub fn sign_order(&self, order: &Order) -> Result<Signature> {
        // Compute order hash (uses pre-computed domain separator)
        let order_hash = self.hash_order(order)?;

        // Sign hash
        let signature = self.private_key.sign_prehash(&order_hash)?;

        Ok(signature)
    }

    fn hash_order(&self, order: &Order) -> Result<H256> {
        // OPTIMIZATION: Use pre-computed domain separator
        let struct_hash = keccak256(&encode_packed(&[
            // ... order fields
        ]));

        let digest = keccak256(&encode_packed(&[
            Token::String("\x19\x01".to_string()),
            Token::Bytes(self.domain_separator.as_bytes().to_vec()),
            Token::Bytes(struct_hash.to_vec()),
        ]));

        Ok(H256::from_slice(&digest))
    }
}
```

**Performance impact:**
- **Before:** Compute domain separator per order (~10-20μs)
- **After:** Use pre-computed value (~0μs)
- **Savings:** 10-20μs per order (20-40μs for 2-order arbitrage)

---

### 4. `src/clob/executor.rs` (Batch executor with rollback)

**Responsibilities:**
- Execute arbitrage with batch orders
- Verify both orders succeeded
- Rollback on partial fill
- Integrate with circuit breaker

**Critical implementation:**
```rust
use crate::core::risk::CircuitBreaker;
use crate::types::ArbitrageOpportunity;

pub struct ArbitrageExecutor {
    client: ClobClient,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ArbitrageExecutor {
    /// Execute arbitrage with batch orders + rollback
    pub async fn execute(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<ExecutionResult> {
        // Circuit breaker check
        if !self.circuit_breaker.can_execute() {
            return Err(anyhow!("Circuit breaker tripped"));
        }

        // Track position before execution
        self.circuit_breaker.open_position()?;
        self.circuit_breaker.open_position()?; // Two positions (BUY + SELL)

        // Build batch orders
        let orders = vec![
            CreateOrderParams {
                side: OrderSide::BUY,
                price: opportunity.ask_price,
                size: opportunity.max_size,
                order_type: OrderType::GTC,
                // ... other fields
            },
            CreateOrderParams {
                side: OrderSide::SELL,
                price: opportunity.bid_price,
                size: opportunity.max_size,
                order_type: OrderType::GTC,
                // ... other fields
            },
        ];

        // Execute batch (200ms total)
        let start = Instant::now();
        let response = self.client.create_batch_orders(&orders).await?;
        let latency = start.elapsed();

        tracing::info!("Batch order latency: {:?}", latency);

        // CRITICAL: Verify both orders succeeded
        let result = self.verify_and_rollback(&response).await?;

        // Update circuit breaker
        match &result {
            ExecutionResult::Success { pnl, .. } => {
                self.circuit_breaker.record_trade(*pnl)?;
            }
            ExecutionResult::PartialFill { .. } => {
                self.circuit_breaker.close_position();
                self.circuit_breaker.close_position();
                self.circuit_breaker.record_error();
            }
            ExecutionResult::Failed { .. } => {
                self.circuit_breaker.close_position();
                self.circuit_breaker.close_position();
                self.circuit_breaker.record_error();
            }
        }

        Ok(result)
    }

    /// Verify both orders succeeded, rollback if needed
    async fn verify_and_rollback(
        &self,
        response: &BatchOrderResponse,
    ) -> Result<ExecutionResult> {
        let buy_hash = response.order_hashes.get(0);
        let sell_hash = response.order_hashes.get(1);

        match (buy_hash, sell_hash) {
            (Some(buy), Some(sell)) => {
                // Both succeeded ✓
                tracing::info!("✅ Arbitrage executed: BUY={} SELL={}", buy, sell);
                Ok(ExecutionResult::Success {
                    buy_hash: buy.clone(),
                    sell_hash: sell.clone(),
                    pnl: 0.0, // Calculate from fills
                })
            }
            (Some(buy), None) => {
                // Only BUY succeeded - DANGER!
                tracing::error!("⚠️ One-sided fill! Only BUY succeeded, cancelling...");

                // Try to cancel BUY order
                match self.client.cancel_order(buy).await {
                    Ok(_) => {
                        tracing::info!("Rolled back BUY order: {}", buy);
                        Ok(ExecutionResult::PartialFill {
                            filled_order: buy.clone(),
                            rolled_back: true,
                        })
                    }
                    Err(e) => {
                        tracing::error!("❌ FAILED TO ROLLBACK: {}", e);
                        // CRITICAL: Manual intervention needed!
                        self.circuit_breaker.trip();
                        Err(anyhow!("Rollback failed: {}", e))
                    }
                }
            }
            (None, Some(sell)) => {
                // Only SELL succeeded - DANGER!
                tracing::error!("⚠️ One-sided fill! Only SELL succeeded, cancelling...");

                match self.client.cancel_order(sell).await {
                    Ok(_) => {
                        tracing::info!("Rolled back SELL order: {}", sell);
                        Ok(ExecutionResult::PartialFill {
                            filled_order: sell.clone(),
                            rolled_back: true,
                        })
                    }
                    Err(e) => {
                        tracing::error!("❌ FAILED TO ROLLBACK: {}", e);
                        self.circuit_breaker.trip();
                        Err(anyhow!("Rollback failed: {}", e))
                    }
                }
            }
            (None, None) => {
                // Both failed - safe
                tracing::warn!("Batch order failed: {}", response.errorMsg);
                Ok(ExecutionResult::Failed {
                    error: response.errorMsg.clone(),
                })
            }
        }
    }
}

pub enum ExecutionResult {
    Success {
        buy_hash: String,
        sell_hash: String,
        pnl: f64,
    },
    PartialFill {
        filled_order: String,
        rolled_back: bool,
    },
    Failed {
        error: String,
    },
}
```

**Safety guarantees:**
- ✅ Both orders succeed → arbitrage complete
- ✅ Only one succeeds → cancel immediately, trip circuit breaker if cancel fails
- ✅ Both fail → safe, no action needed
- ✅ Rollback failure → trip circuit breaker, alert operator

---

### 5. `src/clob/mod.rs` (Module exports)

```rust
//! CLOB client for Polymarket order execution
//!
//! Phase 4: Implements all Tier 1 HFT optimizations:
//! - Batch orders (50% latency reduction)
//! - TCP_NODELAY (40-200ms saved)
//! - Connection pooling (eliminates handshake overhead)
//! - Optimistic nonce (100ms → 0ms)
//! - Pre-computed EIP-712 hashes (10-20μs saved)

mod client;
mod nonce_manager;
mod eip712;
mod executor;

pub use client::ClobClient;
pub use nonce_manager::NonceManager;
pub use eip712::OrderSigner;
pub use executor::{ArbitrageExecutor, ExecutionResult};
```

---

## Testing Strategy

### Unit Tests

**`tests/clob_client_tests.rs`:**
- HTTP client configuration (TCP_NODELAY, pooling)
- Request serialization
- Response deserialization
- Error handling

**`tests/nonce_manager_tests.rs`:**
- Optimistic nonce increment
- Concurrent access (thread safety)
- Conflict handling
- Reset behavior

**`tests/eip712_tests.rs`:**
- Domain separator computation
- Order hash generation
- Signature verification
- Pre-computed values match

**`tests/executor_tests.rs`:**
- Both orders succeed → ExecutionResult::Success
- Only BUY succeeds → rollback SELL
- Only SELL succeeds → rollback BUY
- Both fail → ExecutionResult::Failed
- Rollback failure → circuit breaker trips

### Integration Tests

**`tests/integration/batch_orders.rs`:**
- Submit real batch order to testnet
- Verify latency <200ms
- Test rollback on simulated partial fill
- Circuit breaker integration

**Benchmarks:**

**`benches/clob_bench.rs`:**
```rust
fn bench_batch_order_creation(c: &mut Criterion) {
    c.bench_function("batch_order_2", |b| {
        b.iter(|| {
            // Benchmark full batch order creation
            // Target: <200ms total
        });
    });
}

fn bench_nonce_increment(c: &mut Criterion) {
    c.bench_function("optimistic_nonce", |b| {
        b.iter(|| {
            // Benchmark nonce increment
            // Target: <1μs
        });
    });
}

fn bench_eip712_signing(c: &mut Criterion) {
    c.bench_function("eip712_sign", |b| {
        b.iter(|| {
            // Benchmark order signing
            // Target: <50μs with pre-computed separator
        });
    });
}
```

---

## Performance Targets

| Component | Target | Tier 1 Optimization |
|-----------|--------|---------------------|
| HTTP request (pooled) | <150ms | TCP_NODELAY + pooling |
| Batch order (2 orders) | <200ms | Single HTTP request |
| Nonce lookup | <1μs | Optimistic atomic |
| EIP-712 signing | <50μs | Pre-computed separator |
| **Total arbitrage execution** | **<200ms** | **All optimizations** |

**Breakdown:**
```
Detection:        47ns    (Phase 2 SIMD)
Risk check:       1-5ns   (Phase 3 circuit breaker)
Nonce lookup:     <1μs    (Optimistic)
Order signing:    <100μs  (Pre-computed EIP-712) × 2
HTTP batch:       ~150ms  (TCP_NODELAY + pooling)
Verification:     <1ms    (Check response)
─────────────────────────────────────────────
TOTAL:            ~151ms  (49ms faster than 200ms target!)
```

---

## Implementation Timeline

### Day 1: Core HTTP Client
- [ ] Create `src/clob/client.rs`
- [ ] Configure reqwest with TCP_NODELAY + pooling
- [ ] Implement single order creation
- [ ] Implement batch order creation
- [ ] Implement order cancellation
- [ ] Unit tests for client

### Day 2: Optimistic Nonce + EIP-712
- [ ] Create `src/clob/nonce_manager.rs`
- [ ] Implement atomic nonce tracking
- [ ] Conflict handling
- [ ] Create `src/clob/eip712.rs`
- [ ] Pre-compute domain separator
- [ ] Order signing with pre-computed hash
- [ ] Unit tests for nonce + EIP-712

### Day 3: Batch Executor + Rollback
- [ ] Create `src/clob/executor.rs`
- [ ] Implement batch execution logic
- [ ] Implement rollback strategy
- [ ] Circuit breaker integration
- [ ] Unit tests for executor
- [ ] Rollback tests (all 4 scenarios)

### Day 4: Integration + Benchmarks
- [ ] Integration tests on testnet
- [ ] Measure actual latency
- [ ] Benchmark all components
- [ ] Verify <200ms target achieved
- [ ] Load testing (concurrent executions)

### Day 5: Error Handling + Documentation
- [ ] Comprehensive error handling
- [ ] Retry logic for transient errors
- [ ] Logging and telemetry
- [ ] Code documentation
- [ ] Example usage

---

## Success Criteria

- ✅ All unit tests passing (>50 tests)
- ✅ Integration tests passing on testnet
- ✅ Batch order latency <200ms (measured)
- ✅ Optimistic nonce <1μs (benchmarked)
- ✅ EIP-712 signing <50μs (benchmarked)
- ✅ Rollback works in all scenarios
- ✅ Circuit breaker integration working
- ✅ Zero one-sided positions in testing
- ✅ Comprehensive error handling
- ✅ Documentation complete

---

## Risk Mitigation

### Risk 1: Partial Fill (one order succeeds)
**Mitigation:** Immediate cancellation of successful order, circuit breaker trip if cancel fails

### Risk 2: Cancel Failure
**Mitigation:** Trip circuit breaker, alert operator, manual intervention

### Risk 3: Network Timeout
**Mitigation:** 10s timeout, automatic retry with exponential backoff

### Risk 4: Nonce Conflict
**Mitigation:** Detect conflict in error response, reset nonce manager, retry

### Risk 5: Rate Limiting
**Mitigation:** Track request rate, exponential backoff on 429 errors

### Risk 6: Order Delay
**Mitigation:** Handle "ORDER_DELAYED" status, implement delay retry logic

---

## Next Steps After Phase 4

Once Phase 4 is complete and validated:

1. **Phase 5:** WebSocket orderbook streaming (real-time arbitrage detection)
2. **Phase 6:** Tier 2 optimizations (SIMD JSON, zero-copy, parallel signing)
3. **Phase 7:** Production deployment + monitoring
4. **Phase 8:** Tier 3 optimizations (custom allocator, CPU pinning)

---

## Summary

Phase 4 is **THE most critical phase** because:
- **50% latency reduction** from batch orders (400ms → 200ms)
- **All Tier 1 optimizations** baked in from day 1
- **Proper risk management** with rollback and circuit breaker
- **Production-ready** error handling and monitoring

**This phase transforms the bot from a proof-of-concept to a competitive HFT arbitrage system!**
