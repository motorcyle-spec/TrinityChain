use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    routing::{get, post},
    Json, Router, http::StatusCode, response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tokio::task::JoinHandle;
use futures_util::{StreamExt, SinkExt};

use crate::blockchain::{Blockchain, Block};
use crate::persistence::Database;
use crate::transaction::Transaction;
use crate::crypto::KeyPair;
use crate::miner;
use crate::network::{Node, NetworkMessage};
use secp256k1::ecdsa::Signature;

/// Mining state that tracks the current mining operation
#[derive(Clone)]
struct MiningState {
    is_mining: Arc<AtomicBool>,
    blocks_mined: Arc<AtomicU64>,
    last_block_time: Arc<Mutex<Option<Instant>>>,
    mining_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Default for MiningState {
    fn default() -> Self {
        Self {
            is_mining: Arc::new(AtomicBool::new(false)),
            blocks_mined: Arc::new(AtomicU64::new(0)),
            last_block_time: Arc::new(Mutex::new(None)),
            mining_task: Arc::new(Mutex::new(None)),
        }
    }
}

/// Network state that tracks peers and node information
#[derive(Clone, Default)]
struct NetworkState {
    peers: Arc<Mutex<Vec<Node>>>,
    node_id: Arc<Mutex<String>>,
    listening_port: Arc<Mutex<u16>>,
}

#[derive(Clone)]
struct AppState {
    blockchain: Arc<Mutex<Blockchain>>,
    db: Arc<Mutex<Database>>,
    mining: MiningState,
    network: NetworkState,
}

pub async fn run_api_server() {
    let db = match Database::open("trinitychain.db") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}. Ensure trinitychain.db is accessible.", e);
            std::process::exit(1);
        }
    };
    let blockchain = match db.load_blockchain() {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Failed to load blockchain from database: {}. Database may be corrupted.", e);
            std::process::exit(1);
        }
    };

    let app_state = AppState {
        blockchain: Arc::new(Mutex::new(blockchain)),
        db: Arc::new(Mutex::new(db)),
        mining: MiningState::default(),
        network: NetworkState::default(),
    };

    // Initialize network state with default values
    {
        let mut node_id = match app_state.network.node_id.lock() {
            Ok(lock) => lock,
            Err(e) => {
                eprintln!("FATAL: node_id lock is poisoned: {}", e);
                std::process::exit(1);
            }
        };
        *node_id = format!("trinity-node-{}", rand::random::<u32>());
        let mut port = match app_state.network.listening_port.lock() {
            Ok(lock) => lock,
            Err(e) => {
                eprintln!("FATAL: listening_port lock is poisoned: {}", e);
                std::process::exit(1);
            }
        };
        *port = 8333;
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Blockchain endpoints
        .route("/blockchain/height", get(get_blockchain_height))
        .route("/blockchain/stats", get(get_blockchain_stats))
        .route("/blockchain/blocks", get(get_recent_blocks))
        .route("/blockchain/block/:hash", get(get_block_by_hash))
        .route("/blockchain/block/by-height/:height", get(get_block_by_height))
        .route("/blockchain/reward/:height", get(get_block_reward_info))
        // Address & Balance
        .route("/address/:addr/balance", get(get_address_balance))
        .route("/address/:addr/triangles", get(get_address_triangles))
        .route("/address/:addr/history", get(get_address_history))
        // Transactions
        .route("/transaction", post(submit_transaction))
        .route("/transaction/:hash", get(get_transaction_status))
        .route("/transactions/pending", get(get_pending_transactions))
        .route("/transactions/mempool-stats", get(get_mempool_stats))
        // Wallet
        .route("/wallet/create", post(create_wallet))
        .route("/wallet/send", post(send_transaction))
        .route("/wallet/import", post(import_wallet))
        // Mining
        .route("/mining/status", get(get_mining_status))
        .route("/mining/start", post(start_mining))
        .route("/mining/stop", post(stop_mining))
        // Network
        .route("/network/peers", get(get_peers))
        .route("/network/info", get(get_network_info))
        // WebSocket P2P Bridge
        .route("/ws/p2p", get(ws_p2p_handler))
        .with_state(app_state)
        .layer(cors.clone());

    // Serve static files from dashboard/dist directory (Vite build output)
    let serve_dir = ServeDir::new("dashboard/dist");

    let app = Router::new()
        .route("/", get(serve_landing))
        .route("/dashboard", get(serve_dashboard))
        .nest("/api", api_routes)
        .fallback_service(serve_dir)
        .layer(cors);

    // Use PORT env var (for Render.com) or default to 3000
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    // Bind to 0.0.0.0 to accept external connections (required for Render.com)
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind to address {}: {}. Port may already be in use.", addr, e);
            std::process::exit(1);
        }
    };
    println!("API server listening on http://{}", addr);
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("API server encountered a fatal error: {}", e);
    }
}

