# TrinityChain Node Setup Guide

## Quick Start: Connect to the Main Network

### 1. **Build the Node**

```bash
cargo build --release --bin trinity-node
```

### 2. **Run Your Node and Connect to Main Network**

The main TrinityChain node is running on Render at:
- **Host:** `trinitychain.onrender.com`
- **P2P Port:** 8333 (default)
- **API:** `https://trinitychain.onrender.com/api/*`
- **Dashboard:** `https://trinitychain.onrender.com/`

```bash
# Start your local node on port 8333
./target/release/trinity-node --port 8333

# In another terminal, connect to the main network
cargo run --bin connect-to-network -- trinitychain.onrender.com:8333
```

**Note:** The Render node may take 30-60 seconds to wake up from sleep (free tier limitation).

---

## Manual Node Connection

### Connect Two Local Nodes

**Terminal 1 - Node A:**
```bash
./target/release/trinity-node --port 8333
```

**Terminal 2 - Node B:**
```bash
./target/release/trinity-node --port 8334

# Connect Node B to Node A
# Use trinity-node --connect flag or API call:
curl -X POST http://localhost:8334/api/network/connect \
  -H "Content-Type: application/json" \
  -d '{"host": "localhost", "port": 8333}'
```

---

## Dashboard Features

### Working Features ✅
- **Auto-refresh Toggle** - Pause/Resume automatic updates (now fixed!)
- **Manual Refresh** - Force refresh blockchain data
- **Block List** - View last 50 blocks with full details
- **Blockchain Stats** - Height, difficulty, UTXO count, mempool size
- **Search** - Find blocks by hash, height, or previous hash
- **Real-time Updates** - Auto-refresh every 3 seconds (configurable)

### API Endpoints Available

#### Blockchain
- `GET /api/blockchain/height` - Current chain height
- `GET /api/blockchain/stats` - Blockchain statistics
- `GET /api/blockchain/blocks?limit=50` - Recent blocks (full details)
- `GET /api/blockchain/block/:hash` - Specific block by hash
- `GET /api/blockchain/block/by-height/:height` - Block by height
- `GET /api/blockchain/reward/:height` - Block reward for height

#### Address & Balance
- `GET /api/address/:addr/balance` - Address balance
- `GET /api/address/:addr/triangles` - Triangles owned by address
- `GET /api/address/:addr/history` - Transaction history

#### Transactions
- `POST /api/transaction` - Submit transaction
- `GET /api/transaction/:hash` - Transaction status
- `GET /api/transactions/pending` - Pending transactions
- `GET /api/transactions/mempool-stats` - Mempool statistics

#### Mining
- `GET /api/mining/status` - Mining status (is_mining, hashrate, blocks_mined)
- `POST /api/mining/start` - Start mining (body: `{"miner_address": "your_address"}`)
- `POST /api/mining/stop` - Stop mining

#### Network
- `GET /api/network/peers` - Connected peers
- `GET /api/network/info` - Network information (peer count, node ID, port)

---

## Node Architecture

### Available Binaries

1. **`trinity-api`** - REST API server (headless, production)
   - Serves blockchain data via HTTP
   - Serves dashboard static files
   - **Recommended for production/Render deployment**

2. **`trinity-node`** - P2P network node
   - Connects to other nodes
   - Syncs blockchain
   - Participates in network consensus

3. **`trinity-miner`** - Mining daemon
   - Continuous mining
   - Auto-submits blocks

