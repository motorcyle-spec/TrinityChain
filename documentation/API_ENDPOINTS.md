# TrinityChain API Endpoints

Base URL: `https://trinitychain.onrender.com` (or `http://localhost:3000` for local dev)

## Blockchain Endpoints

### GET `/api/blockchain/stats`
Get blockchain statistics including chain height, total supply, halving info, etc.

**Response:**
```json
{
  "chainHeight": 123,
  "blocksMined": 123,
  "totalEarned": 123000,
  "totalSupply": 123000,
  "maxSupply": 420000000,
  "currentReward": 1000,
  "halvingEra": 0,
  "blocksToHalving": 209877,
  "difficulty": 4,
  "avgBlockTime": 5.2,
  "uptime": 3600
}
```

### GET `/api/blockchain/blocks?limit=50`
Get recent blocks from the blockchain.

**Query Parameters:**
- `limit` (optional, default: 50) - Number of blocks to return

**Response:**
```json
{
  "blocks": [
    {
      "index": 123,
      "timestamp": "2025-01-21T...",
      "previousHash": "0000...",
      "hash": "0000...",
      "transactions": [],
      "nonce": 12345,
      "difficulty": 4,
      "reward": 1000
    }
  ]
}
```

### GET `/api/blockchain/height`
Get current blockchain height.

**Response:**
```json
{
  "height": 123
}
```

### GET `/api/blockchain/block/:hash`
Get block by hash.

### GET `/api/blockchain/block/by-height/:height`
Get block by height.

### GET `/api/blockchain/reward/:height`
Get block reward info for a specific height.

## Address & Balance Endpoints

### GET `/api/address/:addr/balance`
Get balance for an address.

**Response:**
```json
{
  "address": "trinity...",
  "balance": 5000
}
```

### GET `/api/address/:addr/triangles`
Get triangle (balance) count for an address.

### GET `/api/address/:addr/history`
Get transaction history for an address.

## Transaction Endpoints

### POST `/api/transaction`
Submit a new transaction.

**Request Body:**
```json
{
  "from": "trinity...",
  "to": "trinity...",
  "amount": 100,
  "signature": "..."
}
```

### GET `/api/transaction/:hash`
Get transaction status by hash.

### GET `/api/transactions/pending`
Get pending transactions in mempool.

### GET `/api/transactions/mempool-stats`
Get mempool statistics.

## Wallet Endpoints

### POST `/api/wallet/create`
Create a new wallet.

**Response:**
```json
{
  "address": "trinity...",
  "privateKey": "...",
  "publicKey": "..."
}
```

### POST `/api/wallet/send`
Send a transaction.

**Request Body:**
```json
{
  "from": "trinity...",
  "to": "trinity...",
  "amount": 100,
  "privateKey": "..."
}
```

### POST `/api/wallet/import`
Import an existing wallet.

## Mining Endpoints

### GET `/api/mining/status`
Get current mining status.

**Response:**
```json
{
  "mining": false,
  "hashrate": 0,
  "blocksMined": 0
}
```

### POST `/api/mining/start`
Start mining.

**Request Body:**
```json
{
  "threads": 4
}
```

### POST `/api/mining/stop`
Stop mining.

## Network Endpoints

### GET `/api/network/peers`
Get connected peers.

**Response:**
```json
{
  "peers": []
}
```

### GET `/api/network/info`
Get network information.

**Response:**
```json
{
  "nodeId": "trinity-node-123",
  "listeningPort": 8333,
  "peerCount": 0,
  "version": "1.0.0"
}
```

## Dashboard

### GET `/`
Serves the React dashboard (TrinityChain Mining Dashboard v2.0).

The dashboard auto-refreshes every 3 seconds and displays:
- Chain statistics
- Recent blocks
- Analytics charts
- Block explorer
- Network performance

## Testing the API

### Using curl:
```bash
# Get blockchain stats
curl https://trinitychain.onrender.com/api/blockchain/stats

# Get recent blocks
curl https://trinitychain.onrender.com/api/blockchain/blocks?limit=10

# Get balance for an address
curl https://trinitychain.onrender.com/api/address/YOUR_ADDRESS/balance
```

### Using the Dashboard:
Navigate to `https://trinitychain.onrender.com` to see the full dashboard UI with real-time updates.

## Health Check

Render uses `/api/blockchain/stats` as the health check endpoint to ensure the service is running properly.
