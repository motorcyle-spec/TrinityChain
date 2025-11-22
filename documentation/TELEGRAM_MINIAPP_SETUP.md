# Telegram Mini App Setup Guide

## Current Status

✅ **Trinity API Server**: Running on `http://127.0.0.1:3000`
✅ **Telegram Bot**: Running (@TrinityChainBot)
❌ **Public Tunnel**: Needs manual setup due to Termux limitations

## Issue

Termux on Android cannot run standard Linux binaries (ngrok, cloudflared, localtunnel). You need to expose your local API to the internet for the Telegram Mini App to work.

## Solutions

### Option 1: Use SSH Tunnel (Recommended for Termux)

If you have access to a remote server with SSH:

```bash
# On your Termux device
ssh -R 80:localhost:3000 serveo.net
```

This will give you a public URL like `https://yourname.serveo.net`

### Option 2: Use FRP (Fast Reverse Proxy)

1. Download FRP for ARM64:
```bash
cd ~
wget https://github.com/fatedier/frp/releases/download/v0.52.3/frp_0.52.3_linux_arm64.tar.gz
tar -xzf frp_0.52.3_linux_arm64.tar.gz
cd frp_0.52.3_linux_arm64
```

2. Create `frpc.ini`:
```ini
[common]
server_addr = free.frp.fun
server_port = 7000

[trinity-api]
type = http
local_ip = 127.0.0.1
local_port = 3000
custom_domains = trinity-YOURNAME.free.frp.fun
```

3. Run:
```bash
./frpc -c frpc.ini
```

### Option 3: Use Serveo (Simplest)

```bash
ssh -o StrictHostKeyChecking=no -R 80:localhost:3000 serveo.net
```

Copy the public URL it gives you (e.g., `https://abc123.serveo.net`)

## After Getting Public URL

### 1. Update Bot Configuration

Edit `src/bin/trinity-telegram-bot.rs` and look for the Mini App URL configuration (around line 42-44 or in the `/start` command):

```rust
// Replace the URL with your public tunnel URL
let webapp_url = "https://YOUR-TUNNEL-URL.com";
```

### 2. Set Bot Menu Button

Use @BotFather:
1. Send `/mybots`
2. Select @TrinityChainBot
3. Select "Bot Settings" → "Menu Button"
4. Set URL to: `https://YOUR-TUNNEL-URL.com/dashboard/index.html`

### 3. Update Dashboard API Endpoint

The dashboard (`dashboard/app.js`) already supports configuring the API via:
- Query parameter: `?api=https://YOUR-TUNNEL-URL.com`
- Or it defaults to relative URLs

### 4. Fix the `/dashboard` Command

Edit trinity-telegram-bot.rs and update the `/dashboard` command to fetch live data from the API instead of hardcoded values.

## Testing

1. Open Telegram and search for @TrinityChainBot
2. Send `/start`
3. Click the menu button (bottom-left)
4. The Mini App should load with live blockchain data!

## Current Running Services

- **API**: http://127.0.0.1:3000
- **Bot**: @TrinityChainBot (bot ID: c3fb94)
- **Blockchain Height**: 19,718 blocks

## Next Steps

1. Choose a tunneling solution above
2. Get your public URL
3. Update bot with the URL
4. Restart the bot
5. Test the Mini App!
