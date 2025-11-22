//! P2P Networking for TrinityChain

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::blockchain::Blockchain;
use crate::error::ChainError;
use crate::sync::NodeSynchronizer;

/// Maximum message size to prevent DoS attacks (10MB)
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub host: String,
    pub port: u16,
}

impl Node {
    pub fn new(host: String, port: u16) -> Self {
        Node { host, port }
    }
    
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub struct NetworkNode {
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<RwLock<Vec<Node>>>,
    synchronizer: Arc<NodeSynchronizer>,
}

impl NetworkNode {
    pub fn new(blockchain: Blockchain, _db_path: String) -> Self {
        NetworkNode {
            blockchain: Arc::new(RwLock::new(blockchain)),
            peers: Arc::new(RwLock::new(Vec::new())),
            synchronizer: Arc::new(NodeSynchronizer::new()),
        }
    }
    
    /// Get a reference to the synchronizer
    pub fn synchronizer(&self) -> &Arc<NodeSynchronizer> {
        &self.synchronizer
    }
    
    pub async fn start_server(&self, port: u16) -> Result<(), ChainError> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| ChainError::NetworkError(format!("Failed to bind: {}", e)))?;
        
        println!("🌐 Node listening on {}", addr);
        
        loop {
            match listener.accept().await {
                Ok((socket, peer_addr)) => {
                    println!("📡 New connection from {}", peer_addr);
                    let blockchain = self.blockchain.clone();
                    let peers = self.peers.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(socket, blockchain, peers).await {
                            eprintln!("❌ Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Accept error: {}", e);
                }
            }
        }
    }
    
