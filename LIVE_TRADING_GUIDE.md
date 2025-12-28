# Live Trading Guide - Starting with $20

## ⚠️ IMPORTANT WARNINGS

**Before You Start:**
- 🚨 **You will likely LOSE money** while learning
- 🚨 **$20 is too small** for meaningful profits (fees will eat most gains)
- 🚨 **No stop-loss automation** yet (you need to monitor manually)
- 🚨 **No position reconciliation** (restart = lost state)
- 🚨 **Competition is fierce** (other bots are faster/smarter)

**This is a LEARNING EXERCISE, not a money-making strategy!**

---

## Prerequisites

### 1. Polymarket Account Setup
- [ ] Create account at https://polymarket.com
- [ ] Complete KYC verification (required for trading)
- [ ] Fund account with USDC on Polygon network

### 2. Wallet Requirements
- [ ] Polygon (MATIC) wallet with private key
- [ ] At least **$25 total:**
  - $20 USDC for trading
  - $5 MATIC for gas fees (transactions)

### 3. Get USDC on Polygon

**Option A: Bridge from Ethereum**
1. Buy USDC on Ethereum (Coinbase, Kraken, etc.)
2. Bridge to Polygon using https://wallet.polygon.technology/

**Option B: Buy directly on Polygon**
1. Use Crypto.com or other exchange supporting Polygon
2. Withdraw USDC directly to Polygon network

**Option C: On-ramp (easiest)**
1. Use Polymarket's built-in on-ramp
2. Buy USDC directly with credit card

---

## Configuration Steps

### Step 1: Get Your Wallet Info

**Extract Private Key (MetaMask example):**
1. Open MetaMask
2. Click 3 dots → Account Details → Export Private Key
3. Enter password
4. **COPY PRIVATE KEY** (starts with `0x...`)

**Get Wallet Address:**
1. Click on account name in MetaMask
2. Copy address (starts with `0x...`)

### Step 2: Create Environment File

Create `.env` file in project root:

```bash
# Wallet Configuration
BOT__WALLET__PRIVATE_KEY=0xYOUR_PRIVATE_KEY_HERE
BOT__WALLET__ADDRESS=0xYOUR_WALLET_ADDRESS_HERE
BOT__WALLET__CHAIN_ID=137  # Polygon mainnet

# Trading Configuration
BOT__TRADING__DEFAULT_AMOUNT=5.0         # Start VERY small ($5 per trade)
BOT__TRADING__PRICE_THRESHOLD=0.01       # 1% minimum profit (be realistic!)
BOT__TRADING__TAKE_PROFIT_AMOUNT=0.02    # Take profit at 2%
BOT__TRADING__STOP_LOSS_AMOUNT=0.02      # Stop loss at 2%

# Risk Management (CRITICAL!)
BOT__RISK__MAX_DAILY_LOSS=10.0           # Stop after losing $10/day
BOT__RISK__MAX_POSITION_SIZE=10.0        # Max $10 per position
BOT__RISK__MAX_OPEN_POSITIONS=2          # Max 2 positions at once

# Polymarket API
BOT__POLYMARKET__CLOB_API_URL=https://clob.polymarket.com
BOT__POLYMARKET__GAMMA_API_URL=https://gamma-api.polymarket.com

# Features
BOT__FEATURES__ARBITRAGE_ENABLED=true
BOT__FEATURES__DRY_RUN=false             # SET TO false FOR LIVE TRADING!

# Logging (recommended)
RUST_LOG=info,polymarket_hft_bot=debug
```

### Step 3: Test Configuration

**First, test in DRY-RUN mode:**

```bash
# Set DRY_RUN=true in .env first!
BOT__FEATURES__DRY_RUN=true

# Run the bot
cargo run --release

# You should see:
# - "Dry-run mode: ENABLED" in logs
# - WebSocket connections
# - Arbitrage detection (if any)
# - NO actual orders placed
```

---

## Safety Checklist Before Going Live