// Landing page handler
async fn serve_landing() -> impl IntoResponse {
    match tokio::fs::read_to_string("dashboard/dist/landing.html").await {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Landing page not found").into_response(),
    }
}

// Dashboard app handler
async fn serve_dashboard() -> impl IntoResponse {
    match tokio::fs::read_to_string("dashboard/dist/index.html").await {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Dashboard not found").into_response(),
    }
}

async fn get_blockchain_height(State(state): State<AppState>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    Json(blockchain.blocks.len() as u64).into_response()
}

async fn get_block_by_hash(State(state): State<AppState>, Path(hash): Path<String>) -> Result<Json<Option<Block>>, Response> {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response()),
    };
    let hash_bytes = match hex::decode(hash) {
        Ok(bytes) => bytes,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid hash format").into_response()),
    };
    let mut hash_arr = [0u8; 32];
    if hash_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "Invalid hash length").into_response());
    }
    hash_arr.copy_from_slice(&hash_bytes);
    let block = blockchain.block_index.get(&hash_arr).cloned();
    Ok(Json(block))
}

#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
    pub triangles: Vec<String>,
    pub total_area: f64,
}

#[derive(Serialize, Deserialize)]
pub struct RecentBlock {
    pub height: u64,
    pub hash: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub chain_height: u64,
    pub difficulty: u64,
    pub utxo_count: usize,
    pub mempool_size: usize,
    pub blocks_to_halving: u64,
    pub recent_blocks: Vec<RecentBlock>,
    // Additional fields for dashboard
    pub blocks_mined: u64,
    pub total_earned: u64,
    pub current_reward: u64,
    pub avg_block_time: f64,
    pub uptime: u64,
    pub total_supply: u64,
    pub max_supply: u64,
    pub halving_era: u64,
}