4. **`trinity-server`** - Combined API + TUI
   - Local development only
   - Has terminal UI (doesn't work on Render)

---

## Testing Your Connection

### 1. Check if Render Node is Awake
```bash
curl https://trinitychain.onrender.com/api/blockchain/stats
```

**Expected Output:**
```json
{
  "height": 123,
  "difficulty": 2,
  "utxo_count": 456,
  "mempool_size": 0,
  "recent_blocks": [...]
}
```

### 2. Check Your Local Node
```bash
curl http://localhost:8333/api/blockchain/stats
```

### 3. Check Peer Connections
```bash
curl http://localhost:8333/api/network/peers
```

Should show the Render node if connected:
```json
[
  {
    "address": "trinitychain.onrender.com:8333",
    "last_seen": 1700000000
  }
]
```

---

## Syncing Process

When you connect your node to the network:

1. **Connection** - Node establishes TCP connection to peer
2. **Header Sync** - Downloads block headers after your current height
3. **Block Sync** - Downloads full blocks in batches (50 at a time)
4. **Validation** - Verifies proof-of-work and applies transactions
5. **Mempool Sync** - Receives pending transactions (if any)

**Expected Output:**
```
🔗 Connecting to peer: trinitychain.onrender.com:8333
📥 Found 50 new block headers
📥 Received batch of 50 blocks
✅ Applied batch successfully
✅ Already up to date
```

---

## Mining on the Network

### Start Mining to Your Address

**Step 1: Get your address**
```bash
cargo run --bin trinity-wallet
# Copy your address
```

**Step 2: Start mining**
```bash
curl -X POST http://localhost:8333/api/mining/start \
  -H "Content-Type: application/json" \
  -d '{"miner_address": "YOUR_ADDRESS_HERE"}'
```

**Step 3: Check mining status**
```bash
curl http://localhost:8333/api/mining/status
```

**Output:**
```json
{
  "is_mining": true,
  "blocks_mined": 5,
  "hashrate": 1234567.89
}
```

---

## Troubleshooting

### Dashboard Not Loading
**Problem:** "Failed to fetch" error
**Solution:**
1. Check Render node is awake (may take 30-60s on free tier)
2. Check URL is correct: `https://trinitychain.onrender.com`
3. Check browser console for CORS errors

### Node Won't Connect
**Problem:** Connection refused
**Solutions:**
1. Render node may be asleep - wait 60s and retry
2. Check firewall isn't blocking port 8333
3. Verify peer address is correct

### Blocks Not Syncing
**Problem:** Node stays at old height
**Solutions:**
1. Check peer is actually connected: `/api/network/peers`
2. Check peer has higher height: query their `/api/blockchain/stats`
3. Restart sync by reconnecting

### Mining Not Working
**Problem:** "Not mining" status
**Solutions:**
1. Check you sent POST request (not GET)
2. Verify JSON body has correct `miner_address` field
3. Check logs for errors

---

## Network Configuration

### Default Ports
- **P2P Network:** 8333
- **API Server:** 3000 (configurable via `PORT` env var)
- **Render Production:** Uses Render's dynamic port (automatically configured)

### Environment Variables

```bash
# API server port
PORT=3000

# P2P network port
P2P_PORT=8333

# CORS origin (for API)
CORS_ORIGIN="*"

# Log level
RUST_LOG=info
```

---

## Network Status

### Main Network (Production)
- **Status:** ✅ Live on Render
- **URL:** https://trinitychain.onrender.com
- **Uptime:** Subject to free tier limitations (may sleep after inactivity)
- **Genesis:** January 1, 2024 00:00:00 UTC
- **Difficulty:** Dynamic (adjusts every 10 blocks)
- **Block Time Target:** 60 seconds

### Testnet
- **Status:** Not deployed yet
- **Planned:** Q1 2026

---

## Dashboard Fixes Applied

### What Was Broken
- ❌ Play/Pause button showed wrong icon (Play when ON, Pause when OFF)
- ❌ Blocks API returned minimal data (only hash + height)
- ❌ Dashboard couldn't display block details

### What Was Fixed
- ✅ Play/Pause button now correct (Pause when auto-refresh ON)
- ✅ Blocks API returns full data (timestamp, difficulty, nonce, merkleRoot, tx count)
- ✅ Dashboard can display complete block information
- ✅ Real-time auto-refresh with configurable interval

---

## Contributing

Want to run your own seed node? Awesome!

1. Deploy `trinity-api` on a cloud provider (Render, Heroku, AWS, etc.)
2. Open port 8333 for P2P connections
3. Add your node to the seed list (submit PR)
4. Monitor with the dashboard

---

## Support

- **Issues:** https://github.com/TrinityChain/TrinityChain/issues
- **Documentation:** See `AUDIT_FIXES.md` for recent changes
- **Development Plan:** See `documentation/DEVELOPMENT_PLAN.md`

---

**Last Updated:** November 21, 2025
**Network Version:** v0.1.0 (Beta-ready)