### Pre-Flight Checks
- [ ] Wallet has $20 USDC on Polygon
- [ ] Wallet has $5 MATIC for gas fees
- [ ] Private key is correct (test with dry-run first)
- [ ] Risk limits are conservative (MAX_DAILY_LOSS=10, MAX_POSITION_SIZE=10)
- [ ] You understand the bot will trade automatically
- [ ] You have a way to kill the process (Ctrl+C)
- [ ] You're watching the terminal output
- [ ] You accept you might lose money

### Risk Management Verification
```bash
# Verify risk settings before going live
grep "RISK" .env

# Should show:
# BOT__RISK__MAX_DAILY_LOSS=10.0
# BOT__RISK__MAX_POSITION_SIZE=10.0
# BOT__RISK__MAX_OPEN_POSITIONS=2
```

---

## Going Live (3 Stages)

### Stage 1: Observation Mode (Recommended First)
**Duration: 1-2 hours**
**Goal: See what opportunities exist**

```bash
# Keep DRY_RUN=true
BOT__FEATURES__DRY_RUN=true

cargo run --release

# Watch for:
# - How often arbitrage is detected
# - Typical profit margins
# - Market volatility
# - Execution speed estimates
```

**Expected Reality:**
- You'll probably see **ZERO arbitrage opportunities**
- Markets are very efficient
- Other bots are faster
- This is NORMAL and GOOD TO KNOW!

---

### Stage 2: Micro Trading (If Proceeding)
**Duration: 1 day**
**Amount: $2-5 per trade**

```bash
# .env changes:
BOT__FEATURES__DRY_RUN=false
BOT__TRADING__DEFAULT_AMOUNT=2.0        # $2 per trade
BOT__RISK__MAX_DAILY_LOSS=5.0           # Stop at $5 loss
BOT__RISK__MAX_POSITION_SIZE=5.0
BOT__RISK__MAX_OPEN_POSITIONS=1         # Only 1 position

cargo run --release
```

**Monitor Closely:**
```bash
# Keep terminal open, watch for:
# - "🎯 ARBITRAGE OPPORTUNITY DETECTED"
# - "✅ Order placed successfully"
# - "❌ Order failed"
# - "🚨 Circuit breaker TRIPPED"

# Manual kill if needed:
# Ctrl + C (or kill the process)
```

---

### Stage 3: Normal Trading (If Successful)
**Duration: Ongoing**
**Amount: $5-10 per trade**

```bash
# .env changes:
BOT__TRADING__DEFAULT_AMOUNT=5.0        # $5 per trade
BOT__RISK__MAX_DAILY_LOSS=10.0          # Stop at $10 loss
BOT__RISK__MAX_POSITION_SIZE=10.0
BOT__RISK__MAX_OPEN_POSITIONS=2         # Max 2 positions

cargo run --release
```

---

## What to Expect (Reality Check)

### Likely Scenarios

**Scenario 1: No Arbitrage Found (90% probability)**
```
🔴 Connected to Polymarket WebSocket
📊 Monitoring 5 markets...
⏳ Waiting for arbitrage opportunities...
⏳ Waiting for arbitrage opportunities...
⏳ Waiting for arbitrage opportunities...
(nothing happens for hours)
```

**Why?**
- Markets are very efficient
- Other bots are faster (sub-millisecond execution)
- Our bot is still in Rust (fast but not colocated)
- Fee structure makes most arbitrage unprofitable

**What to do:**
- This is NORMAL
- Don't expect to make money
- Consider it a success if you see the bot working correctly
- Learn about market dynamics

---

**Scenario 2: Rare Arbitrage Detected (9% probability)**
```
🎯 ARBITRAGE OPPORTUNITY DETECTED!
   Market: Trump 2024 Wins
   Profit: 1.2% margin
   Size: $5.00
📤 Placing orders...
❌ Order rejected: Price moved (someone else was faster)
```