async fn get_blockchain_stats(State(state): State<AppState>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let recent_blocks: Vec<RecentBlock> = blockchain.blocks.iter().rev().take(6).map(|b| RecentBlock {
        height: b.header.height,
        hash: hex::encode(b.hash),
    }).collect();

    let height = blockchain.blocks.len() as u64;
    const HALVING_INTERVAL: u64 = 210_000;
    let blocks_to_halving = HALVING_INTERVAL - (height % HALVING_INTERVAL);

    // Calculate halving era (0 = first era with full reward)
    let halving_era = height / HALVING_INTERVAL;

    // Current block reward
    let current_reward = Blockchain::calculate_block_reward(height);

    // Max supply (geometric series: 50*210000 * (1 + 0.5 + 0.25 + ...) ≈ 21M equivalent)
    // For TrinityChain with 1000 initial reward: 1000 * 210000 * 2 = 420M
    const MAX_SUPPLY: u64 = 420_000_000;

    // Calculate total supply minted so far
    let total_supply: u64 = (0..=halving_era).map(|era| {
        let era_reward = 1000u64 >> era; // 1000, 500, 250, etc.
        let blocks_in_era = if era < halving_era {
            HALVING_INTERVAL
        } else {
            height % HALVING_INTERVAL
        };
        era_reward.saturating_mul(blocks_in_era)
    }).sum();

    // Calculate average block time from recent blocks
    let avg_block_time = if blockchain.blocks.len() > 1 {
        let recent: Vec<_> = blockchain.blocks.iter().rev().take(10).collect();
        if recent.len() > 1 {
            let time_diffs: Vec<f64> = recent.windows(2)
                .map(|w| (w[0].header.timestamp - w[1].header.timestamp).abs() as f64)
                .collect();
            if !time_diffs.is_empty() {
                time_diffs.iter().sum::<f64>() / time_diffs.len() as f64
            } else {
                0.0
            }
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Total earned (sum of all coinbase rewards in chain)
    let total_earned: u64 = blockchain.blocks.iter()
        .filter_map(|b| b.transactions.first())
        .filter_map(|tx| match tx {
            crate::transaction::Transaction::Coinbase(cb) => Some(cb.reward_area),
            _ => None,
        })
        .sum();

    Json(StatsResponse {
        chain_height: height,
        difficulty: blockchain.difficulty,
        utxo_count: blockchain.state.utxo_set.len(),
        mempool_size: blockchain.mempool.len(),
        blocks_to_halving,
        recent_blocks,
        blocks_mined: height,
        total_earned,
        current_reward,
        avg_block_time,
        uptime: 0, // Would need server start time tracking
        total_supply,
        max_supply: MAX_SUPPLY,
        halving_era,
    }).into_response()
}

async fn get_address_balance(State(state): State<AppState>, Path(addr): Path<String>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let mut triangles = Vec::new();
    let mut total_area = 0.0;

    for (hash, triangle) in &blockchain.state.utxo_set {
        if triangle.owner == addr {
            triangles.push(hex::encode(hash));
            total_area += triangle.area();
        }
    }

    Json(BalanceResponse {
        triangles,
        total_area,
    }).into_response()
}

async fn submit_transaction(State(state): State<AppState>, Json(tx): Json<Transaction>) -> impl IntoResponse {
    let mut blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let tx_hash = tx.hash_str();
    match blockchain.mempool.add_transaction(tx) {
        Ok(_) => Json(tx_hash).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, format!("Failed to add transaction: {}", e)).into_response(),
    }
}

async fn get_transaction_status(State(state): State<AppState>, Path(hash): Path<String>) -> Result<Json<Option<Transaction>>, Response> {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response()),
    };
    let hash_bytes = match hex::decode(hash) {
        Ok(bytes) => bytes,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid hash format").into_response()),
    };
    let mut hash_arr = [0u8; 32];
    if hash_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "Invalid hash length").into_response());
    }
    hash_arr.copy_from_slice(&hash_bytes);
    if let Some(tx) = blockchain.mempool.get_transaction(&hash_arr).cloned() {
        return Ok(Json(Some(tx)));
    }

    for block in &blockchain.blocks {
        if let Some(tx) = block.transactions.iter().find(|tx| tx.hash() == hash_arr) {
            return Ok(Json(Some(tx.clone())));
        }
    }

    Ok(Json(None))
}

// New endpoints

async fn get_recent_blocks(State(state): State<AppState>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let blocks: Vec<serde_json::Value> = blockchain.blocks.iter().rev().take(50).map(|b| {
        // Extract reward from coinbase transaction
        let reward = b.transactions.first()
            .and_then(|tx| match tx {
                crate::transaction::Transaction::Coinbase(cb) => Some(cb.reward_area),
                _ => None,
            })
            .unwrap_or(0);

        serde_json::json!({
            "index": b.header.height,
            "height": b.header.height,
            "hash": hex::encode(b.hash),
            "previousHash": hex::encode(b.header.previous_hash),
            "timestamp": b.header.timestamp,
            "difficulty": b.header.difficulty,
            "nonce": b.header.nonce,
            "merkleRoot": hex::encode(b.header.merkle_root),
            "transactions": b.transactions.len(),
            "reward": reward,
        })
    }).collect();
    // Wrap in object for dashboard compatibility
    Json(serde_json::json!({ "blocks": blocks })).into_response()
}

async fn get_block_by_height(State(state): State<AppState>, Path(height): Path<u64>) -> Result<Json<Option<Block>>, Response> {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response()),
    };
    let block = blockchain.blocks.iter().find(|b| b.header.height == height).cloned();
    Ok(Json(block))
}

#[derive(Serialize, Deserialize)]
pub struct TriangleInfo {
    pub hash: String,
    pub area: f64,
    pub vertices: Vec<(f64, f64)>,
}