    pub async fn connect_peer(&self, host: String, port: u16) -> Result<(), ChainError> {
        let addr = format!("{}:{}", host, port);
        println!("🔗 Connecting to peer: {}", addr);

        let node = Node::new(host.clone(), port);
        
        let mut stream = TcpStream::connect(&addr).await
            .map_err(|e| ChainError::NetworkError(format!("Failed to connect: {}", e)))?;

        // 1. Get remote headers
        let local_height = self.get_height().await;
        let request = NetworkMessage::GetBlockHeaders { after_height: local_height };
        let data = bincode::serialize(&request)
            .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;

        let len = data.len() as u32;
        stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
        stream.write_all(&data).await
            .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;

        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await
            .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        // Prevent DoS: reject messages larger than MAX_MESSAGE_SIZE
        if len > MAX_MESSAGE_SIZE {
            return Err(ChainError::NetworkError(format!("Message too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE)));
        }

        let mut buffer = vec![0u8; len];
        stream.read_exact(&mut buffer).await
            .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;

        let response: NetworkMessage = bincode::deserialize(&buffer)
            .map_err(|e| ChainError::NetworkError(format!("Deserialization failed: {}", e)))?;

        let remote_headers = match response {
            NetworkMessage::BlockHeaders(headers) => headers,
            _ => return Err(ChainError::NetworkError("Unexpected response".to_string())),
        };

        // Register peer with synchronizer
        let remote_height = remote_headers.last().map(|h| h.height).unwrap_or(local_height);
        if let Err(e) = self.synchronizer.register_peer(node.clone(), remote_height).await {
            eprintln!("⚠️  Warning: Failed to register peer in synchronizer: {}", e);
        }

        if remote_headers.is_empty() {
            println!("✅ Already up to date");
            return Ok(());
        }

        println!("📥 Found {} new block headers", remote_headers.len());

        // 2. Request missing blocks in batches (50 blocks at a time for efficiency)
        const BATCH_SIZE: usize = 50;
        let block_hashes: Vec<_> = remote_headers.iter()
            .map(|h| h.calculate_hash())
            .collect();

        for chunk in block_hashes.chunks(BATCH_SIZE) {
            let mut stream = TcpStream::connect(&addr).await
                .map_err(|e| ChainError::NetworkError(format!("Failed to connect: {}", e)))?;

            let request = NetworkMessage::GetBlocks(chunk.to_vec());
            let data = bincode::serialize(&request)
                .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;

            let len = data.len() as u32;
            stream.write_all(&len.to_be_bytes()).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            stream.write_all(&data).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;

            let mut len_bytes = [0u8; 4];
            stream.read_exact(&mut len_bytes).await
                .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
            let len = u32::from_be_bytes(len_bytes) as usize;

            // Prevent DoS: reject messages larger than MAX_MESSAGE_SIZE
            if len > MAX_MESSAGE_SIZE {
                return Err(ChainError::NetworkError(format!("Message too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE)));
            }

            let mut buffer = vec![0u8; len];
            stream.read_exact(&mut buffer).await
                .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;

            let response: NetworkMessage = bincode::deserialize(&buffer)
                .map_err(|e| ChainError::NetworkError(format!("Deserialization failed: {}", e)))?;

            if let NetworkMessage::Blocks(blocks) = response {
                let mut chain = self.blockchain.write().await;

                println!("📥 Received batch of {} blocks", blocks.len());

                for block in blocks {
                    match chain.apply_block(block) {
                        Ok(_) => {
                            if let Err(e) = self.synchronizer.record_block_received(&node.addr()).await {
                                eprintln!("⚠️  Warning: Failed to record block received: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to apply block: {}", e);
                            if let Err(e) = self.synchronizer.record_sync_failure(&node.addr()).await {
                                eprintln!("⚠️  Warning: Failed to record sync failure: {}", e);
                            }
                        }
                    }
                }

                println!("✅ Applied batch successfully");
            }
        }

        // 3. Get peers from remote
        let mut stream = TcpStream::connect(&addr).await
            .map_err(|e| ChainError::NetworkError(format!("Failed to connect: {}", e)))?;

        let request = NetworkMessage::GetPeers;
        let data = bincode::serialize(&request)
            .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;

        let len = data.len() as u32;
        stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
        stream.write_all(&data).await
            .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;

        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await
            .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        // Prevent DoS: reject messages larger than MAX_MESSAGE_SIZE
        if len > MAX_MESSAGE_SIZE {
            return Err(ChainError::NetworkError(format!("Message too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE)));
        }

        let mut buffer = vec![0u8; len];
        stream.read_exact(&mut buffer).await
            .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;

        let response: NetworkMessage = bincode::deserialize(&buffer)
            .map_err(|e| ChainError::NetworkError(format!("Deserialization failed: {}", e)))?;

        if let NetworkMessage::Peers(new_peers) = response {
            let mut local_peers = self.peers.write().await;
            for peer in new_peers {
                if !local_peers.iter().any(|p| p.addr() == peer.addr()) {
                    println!("Discovered new peer: {}", peer.addr());
                    local_peers.push(peer);
                }
            }
        }

        let mut peers = self.peers.write().await;
        let peer = Node::new(host, port);
        if !peers.iter().any(|p| p.addr() == peer.addr()) {
            peers.push(peer);
        }

        Ok(())
    }
    
    pub async fn broadcast_transaction(&self, tx: &crate::transaction::Transaction) -> Result<(), ChainError> {
        let peers = self.peers.read().await;
        let message = NetworkMessage::NewTransaction(Box::new(tx.clone()));
        let data = bincode::serialize(&message)
            .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;

        for peer in peers.iter() {
            let mut stream = match TcpStream::connect(peer.addr()).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("❌ Failed to connect to peer {}: {}", peer.addr(), e);
                    continue;
                }
            };

            let len = data.len() as u32;
            if let Err(e) = stream.write_all(&len.to_be_bytes()).await {
                eprintln!("❌ Failed to write to peer {}: {}", peer.addr(), e);
                continue;
            }
            if let Err(e) = stream.write_all(&data).await {
                eprintln!("❌ Failed to write to peer {}: {}", peer.addr(), e);
                continue;
            }
            println!("📢 Broadcasted transaction to {}", peer.addr());
        }

        Ok(())
    }

    pub async fn broadcast_block(&self, block: &crate::blockchain::Block) -> Result<(), ChainError> {
        let peers = self.peers.read().await;
        let message = NetworkMessage::NewBlock(Box::new(block.clone()));
        let data = bincode::serialize(&message)
            .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;

        for peer in peers.iter() {
            let mut stream = match TcpStream::connect(peer.addr()).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("❌ Failed to connect to peer {}: {}", peer.addr(), e);
                    continue;
                }
            };

            let len = data.len() as u32;
            if let Err(e) = stream.write_all(&len.to_be_bytes()).await {
                eprintln!("❌ Failed to write to peer {}: {}", peer.addr(), e);
                continue;
            }
            if let Err(e) = stream.write_all(&data).await {
                eprintln!("❌ Failed to write to peer {}: {}", peer.addr(), e);
                continue;
            }
            println!("📢 Broadcasted block {} to {}", block.header.height, peer.addr());
        }

        Ok(())
    }

    pub async fn get_height(&self) -> u64 {
        let chain = self.blockchain.read().await;
        chain.blocks.last().map(|b| b.header.height).unwrap_or(0)
    }

    /// Return the current number of peers
    pub async fn peers_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.len()
    }

    /// Return a cloned list of peers
    pub async fn list_peers(&self) -> Vec<Node> {
        let peers = self.peers.read().await;
        peers.clone()
    }

    /// Validates an entire blockchain by checking all blocks
    pub fn validate_chain(chain: &Blockchain) -> bool {
        if chain.blocks.is_empty() {
            return false;
        }

        // Validate each block's proof of work and merkle root
        for block in &chain.blocks {
            if !block.verify_proof_of_work() {
                println!("❌ Block {} has invalid proof of work", block.header.height);
                return false;
            }

            let calculated_merkle = crate::blockchain::Block::calculate_merkle_root(&block.transactions);
            if block.header.merkle_root != calculated_merkle {
                println!("❌ Block {} has invalid merkle root", block.header.height);
                return false;
            }
        }

        // Validate block linkage
        for i in 1..chain.blocks.len() {
            let prev = &chain.blocks[i - 1];
            let curr = &chain.blocks[i];

            if curr.header.height != prev.header.height + 1 {
                println!("❌ Invalid block height at block {}", curr.header.height);
                return false;
            }

            if curr.header.previous_hash != prev.hash {
                println!("❌ Invalid block linkage at block {}", curr.header.height);
                return false;
            }
        }

        true
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum NetworkMessage {
    GetBlockHeaders { after_height: u64 },
    BlockHeaders(Vec<crate::blockchain::BlockHeader>),
    GetBlock(crate::blockchain::Sha256Hash),
    Block(Box<crate::blockchain::Block>),
    // Batch block requests for faster syncing
    GetBlocks(Vec<crate::blockchain::Sha256Hash>),
    Blocks(Vec<crate::blockchain::Block>),
    NewBlock(Box<crate::blockchain::Block>),
    NewTransaction(Box<crate::transaction::Transaction>),
    GetPeers,
    Peers(Vec<Node>),
    GetBlockchain,
    Blockchain(Blockchain),
    Ping,
    Pong,
}

async fn handle_connection(
    mut socket: TcpStream,
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<RwLock<Vec<Node>>>,
) -> Result<(), ChainError> {
    let mut len_bytes = [0u8; 4];
    socket.read_exact(&mut len_bytes).await
        .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    // Prevent DoS: reject messages larger than MAX_MESSAGE_SIZE
    if len > MAX_MESSAGE_SIZE {
        return Err(ChainError::NetworkError(format!("Message too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE)));
    }

    let mut buffer = vec![0u8; len];
    socket.read_exact(&mut buffer).await
        .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
    
    let message: NetworkMessage = bincode::deserialize(&buffer)
        .map_err(|e| ChainError::NetworkError(format!("Deserialization failed: {}", e)))?;
    
    match message {
        NetworkMessage::GetBlockHeaders { after_height } => {
            let chain = blockchain.read().await;
            let headers = chain.blocks
                .iter()
                .filter(|b| b.header.height > after_height)
                .map(|b| b.header.clone())
                .collect::<Vec<_>>();

            let response = NetworkMessage::BlockHeaders(headers);
            let data = bincode::serialize(&response)
                .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
            
            let len = data.len() as u32;
            socket.write_all(&len.to_be_bytes()).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            socket.write_all(&data).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            
            println!("📤 Sent {} block headers", chain.blocks.len());
        }
        NetworkMessage::GetBlock(hash) => {
            let chain = blockchain.read().await;
            if let Some(block) = chain.block_index.get(&hash) {
                let response = NetworkMessage::Block(Box::new(block.clone()));
                let data = bincode::serialize(&response)
                    .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;

                let len = data.len() as u32;
                socket.write_all(&len.to_be_bytes()).await
                    .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
                socket.write_all(&data).await
                    .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;

                println!("📤 Sent block {}", hex::encode(hash));
            }
        }
        // Batch block requests for faster syncing
        NetworkMessage::GetBlocks(hashes) => {
            let chain = blockchain.read().await;
            let mut blocks = Vec::new();

            for hash in hashes {
                if let Some(block) = chain.block_index.get(&hash) {
                    blocks.push(block.clone());
                }
            }

            if !blocks.is_empty() {
                let response = NetworkMessage::Blocks(blocks.clone());
                let data = bincode::serialize(&response)
                    .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;

                let len = data.len() as u32;
                socket.write_all(&len.to_be_bytes()).await
                    .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
                socket.write_all(&data).await
                    .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;

                println!("📤 Sent {} blocks in batch", blocks.len());
            }
        }
        NetworkMessage::GetPeers => {
            let peer_list = peers.read().await;
            let response = NetworkMessage::Peers(peer_list.clone());
            let data = bincode::serialize(&response)
                .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
            
            let len = data.len() as u32;
            socket.write_all(&len.to_be_bytes()).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            socket.write_all(&data).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            
            println!("📤 Sent peer list to peer");
        }
        NetworkMessage::GetBlockchain => {
            let chain = blockchain.read().await;
            let response = NetworkMessage::Blockchain(chain.clone());
            let data = bincode::serialize(&response)
                .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
            
            let len = data.len() as u32;
            socket.write_all(&len.to_be_bytes()).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            socket.write_all(&data).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            
            println!("📤 Sent blockchain to peer");
        }
        NetworkMessage::NewTransaction(tx) => {
            let mut chain = blockchain.write().await;
            if let Err(e) = chain.mempool.add_transaction(*tx) {
                eprintln!("❌ Failed to add new transaction to mempool: {}", e);
            } else {
                println!("✅ Added new transaction to mempool");
            }
        }
        NetworkMessage::NewBlock(block) => {
            let mut chain = blockchain.write().await;
            if let Err(e) = chain.apply_block(*block.clone()) {
                if let ChainError::OrphanBlock = e {
                    println!("Orphan block received, requesting parent");
                    let request = NetworkMessage::GetBlock(block.header.previous_hash);
                    let data = bincode::serialize(&request)
                        .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
                    
                    let len = data.len() as u32;
                    socket.write_all(&len.to_be_bytes()).await
                        .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
                    socket.write_all(&data).await
                        .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
                } else {
                    eprintln!("❌ Failed to apply new block: {}", e);
                }
            } else {
                println!("✅ Applied new block from peer");
            }
        }
        NetworkMessage::Ping => {
            let response = NetworkMessage::Pong;
            let data = bincode::serialize(&response)
                .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
            
            let len = data.len() as u32;
            socket.write_all(&len.to_be_bytes()).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            socket.write_all(&data).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
        }
        _ => {}
    }
    
    Ok(())
}