**Why?**
- You detected it, but so did 10 other bots
- They executed faster (maybe colocated)
- By the time your order arrived, price moved

**What to do:**
- This is still good! Your detection works!
- You're competing with professional firms
- Consider lowering PRICE_THRESHOLD to catch more opportunities

---

**Scenario 3: Successful Trade (1% probability)**
```
🎯 ARBITRAGE OPPORTUNITY DETECTED!
   Market: Bitcoin $100k by EOY
   Profit: 2.5% margin
   Size: $5.00
📤 Placing orders...
✅ BUY order filled: $5.00 @ 0.75
✅ SELL order filled: $5.00 @ 0.78
💰 Profit: $0.15 (3% after fees)
```

**Why you succeeded:**
- Got lucky with timing
- Other bots were down or slower
- Market had temporary inefficiency

**Reality check:**
- Fees ate most profit ($0.15 on $5 = 3%)
- Need to do this 100x to make meaningful money
- One bad trade can wipe out 10 good trades

---

## Monitoring & Management

### Real-Time Monitoring

**Terminal Output:**
```bash
# Key things to watch:
[INFO] WebSocket connected           # Good: streaming data
[DEBUG] Arbitrage detected: 1.2%     # Good: opportunities exist
[INFO] Order placed: BUY $5.00       # Good: executing trades
[WARN] Circuit breaker tripped       # WARNING: hit risk limit
[ERROR] API error: rate limited      # ERROR: too many requests
```

### Manual Intervention

**Stop the bot:**
```bash
# Press Ctrl+C in terminal
# Bot should shutdown gracefully
# (Currently no position reconciliation, so restart loses state)
```

**Check Polymarket positions manually:**
1. Go to https://polymarket.com/activity
2. Check open positions
3. Manually close if needed

### Daily Checklist

**End of Day:**
- [ ] Check total P&L on Polymarket
- [ ] Review terminal logs for errors
- [ ] Check circuit breaker trips
- [ ] Verify no orphaned positions
- [ ] Calculate actual profit after fees

---

## Common Issues & Solutions

### Issue 1: "TLS handshake failed"
**Solution:**
```bash
# Check internet connection
ping polymarket.com

# Verify TLS dependencies
cargo clean && cargo build --release
```

### Issue 2: "Order rejected: Insufficient balance"
**Solution:**
```bash
# Check USDC balance on Polygon
# Make sure you have both USDC and MATIC
```

### Issue 3: "Nonce conflict"
**Solution:**
```bash
# Restart the bot (nonce manager will re-sync)
# Wait 30 seconds before restarting
```

### Issue 4: "Circuit breaker tripped"
**Solution:**
```bash
# This is GOOD - risk management working!
# Review what happened
# Adjust risk parameters if needed
# Restart bot to reset (after review)
```

### Issue 5: "WebSocket disconnected"
**Solution:**
```bash
# Bot should auto-reconnect
# If not, check internet and restart
```

---

## Fee Impact Analysis

### Polymarket Fee Structure
- **Taker fee:** 0.5% (when you take liquidity)
- **Maker fee:** 0% (when you provide liquidity)
- **Gas fees:** ~$0.01-0.05 per transaction (Polygon)

### Profit Calculation Example

**Scenario: $5 arbitrage trade**
```
Buy:  $5.00 @ 0.750  ($5.00 invested)
Sell: $5.00 @ 0.765  (expect $5.10 back)

Gross profit: $0.10 (2% margin)

Fees:
- Buy taker fee:   $5.00 * 0.005 = $0.025
- Sell taker fee:  $5.10 * 0.005 = $0.025
- Gas (buy):       $0.02
- Gas (sell):      $0.02
Total fees:        $0.09

Net profit:        $0.10 - $0.09 = $0.01
Actual return:     0.2% (not worth it!)
```

**Minimum Profitable Margin:**
- Need at least **2-3% margin** to cover fees
- Current config: PRICE_THRESHOLD=0.01 (1%)
- **Recommendation:** Increase to 0.03 (3%) for live trading

