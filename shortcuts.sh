#!/bin/bash

# TrinityChain Quick Actions
# Source this file to use shortcuts: source shortcuts.sh

TRINITY_DIR="$(pwd)"

# Wallet commands
alias wallet="cargo run --quiet --bin trinity-wallet --manifest-path=$TRINITY_DIR/Cargo.toml"
alias wallet-new="cargo run --quiet --bin trinity-wallet-new --manifest-path=$TRINITY_DIR/Cargo.toml"
alias wallet-backup="cargo run --quiet --bin trinity-wallet-backup --manifest-path=$TRINITY_DIR/Cargo.toml"
alias wallet-restore="cargo run --quiet --bin trinity-wallet-restore --manifest-path=$TRINITY_DIR/Cargo.toml"

# Transaction commands
alias send="cargo run --quiet --bin trinity-send --manifest-path=$TRINITY_DIR/Cargo.toml"
alias balance="cargo run --quiet --bin trinity-balance --manifest-path=$TRINITY_DIR/Cargo.toml"
alias history="cargo run --quiet --bin trinity-history --manifest-path=$TRINITY_DIR/Cargo.toml"

# Mining commands
alias miner="cargo run --quiet --bin trinity-miner --manifest-path=$TRINITY_DIR/Cargo.toml"
alias mine-block="cargo run --quiet --bin trinity-mine-block --manifest-path=$TRINITY_DIR/Cargo.toml"

# Network commands
alias node="cargo run --quiet --bin trinity-node --manifest-path=$TRINITY_DIR/Cargo.toml"
alias api="cargo run --quiet --bin trinity-api --manifest-path=$TRINITY_DIR/Cargo.toml"
alias server="cargo run --quiet --bin trinity-server --manifest-path=$TRINITY_DIR/Cargo.toml"

# Utility commands
alias addressbook="cargo run --quiet --bin trinity-addressbook --manifest-path=$TRINITY_DIR/Cargo.toml"
alias bot="cargo run --quiet --bin trinity-telegram-bot --manifest-path=$TRINITY_DIR/Cargo.toml"

# Release mode aliases (faster execution, quieter output)
alias wallet-release="cargo run --quiet --release --bin trinity-wallet --manifest-path=$TRINITY_DIR/Cargo.toml"
alias miner-release="cargo run --quiet --release --bin trinity-miner --manifest-path=$TRINITY_DIR/Cargo.toml"
alias node-release="cargo run --quiet --release --bin trinity-node --manifest-path=$TRINITY_DIR/Cargo.toml"
alias api-release="cargo run --quiet --release --bin trinity-api --manifest-path=$TRINITY_DIR/Cargo.toml"
alias server-release="cargo run --quiet --release --bin trinity-server --manifest-path=$TRINITY_DIR/Cargo.toml"

# Quick shortcuts using pre-built binaries (fastest!)
alias tm="$TRINITY_DIR/target/release/trinity-miner"
alias tn="$TRINITY_DIR/target/release/trinity-node"
alias ta="$TRINITY_DIR/target/release/trinity-api"
alias ts="$TRINITY_DIR/target/release/trinity-server"
alias tw="$TRINITY_DIR/target/release/trinity-wallet"

echo "✅ TrinityChain shortcuts loaded!"
echo ""
echo "📦 Main Commands:"
echo "  wallet, send, balance, history"
echo "  miner, node, api, server (NEW!)"
echo ""
echo "🚀 Release Mode (optimized):"
echo "  miner-release, node-release, api-release, server-release"
echo ""
echo "⚡ Quick Binaries (pre-built, fastest):"
echo "  tm (miner), tn (node), ta (api), ts (server), tw (wallet)"
echo ""
