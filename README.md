<p align="center">
  <img src="assets/logo.png" alt="TrinityChain Logo" width="200"/>
</p>

<h1 align="center">TrinityChain</h1>

<p align="center">
  <strong>The Geometric Blockchain</strong><br>
  <em>Where Area = Value</em>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#api">API</a> •
  <a href="#contributing">Contributing</a>
</p>

---

## Overview

TrinityChain is a revolutionary proof-of-work blockchain where **value is represented as geometric triangles**. Instead of abstract numbers, every UTXO is a triangle with real coordinates—its spendable value equals its geometric area.

```
    △ Area = Value
   /  \
  /    \  Subdivide into 3 child triangles (Sierpiński)
 /______\ Transfer ownership while preserving geometry
```

**Status:** Functional with tests, CLI tools, and web dashboard. Built by one developer with AI assistance—we welcome contributors!

---

## Features

### Core Innovation: Triangle-Based UTXO

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| **Coinbase** | ∅ | 1 △ | Mining reward creates a new triangle |
| **Transfer** | 1 △ | 1 △ | Change ownership, pay geometric fees |
| **Subdivision** | 1 △ | 3 △ | Split into Sierpiński children |

### Geometric Fee Model

Transaction fees are paid by **reducing the triangle's geometric area**:

```rust
// Fee deduction preserves triangle identity
input_area: 100.0 → fee: 0.1 → output_area: 99.9
```

The triangle's shape and coordinates remain unchanged—only its spendable value decreases.

### Technical Features

- **Proof-of-Work**: SHA-256 mining with dynamic difficulty adjustment
- **Cryptography**: secp256k1 signatures, HD wallets with BIP-39 mnemonics
- **Persistence**: SQLite-backed blockchain storage
- **Networking**: TCP P2P with WebSocket bridge support
- **API**: Full REST API + WebSocket for real-time updates
- **Dashboard**: React-based web interface with live stats

---

## Quick Start

### Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- SQLite3

### Build & Test

```bash
git clone https://github.com/TrinityChain/TrinityChain.git
cd TrinityChain

# Build optimized release
cargo build --release

# Run all tests
cargo test --lib
```

### Create a Wallet

```bash
# Generate new HD wallet with mnemonic
./target/release/trinity-wallet-new

# Or restore from mnemonic
./target/release/trinity-wallet-restore
```

### Run a Node

```bash
# Start node with API server on port 3000
./target/release/trinity-node

# Or specify custom port
./target/release/trinity-node 8080
```

### Mine Blocks

```bash
# Mine a single block
./target/release/trinity-mine-block <your_address>

# Parallel mining with multiple threads
./target/release/trinity-mine-block --threads 4 <your_address>
```

### Send Triangles

```bash
# Transfer a triangle to another address
./target/release/trinity-send <to_address> <triangle_hash> [memo]
```

---

## Architecture

<p align="center">
  <img src="assets/architecture.svg" alt="Architecture Diagram" width="600"/>
</p>

### Module Structure

```
src/
├── geometry.rs      # Triangle primitives, area calculation (Shoelace formula)
├── transaction.rs   # Coinbase, Transfer, Subdivision transactions
├── blockchain.rs    # Chain state, UTXO set, mempool, validation
├── network.rs       # P2P networking, peer discovery, message handling
├── miner.rs         # PoW mining, difficulty adjustment
├── persistence.rs   # SQLite database layer
├── api.rs           # REST API + WebSocket endpoints
├── crypto.rs        # secp256k1 keys, signatures
├── wallet.rs        # Wallet management
└── hdwallet.rs      # BIP-39/BIP-32 HD wallet derivation
```

### Data Flow

```
Wallet → Transaction → Mempool → Miner → Block → Blockchain → Persistence
                         ↑                           ↓
                    P2P Network ←──────────────── Broadcast
```

### Precision & Safety

- **Floating-point**: IEEE 754 `f64` with `GEOMETRIC_TOLERANCE = 1e-9`
- **Concurrency**: `Arc<RwLock<T>>` for P2P, `Arc<Mutex<T>>` for API
- **Atomic mining**: `AtomicBool` + `AtomicU64` with `SeqCst` ordering

---

## API

The node exposes a REST API on port 3000 (configurable via `PORT` env var).

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/blockchain/stats` | GET | Chain height, difficulty, supply stats |
| `/api/blockchain/blocks` | GET | Recent blocks with rewards |
| `/api/blockchain/block/:hash` | GET | Block by hash |
| `/api/address/:addr/balance` | GET | Address balance and triangles |
| `/api/address/:addr/triangles` | GET | Triangle details with vertices |
| `/api/transaction` | POST | Submit transaction |
| `/api/mining/start` | POST | Start mining |
| `/api/mining/stop` | POST | Stop mining |
| `/api/mining/status` | GET | Mining status and hashrate |
| `/api/network/peers` | GET | Connected peers |
| `/ws/p2p` | WS | WebSocket P2P bridge |

### Example

```bash
# Get blockchain stats
curl http://localhost:3000/api/blockchain/stats

# Get address balance
curl http://localhost:3000/api/address/YOUR_ADDRESS/balance
```

---

## Dashboard

A React-based web dashboard is included for monitoring and interaction.

```bash
cd dashboard
npm install
npm run build
```

Access at `http://localhost:3000/dashboard` when the node is running.

Features:
- Live blockchain stats (height, difficulty, supply)
- Block explorer with transaction details
- Mining controls (start/stop)
- Network performance charts
- Halving countdown

---

## Tokenomics

| Parameter | Value |
|-----------|-------|
| Initial Block Reward | 1,000 TRC |
| Halving Interval | 210,000 blocks |
| Max Supply | 420,000,000 TRC |
| Block Time Target | ~10 seconds |

Supply follows a geometric series with halvings, similar to Bitcoin's emission schedule.

---

## Documentation

Detailed architecture documents are available:

- [`ARCHITECTURE_MOC.md`](ARCHITECTURE_MOC.md) - Visual component map with ASCII diagrams
- [`ARCHITECTURE_AUDIT.md`](ARCHITECTURE_AUDIT.md) - Data flow and component analysis
- [`SAFETY_AUDIT.md`](SAFETY_AUDIT.md) - Mutability, concurrency, error handling
- [`TRIANGLE_UTXO_AUDIT.md`](TRIANGLE_UTXO_AUDIT.md) - Triangle UTXO model deep dive
- [`API_ENDPOINTS.md`](API_ENDPOINTS.md) - Full API reference
- [`NODE_SETUP.md`](NODE_SETUP.md) - Production node deployment guide

---

## Contributing

We actively welcome contributions! This project needs:

- **Developers**: Rust, React, networking
- **Reviewers**: Security audits, code review
- **Testers**: Run nodes, stress testing
- **Writers**: Documentation, tutorials

### How to Contribute

1. Fork & clone the repository
2. Create a feature branch: `git checkout -b feature/your-name`
3. Write tests for your changes
4. Run `cargo test --lib` and ensure all pass
5. Submit a PR with a clear description

Look for `good first issue` labels or open an issue describing what you'd like to work on.

### Code Style

- Follow Rust conventions (`cargo fmt`, `cargo clippy`)
- Add tests for new functionality
- Document public APIs

---

## Links

- **Repository**: https://github.com/TrinityChain/TrinityChain
- **Issues**: https://github.com/TrinityChain/TrinityChain/issues
- **Dashboard**: See `dashboard/README.md`

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Built with Rust</strong><br>
  <em>Thank you for exploring TrinityChain!</em>
</p>
