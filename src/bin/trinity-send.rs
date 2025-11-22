//! Send triangles to another address - Beautiful edition!

use trinitychain::persistence::Database;
use trinitychain::transaction::{Transaction, TransferTx};
use trinitychain::crypto::KeyPair;
use trinitychain::network::NetworkNode;
use secp256k1::SecretKey;
use std::env;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

const LOGO: &str = r#"
╔═══════════════════════════════════════════════════════════════╗
║      ████████╗██████╗ ██╗███╗   ██╗██╗████████╗██╗   ██╗     ║
║      ╚══██╔══╝██╔══██╗██║████╗  ██║██║╚══██╔══╝╚██╗ ██╔╝     ║
║         ██║   ██████╔╝██║██╔██╗ ██║██║   ██║    ╚████╔╝      ║
║         ██║   ██╔══██╗██║██║╚██╗██║██║   ██║     ╚██╔╝       ║
║         ██║   ██║  ██║██║██║ ╚████║██║   ██║      ██║        ║
║         ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝   ╚═╝      ╚═╝        ║
║                 🔺 Blockchain Transfer 🔺                     ║
╚═══════════════════════════════════════════════════════════════╝
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("{}", LOGO.bright_cyan());
        println!("{}", "╔══════════════════════════════════════════════════════════╗".bright_yellow());
        println!("{}", "║                      📖 Usage Guide                      ║".bright_yellow().bold());
        println!("{}", "╠══════════════════════════════════════════════════════════╣".bright_yellow());
        println!("{}", "║                                                          ║".bright_yellow());
        println!("{}", "║  Usage:                                                  ║".bright_yellow());
        println!("{}", "║    send <to_address> <triangle_hash> [memo]              ║".white());
        println!("{}", "║                                                          ║".bright_yellow());
        println!("{}", "║  Examples:                                               ║".bright_yellow());
        println!("{}", "║    send abc123... def456...                              ║".white());
        println!("{}", "║    send abc123... def456... \"Payment for services\"      ║".white());
        println!("{}", "║                                                          ║".bright_yellow());
        println!("{}", "╚══════════════════════════════════════════════════════════╝".bright_yellow());
        println!();
        std::process::exit(1);
    }

    println!("{}", LOGO.bright_cyan());

    let to_address = &args[1];
    let triangle_hash = &args[2];
    let memo = if args.len() > 3 {
        Some(args[3..].join(" "))
    } else {
        None
    };

    println!("{}", "┌─────────────────────────────────────────────────────────────┐".bright_magenta());
    println!("{}", "│                  💸 INITIATING TRANSFER                     │".bright_magenta().bold());
    println!("{}", "└─────────────────────────────────────────────────────────────┘".bright_magenta());
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );

    pb.set_message("Loading wallet...");
    pb.enable_steady_tick(Duration::from_millis(100));

    let home = std::env::var("HOME")?;

    // Support WALLET_NAME environment variable for multi-wallet support
    let wallet_name = std::env::var("WALLET_NAME").unwrap_or_else(|_| String::new());
    let wallet_file = if wallet_name.is_empty() {
        format!("{}/.trinitychain/wallet.json", home)
    } else {
        format!("{}/.trinitychain/wallet_{}.json", home, wallet_name)
    };

    let wallet_content = std::fs::read_to_string(&wallet_file)
        .map_err(|e| format!("Wallet not found at {}: {}", wallet_file, e))?;
    let wallet_data: serde_json::Value = serde_json::from_str(&wallet_content)?;

    let from_address = wallet_data["address"].as_str()
        .ok_or("Wallet address not found")?
        .to_string();
    let secret_hex = wallet_data["secret_key"].as_str()
        .ok_or("Secret key not found")?;
    let secret_bytes = hex::decode(secret_hex)?;
    let secret_key = SecretKey::from_slice(&secret_bytes)?;
    let keypair = KeyPair::from_secret_key(secret_key);

    pb.set_message("Loading blockchain...");

    let db = Database::open("trinitychain.db")?;
    let mut chain = db.load_blockchain()?;

    pb.set_message("Looking up triangle...");

    let full_hash = *chain.state.utxo_set.keys()
        .find(|h| hex::encode(h).starts_with(triangle_hash))
        .ok_or_else(|| format!("Triangle with hash prefix {} not found", triangle_hash))?;

    let triangle = chain.state.utxo_set.get(&full_hash)
        .ok_or("Triangle not found in UTXO set")?
        .clone();

    pb.finish_and_clear();

    let full_hash_hex = hex::encode(full_hash);
    let full_hash_display = if full_hash_hex.len() > 20 {
        format!("{}...{}", &full_hash_hex[..10], &full_hash_hex[full_hash_hex.len()-10..])
    } else {
        full_hash_hex.clone()
    };
    let from_display = if from_address.len() > 20 {
        format!("{}...{}", &from_address[..10], &from_address[from_address.len()-10..])
    } else {
        from_address.clone()
    };
    let to_display = if to_address.len() > 20 {
        format!("{}...{}", &to_address[..10], &to_address[to_address.len()-10..])
    } else {
        to_address.to_string()
    };

    println!("{}", "╔══════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║              🔍 TRANSACTION DETAILS                      ║".bright_cyan().bold());
    println!("{}", "╠══════════════════════════════════════════════════════════╣".bright_cyan());
    println!("{}", format!("║  🔺 Triangle: {:<42} ║", full_hash_display).cyan());
    println!("{}", format!("║  📐 Area: {:<47.6} ║", triangle.area()).cyan());
    println!("{}", format!("║  👤 From: {:<47} ║", from_display).cyan());
    println!("{}", format!("║  🎯 To: {:<49} ║", to_display).cyan());
    if let Some(ref m) = memo {
        let memo_display = if m.len() > 45 {
            format!("{}...", &m[..42])
        } else {
            m.clone()
        };
        println!("{}", format!("║  📝 Memo: {:<47} ║", memo_display).cyan());
    }
    println!("{}", "╚══════════════════════════════════════════════════════════╝".bright_cyan());
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    pb.set_message("Creating transaction...");

    let mut tx = TransferTx::new(full_hash, to_address.to_string(), from_address.clone(), 0.0, chain.blocks.len() as u64);

    if let Some(m) = memo {
        tx = tx.with_memo(m)?;
    }

    pb.set_message("Signing transaction...");

    let message = tx.signable_message();
    let signature = keypair.sign(&message)?;
    let public_key = keypair.public_key.serialize().to_vec();
    tx.sign(signature, public_key);

    let transaction = Transaction::Transfer(tx);
    chain.mempool.add_transaction(transaction.clone())?;

    pb.set_message("Broadcasting to network...");

    let network_node = NetworkNode::new(chain, "trinitychain.db".to_string());
    network_node.broadcast_transaction(&transaction).await?;

    pb.finish_and_clear();

    println!("{}", "╔══════════════════════════════════════════════════════════╗".bright_green());
    println!("{}", "║              ✅ TRANSACTION SUCCESSFUL!                  ║".bright_green().bold());
    println!("{}", "╠══════════════════════════════════════════════════════════╣".bright_green());
    println!("{}", "║  Your transaction has been broadcasted to the network   ║".green());
    println!("{}", "║  and will be included in the next block!                ║".green());
    println!("{}", "╚══════════════════════════════════════════════════════════╝".bright_green());
    println!();
    println!("{}", "🎉 Transfer complete! The triangle is on its way!".bright_blue());
    println!();

    Ok(())
}