---

## Realistic Expectations

### With $20 Capital

**Best Case (extremely unlikely):**
- 10 successful trades per day
- 3% net profit per trade (after fees)
- Daily profit: $20 * 0.03 * 10 = $6/day
- Monthly: ~$180

**Realistic Case:**
- 1-2 trades per week (most days: no opportunities)
- 1-2% net profit per trade
- Weekly profit: $20 * 0.015 * 2 = $0.60/week
- Monthly: ~$2.40

**Likely Case:**
- No profitable trades (markets too efficient)
- Learning experience only
- Break-even or small loss

### Why $20 is Too Small

1. **Fees dominate:** 1-2% fees on small amounts
2. **Slippage:** Price moves on small orders
3. **Opportunity cost:** Better to learn on paper/dry-run
4. **Psychological:** Stress of watching $20 not worth it

### What $20 IS Good For

✅ Testing the bot with real money
✅ Understanding fee impact
✅ Learning Polymarket market dynamics
✅ Validating WebSocket connection
✅ Practicing risk management

❌ Making meaningful profit
❌ Quitting your day job
❌ Scaling to larger capital

---

## Scaling Strategy (If Successful)

**IF (big if) you're profitable with $20:**

1. **Week 1-2:** $20 capital, learn the system
2. **Week 3-4:** $50 capital (if profitable)
3. **Month 2:** $200 capital (if still profitable)
4. **Month 3:** $500 capital (if consistently profitable)
5. **Month 4+:** Consider if this is worth your time

**Reality check:**
- Most people lose money in first month
- Profitable algo trading is HARD
- Professional firms have advantages (speed, capital, data)
- This bot is a learning project, not a business

---

## Emergency Procedures

### If Things Go Wrong

**Circuit Breaker Trips (Good!)**
1. Bot stops automatically
2. Review logs to understand why
3. Check Polymarket for open positions
4. Adjust risk parameters if needed

**Unexpected Loss**
1. Press Ctrl+C immediately
2. Go to Polymarket and manually close positions
3. Review logs: `grep ERROR`
4. Don't restart until you understand what happened

**Bot Crashes**
1. Check error message
2. Positions are NOT reconciled on restart (manual check needed)
3. File GitHub issue with error
4. Don't restart blindly

**Can't Stop Bot**
1. Ctrl+C doesn't work? Press it multiple times
2. Find process: `ps aux | grep polymarket`
3. Kill it: `kill -9 <PID>`
4. Check for orphaned positions on Polymarket

---

## Legal & Compliance

⚖️ **Important:**
- Polymarket requires KYC verification
- Trading may be restricted in your jurisdiction
- Consult local regulations
- You're responsible for tax reporting
- This is NOT financial advice

---

## Summary: Should You Go Live with $20?

### DO IT IF:
- ✅ You want to learn how algorithmic trading works
- ✅ You're okay losing the $20 (learning cost)
- ✅ You want to test the bot with real money
- ✅ You understand markets are efficient
- ✅ You can monitor the bot actively

### DON'T DO IT IF:
- ❌ You expect to make money
- ❌ You can't afford to lose $20
- ❌ You want passive income
- ❌ You think "HFT = easy money"
- ❌ You can't monitor it actively

---

## Recommended Path

**Better Approach:**

1. **Start:** Dry-run mode (observe for 1 week)
2. **Analyze:** Are there ANY arbitrage opportunities?
3. **If yes:** Proceed to $20 live test
4. **If no:** Research market making or other strategies

**Most Valuable:**
- Understanding market dynamics
- Learning the codebase
- Building better strategies
- NOT making $2/month on $20 capital

---

**Remember: This is a LEARNING project, not a get-rich-quick scheme!**

**Good luck, and trade responsibly!** 🚀

---

**Document Version:** 1.0
**Last Updated:** 2024-12-28
**For Bot Version:** Phase 7b.2