async fn get_address_triangles(State(state): State<AppState>, Path(addr): Path<String>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let triangles: Vec<TriangleInfo> = blockchain.state.utxo_set.iter()
        .filter(|(_, triangle)| triangle.owner == addr)
        .map(|(hash, triangle)| TriangleInfo {
            hash: hex::encode(hash),
            area: triangle.area(),
            vertices: vec![
                (triangle.a.x, triangle.a.y),
                (triangle.b.x, triangle.b.y),
                (triangle.c.x, triangle.c.y),
            ],
        })
        .collect();
    Json(triangles).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct TransactionHistory {
    pub tx_hash: String,
    pub block_height: u64,
    pub timestamp: i64,
    pub tx_type: String,
}

async fn get_address_history(State(state): State<AppState>, Path(addr): Path<String>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let mut history = Vec::new();

    for block in &blockchain.blocks {
        for tx in &block.transactions {
            let involves_address = match tx {
                Transaction::Subdivision(tx) => tx.owner_address == addr,
                Transaction::Transfer(tx) => tx.sender == addr || tx.new_owner == addr,
                Transaction::Coinbase(tx) => tx.beneficiary_address == addr,
            };

            if involves_address {
                history.push(TransactionHistory {
                    tx_hash: tx.hash_str(),
                    block_height: block.header.height,
                    timestamp: block.header.timestamp,
                    tx_type: match tx {
                        Transaction::Subdivision(_) => "Subdivision".to_string(),
                        Transaction::Transfer(_) => "Transfer".to_string(),
                        Transaction::Coinbase(_) => "Coinbase".to_string(),
                    },
                });
            }
        }
    }

    Json(history).into_response()
}

async fn get_pending_transactions(State(state): State<AppState>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    Json(blockchain.mempool.get_all_transactions()).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct WalletResponse {
    pub address: String,
    pub public_key: String,
    // SECURITY: Private key removed from API response to prevent exposure
    // Private keys should only be shown once in terminal or downloaded as encrypted wallet file
}


#[derive(Serialize, Deserialize)]
pub struct StartMiningRequest {
    pub miner_address: String,
}



async fn create_wallet() -> Result<Json<WalletResponse>, Response> {
    match KeyPair::generate() {
        Ok(keypair) => {
            let address = keypair.address();
            let public_key = hex::encode(keypair.public_key.serialize());
            // SECURITY: Private key NOT included in response for security
            // Users should use CLI wallet tools to generate and securely store keys

            Ok(Json(WalletResponse {
                address,
                public_key,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to generate keypair: {}", e)).into_response()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct ImportWalletRequest {
    pub private_key: String,
}

async fn import_wallet(Json(req): Json<ImportWalletRequest>) -> Result<Json<WalletResponse>, Response> {
    let private_key_bytes = match hex::decode(&req.private_key) {
        Ok(bytes) => bytes,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid private key format").into_response()),
    };

    match KeyPair::from_secret_bytes(&private_key_bytes) {
        Ok(keypair) => {
            let address = keypair.address();
            let public_key = hex::encode(keypair.public_key.serialize());
            // SECURITY: Private key NOT echoed back in response

            Ok(Json(WalletResponse {
                address,
                public_key,
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, format!("Invalid private key: {}", e)).into_response()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct SendTransactionRequest {
    pub transaction: Transaction,
    pub signature: String,
}

async fn send_transaction(State(state): State<AppState>, Json(req): Json<SendTransactionRequest>) -> impl IntoResponse {
    let mut blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };

    // Verify the signature
    let signature_bytes = match hex::decode(&req.signature) {
        Ok(bytes) => bytes,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid signature format").into_response(),
    };
    let signature = match Signature::from_der(&signature_bytes) {
        Ok(sig) => sig,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid signature").into_response(),
    };

    let tx_hash = req.transaction.hash();
    let message = secp256k1::Message::from_digest_slice(&tx_hash).unwrap();
    let public_key = match &req.transaction {
        Transaction::Transfer(tx) => {
            let key_bytes = hex::decode(&tx.sender).unwrap();
            secp256k1::PublicKey::from_slice(&key_bytes).unwrap()
        }
        _ => return (StatusCode::BAD_REQUEST, "Only transfer transactions are supported").into_response(),
    };

    let secp = secp256k1::Secp256k1::new();
    if !secp.verify_ecdsa(&message, &signature, &public_key).is_ok() {
        return (StatusCode::BAD_REQUEST, "Invalid signature").into_response();
    }

    let tx_hash_str = req.transaction.hash_str();
    match blockchain.mempool.add_transaction(req.transaction) {
        Ok(_) => Json(tx_hash_str).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, format!("Failed to add transaction: {}", e)).into_response(),
    }
}

#[derive(Serialize, Deserialize)]
pub struct MiningStatus {
    pub is_mining: bool,
    pub blocks_mined: u64,
    pub hashrate: f64,
}

async fn get_mining_status(State(state): State<AppState>) -> impl IntoResponse {
    let is_mining = state.mining.is_mining.load(Ordering::Relaxed);
    let blocks_mined = state.mining.blocks_mined.load(Ordering::Relaxed);

    // Calculate approximate hashrate based on last block time
    let hashrate = if is_mining {
        let last_time = match state.mining.last_block_time.lock() {
            Ok(lock) => lock,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get mining state lock").into_response(),
        };
        if let Some(instant) = *last_time {
            let elapsed = instant.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                // Estimate based on difficulty and time
                let blockchain = match state.blockchain.lock() {
                    Ok(lock) => lock,
                    Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
                };
                let difficulty = blockchain.difficulty;
                // Calculate expected hashes safely to prevent overflow
                // For each leading zero, we expect 16x more hashes on average
                // Cap at difficulty 40 to prevent f64 overflow (16^40 < f64::MAX)
                let safe_difficulty = difficulty.min(40);
                let expected_hashes = 16_f64.powi(safe_difficulty as i32);

                // Return hashrate in hashes/second
                expected_hashes / elapsed.max(0.001) // Prevent division by very small numbers
            } else {
                0.0
            }
        } else {
            0.0
        }
    } else {
        0.0
    };

    Json(MiningStatus {
        is_mining,
        blocks_mined,
        hashrate,
    }).into_response()
}

async fn start_mining(State(state): State<AppState>, Json(req): Json<StartMiningRequest>) -> impl IntoResponse {
    // Check if already mining
    if state.mining.is_mining.load(Ordering::Relaxed) {
        return (StatusCode::BAD_REQUEST, "Mining already in progress").into_response();
    }
    let miner_address = req.miner_address;
    state.mining.is_mining.store(true, Ordering::Relaxed);

    // Spawn mining task
    let blockchain_clone = state.blockchain.clone();
    let db_clone = state.db.clone();
    let mining_state = state.mining.clone();

    let task = tokio::spawn(async move {
        loop {
            // Check if we should stop
            if !mining_state.is_mining.load(Ordering::Relaxed) {
                break;
            }

            // Get pending transactions
            let block = {
                let blockchain = match blockchain_clone.lock() {
                    Ok(lock) => lock,
                    Err(e) => {
                        eprintln!("Failed to acquire blockchain lock in mining task: {}", e);
                        mining_state.is_mining.store(false, Ordering::Relaxed); // Stop mining
                        break;
                    }
                };
                let transactions = blockchain.mempool.get_all_transactions();

                let height = blockchain.blocks.len() as u64;
                let last_block = blockchain.blocks.last().expect("Blockchain should have at least a genesis block");
                let previous_hash = last_block.hash;
                let parent_timestamp = last_block.header.timestamp;
                let difficulty = blockchain.difficulty;

                // Calculate proper block reward with halving
                // Block reward is static u64, fees are geometric f64
                let block_reward = Blockchain::calculate_block_reward(height);
                let total_fees = Blockchain::calculate_total_fees(&transactions);
                let reward_area = block_reward.saturating_add(total_fees as u64);

                // Create coinbase transaction
                let coinbase = Transaction::Coinbase(crate::transaction::CoinbaseTx {
                    reward_area,
                    beneficiary_address: miner_address.clone(),
                });

                let mut all_txs = vec![coinbase];
                all_txs.extend(transactions);

                Block::new_with_parent_time(height, previous_hash, parent_timestamp, difficulty, all_txs)
            };

            // Mine the block (this is CPU intensive - run on blocking thread pool)
            let start = Instant::now();
            let mine_result = tokio::task::spawn_blocking(move || {
                miner::mine_block(block)
            }).await;

            // Handle spawn_blocking result
            let mine_result = match mine_result {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Mining task panicked: {}", e);
                    mining_state.is_mining.store(false, Ordering::Relaxed);
                    break;
                }
            };

            match mine_result {
                Ok(mined_block) => {
                    // Update last block time
                    {
                        let mut last_time = match mining_state.last_block_time.lock() {
                            Ok(lock) => lock,
                            Err(e) => {
                                eprintln!("Failed to acquire mining state lock for last_block_time: {}", e);
                                mining_state.is_mining.store(false, Ordering::Relaxed); // Stop mining
                                break;
                            }
                        };
                        *last_time = Some(start);
                    }

                    // Add block to blockchain
                    {
                        let mut blockchain = match blockchain_clone.lock() {
                            Ok(lock) => lock,
                            Err(e) => {
                                eprintln!("Failed to acquire blockchain lock for applying block: {}", e);
                                mining_state.is_mining.store(false, Ordering::Relaxed); // Stop mining
                                break;
                            }
                        };
                        if let Err(e) = blockchain.apply_block(mined_block.clone()) {
                            eprintln!("Failed to apply mined block: {}", e);
                            continue;
                        }

                        // Save to database
                        let db = match db_clone.lock() {
                            Ok(lock) => lock,
                            Err(e) => {
                                eprintln!("Failed to acquire database lock for saving block: {}", e);
                                mining_state.is_mining.store(false, Ordering::Relaxed); // Stop mining
                                break;
                            }
                        };
                        if let Err(e) = db.save_block(&mined_block) {
                            eprintln!("Failed to save block: {}", e);
                        }
                        if let Err(e) = db.save_utxo_set(&blockchain.state) {
                            eprintln!("Failed to save UTXO set: {}", e);
                        }
                    }

                    // Increment blocks mined counter
                    mining_state.blocks_mined.fetch_add(1, Ordering::Relaxed);

                    println!("✅ Mined block at height {}", mined_block.header.height);
                }
                Err(e) => {
                    eprintln!("Mining error: {}", e);
                    mining_state.is_mining.store(false, Ordering::Relaxed); // Stop mining on mining error
                    break;
                }
            }
        }

        println!("Mining stopped");
    });

    // Store the task handle
    {
        let mut task_handle = match state.mining.mining_task.lock() {
            Ok(lock) => lock,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to acquire mining task lock: {}", e)).into_response(),
        };
        *task_handle = Some(task);
    }

    Json("Mining started successfully".to_string()).into_response()
}

async fn stop_mining(State(state): State<AppState>) -> impl IntoResponse {
    // Check if mining is active
    if !state.mining.is_mining.load(Ordering::Relaxed) {
        return (StatusCode::BAD_REQUEST, "Mining is not active").into_response();
    }

    // Signal the mining task to stop
    state.mining.is_mining.store(false, Ordering::Relaxed);

    // Wait for the task to complete (with timeout)
    let task_handle = match state.mining.mining_task.lock() {
        Ok(mut lock) => lock.take(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to acquire mining task lock: {}", e)).into_response(),
    };
    if let Some(handle) = task_handle {
        // Wait up to 5 seconds for the task to finish
        match tokio::time::timeout(Duration::from_secs(5), handle).await {
            Ok(_) => {},
            Err(_) => {
                eprintln!("Warning: Mining task didn't stop within timeout");
            }
        }
    }

    Json("Mining stopped successfully".to_string()).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct PeerInfo {
    pub address: String,
    pub last_seen: i64,
}

async fn get_peers(State(state): State<AppState>) -> impl IntoResponse {
    let peers = match state.network.peers.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get network peers lock").into_response(),
    };
    let peer_info: Vec<PeerInfo> = peers.iter().map(|peer| PeerInfo {
        address: peer.addr(),
        last_seen: chrono::Utc::now().timestamp(), // In a real implementation, track actual last seen time
    }).collect();
    Json(peer_info).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct NetworkInfo {
    pub peers_count: usize,
    pub node_id: String,
    pub listening_port: u16,
}

async fn get_network_info(State(state): State<AppState>) -> impl IntoResponse {
    let peers = match state.network.peers.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get network peers lock").into_response(),
    };
    let node_id = match state.network.node_id.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get node ID lock").into_response(),
    };
    let listening_port = match state.network.listening_port.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get listening port lock").into_response(),
    };

    Json(NetworkInfo {
        peers_count: peers.len(),
        node_id: node_id.clone(),
        listening_port: *listening_port,
    }).into_response()
}

// New endpoints for enhanced block explorer functionality

#[derive(Serialize)]
struct MempoolStatsResponse {
    transaction_count: usize,
    total_fees: u64,
    avg_fee: f64,
    highest_fee: u64,
    lowest_fee: u64,
}

async fn get_mempool_stats(State(state): State<AppState>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let txs = blockchain.mempool.get_all_transactions();

    let fees: Vec<u64> = txs.iter().map(|tx| tx.fee()).collect();
    let total_fees: u64 = fees.iter().sum();
    let avg_fee = if !fees.is_empty() {
        total_fees as f64 / fees.len() as f64
    } else {
        0.0
    };
    let highest_fee = fees.iter().max().copied().unwrap_or(0);
    let lowest_fee = fees.iter().min().copied().unwrap_or(0);

    Json(MempoolStatsResponse {
        transaction_count: txs.len(),
        total_fees,
        avg_fee,
        highest_fee,
        lowest_fee,
    }).into_response()
}

#[derive(Serialize)]
struct RewardInfoResponse {
    current_height: u64,
    current_reward: u64,
    next_halving_height: u64,
    blocks_until_halving: u64,
    reward_after_halving: u64,
}

async fn get_block_reward_info(State(state): State<AppState>, Path(height): Path<u64>) -> impl IntoResponse {
    let blockchain = match state.blockchain.lock() {
        Ok(lock) => lock,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get blockchain lock").into_response(),
    };
    let current_height = blockchain.blocks.len() as u64;
    let query_height = if height == 0 { current_height } else { height };

    let current_reward = Blockchain::calculate_block_reward(query_height);
    let halving_interval = 210_000u64;
    let next_halving_height = ((query_height / halving_interval) + 1) * halving_interval;
    let blocks_until_halving = next_halving_height.saturating_sub(query_height);
    let reward_after_halving = Blockchain::calculate_block_reward(next_halving_height);

    Json(RewardInfoResponse {
        current_height: query_height,
        current_reward,
        next_halving_height,
        blocks_until_halving,
        reward_after_halving,
    }).into_response()
}

/// WebSocket P2P Bridge Handler
/// Allows nodes to communicate over WebSocket instead of raw TCP
async fn ws_p2p_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_p2p(socket, state))
}

async fn handle_ws_p2p(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    println!("🌐 WebSocket P2P connection established");

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                // Deserialize the NetworkMessage from bincode
                match bincode::deserialize::<NetworkMessage>(&data) {
                    Ok(message) => {
                        let response = handle_network_message(message, &state).await;
                        if let Some(resp_data) = response {
                            if let Err(e) = sender.send(Message::Binary(resp_data)).await {
                                eprintln!("❌ WebSocket send error: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to deserialize message: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                println!("🔌 WebSocket P2P connection closed");
                break;
            }
            Ok(_) => {} // Ignore other message types
            Err(e) => {
                eprintln!("❌ WebSocket error: {}", e);
                break;
            }
        }
    }
}

async fn handle_network_message(message: NetworkMessage, state: &AppState) -> Option<Vec<u8>> {
    match message {
        NetworkMessage::GetBlockHeaders { after_height } => {
            let blockchain = state.blockchain.lock().ok()?;
            let headers: Vec<_> = blockchain.blocks
                .iter()
                .filter(|b| b.header.height > after_height)
                .map(|b| b.header.clone())
                .collect();
            let response = NetworkMessage::BlockHeaders(headers);
            bincode::serialize(&response).ok()
        }
        NetworkMessage::GetBlocks(hashes) => {
            let blockchain = state.blockchain.lock().ok()?;
            let blocks: Vec<_> = hashes.iter()
                .filter_map(|h| blockchain.block_index.get(h).cloned())
                .collect();
            let response = NetworkMessage::Blocks(blocks);
            bincode::serialize(&response).ok()
        }
        NetworkMessage::GetPeers => {
            let peers = state.network.peers.lock().ok()?;
            let response = NetworkMessage::Peers(peers.clone());
            bincode::serialize(&response).ok()
        }
        NetworkMessage::NewBlock(block) => {
            let mut blockchain = state.blockchain.lock().ok()?;
            if let Err(e) = blockchain.apply_block(*block) {
                eprintln!("❌ Failed to add block: {}", e);
            }
            None
        }
        NetworkMessage::NewTransaction(tx) => {
            let mut blockchain = state.blockchain.lock().ok()?;
            if let Err(e) = blockchain.mempool.add_transaction(*tx) {
                eprintln!("❌ Failed to add transaction: {}", e);
            }
            None
        }
        NetworkMessage::Ping => {
            let response = NetworkMessage::Pong;
            bincode::serialize(&response).ok()
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    fn test_app() -> Router {
        let blockchain = Blockchain::new();
        let db = match Database::open(":memory:") {
            Ok(db) => db,
            Err(e) => panic!("Failed to open in-memory database for test: {}", e),
        };

        let app_state = AppState {
            blockchain: Arc::new(Mutex::new(blockchain)),
            db: Arc::new(Mutex::new(db)),
            mining: MiningState::default(),
            network: NetworkState::default(),
        };

        Router::new()
            .route("/blockchain/height", get(get_blockchain_height))
            .route("/blockchain/stats", get(get_blockchain_stats)) // Added missing route
            .route("/blockchain/block/:hash", get(get_block_by_hash))
            .route("/address/:addr/balance", get(get_address_balance))
            .route("/transaction", post(submit_transaction))
            .route("/transaction/:hash", get(get_transaction_status))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_get_blockchain_height() {
        let server = TestServer::new(test_app()).expect("Test server setup failed");
        let response = server.get("/blockchain/height").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.json::<u64>(), 1);
    }

    #[tokio::test]
    async fn test_get_block_by_hash() {
        let server = TestServer::new(test_app()).expect("Test server setup failed");
        
        // First, get stats to find the genesis block
        let stats_response = server.get("/blockchain/stats").await;
        assert_eq!(stats_response.status_code(), StatusCode::OK);
        let stats: StatsResponse = stats_response.json();
        
        // The genesis block should be in recent_blocks (last one, since it's height 0)
        let genesis_block_info = stats.recent_blocks.last().expect("Should have genesis block");
        let genesis_hash = &genesis_block_info.hash;
        
        // Get the genesis block using its actual hash
        let response = server.get(&format!("/blockchain/block/{}", genesis_hash)).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let block: Option<Block> = response.json();
        assert!(block.is_some());
        let block = block.expect("Block should be present");
        assert_eq!(block.header.height, 0);
    }

    use crate::transaction::SubdivisionTx;
    use crate::crypto::KeyPair;

    #[tokio::test]
    async fn test_get_address_balance() {
        let server = TestServer::new(test_app()).expect("Test server setup failed");
        let genesis_owner = "genesis_owner";
        let response = server.get(&format!("/address/{}/balance", genesis_owner)).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let balance: BalanceResponse = response.json();
        assert_eq!(balance.triangles.len(), 1);
        assert!(balance.total_area > 0.0);
    }

    #[tokio::test]
    async fn test_submit_and_get_transaction() {
        let server = TestServer::new(test_app()).expect("Test server setup failed");
        let blockchain = Blockchain::new();
        let _genesis = blockchain.blocks[0].clone();
        let keypair = KeyPair::generate().expect("Keypair generation should succeed in test");
        let address = keypair.address();
        let parent_hash = *blockchain.state.utxo_set.keys().next().expect("UTXO set should not be empty in test");
        let children = blockchain.state.utxo_set.values().next().expect("UTXO set should not be empty in test").subdivide();
        let mut tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        let message = tx.signable_message();
        let signature = keypair.sign(&message).expect("Signing message should succeed in test");
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);
        let transaction = Transaction::Subdivision(tx);

        let response = server.post("/transaction").json(&transaction).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let tx_hash: String = response.json();
        assert!(!tx_hash.is_empty());

        let response = server.get(&format!("/transaction/{}", tx_hash)).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let tx_status: Option<Transaction> = response.json();
        assert!(tx_status.is_some());
    }
}
