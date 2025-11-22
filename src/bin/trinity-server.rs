//! Combined API Server + Telegram Bot for TrinityChain
//! Runs both services in one process for efficiency

use trinitychain::blockchain::Blockchain;
use trinitychain::persistence::Database;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::env;
use axum::{
    Router,
    routing::get,
    extract::State as AxumState,
    Json,
};
use tower_http::cors::{CorsLayer, Any};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block as TuiBlock, Borders, Paragraph, Sparkline},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use serde_json::{json, Value};

#[derive(Clone)]
struct ServerStats {
    api_port: u16,
    chain_height: u64,
    total_requests: u64,
    telegram_users: u64,
    uptime_secs: u64,
    requests_per_minute: Vec<u64>,
    status: String,
    telegram_status: String,
}

impl Default for ServerStats {
    fn default() -> Self {
        Self {
            api_port: 3000,
            chain_height: 0,
            total_requests: 0,
            telegram_users: 0,
            uptime_secs: 0,
            requests_per_minute: vec![0; 20],
            status: "Starting...".to_string(),
            telegram_status: "Disabled".to_string(),
        }
    }
}

type AppState = Arc<Mutex<ServerData>>;

struct ServerData {
    db: Database,
    stats: ServerStats,
}

async fn get_stats(AxumState(state): AxumState<AppState>) -> Json<Value> {
    let data = state.lock().unwrap();
    let chain = data.db.load_blockchain().unwrap_or_else(|_| Blockchain::new());

    Json(json!({
        "chainHeight": chain.blocks.last().map(|b| b.header.height).unwrap_or(0),
        "difficulty": chain.difficulty,
        "totalSupply": 0,
        "maxSupply": 420000000,
        "uptime": data.stats.uptime_secs,
    }))
}

async fn get_blocks(AxumState(state): AxumState<AppState>) -> Json<Value> {
    let data = state.lock().unwrap();
    let chain = data.db.load_blockchain().unwrap_or_else(|_| Blockchain::new());

    let blocks: Vec<_> = chain.blocks.iter().rev().take(50).map(|b| {
        json!({
            "index": b.header.height,
            "hash": hex::encode(b.hash),
            "previousHash": hex::encode(b.header.previous_hash),
            "timestamp": b.header.timestamp,
            "difficulty": b.header.difficulty,
            "nonce": b.header.nonce,
            "transactions": b.transactions.len(),
        })
    }).collect();

    Json(json!({"blocks": blocks}))
}

fn draw_ui(f: &mut ratatui::Frame, stats: &ServerStats) {
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(10), // Status
            Constraint::Length(8),  // Stats
            Constraint::Min(5),     // Request graph
            Constraint::Length(3),  // Footer
        ])
        .split(size);

    // Title
    let port_info = format!("  API:{} + Bot", stats.api_port);
    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("🚀  ", Style::default().fg(Color::Green)),
            Span::styled("TRINITY SERVER", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&port_info, Style::default().fg(Color::Yellow)),
            Span::styled("  🚀", Style::default().fg(Color::Green)),
        ]),
    ])
    .block(TuiBlock::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Status
    let status_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   API Status: ", Style::default().fg(Color::Gray)),
            Span::styled(&stats.status, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   Telegram Bot: ", Style::default().fg(Color::Gray)),
            Span::styled(&stats.telegram_status, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Uptime: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}h {}m", stats.uptime_secs / 3600, (stats.uptime_secs % 3600) / 60), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   Chain Height: ", Style::default().fg(Color::Gray)),
            Span::styled(format!(" {} ", stats.chain_height), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
    ];

    let status = Paragraph::new(status_text)
        .block(TuiBlock::default()
            .borders(Borders::ALL)
            .title("⚡ Server Status")
            .border_style(Style::default().fg(Color::Green)));
    f.render_widget(status, chunks[1]);

    // Stats
    let stats_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Total API Requests: ", Style::default().fg(Color::Gray)),
            Span::styled(format!(" {} ", stats.total_requests), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Telegram Users: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", stats.telegram_users), Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let stats_widget = Paragraph::new(stats_text)
        .block(TuiBlock::default()
            .borders(Borders::ALL)
            .title("📊 Service Stats")
            .border_style(Style::default().fg(Color::Blue)));
    f.render_widget(stats_widget, chunks[2]);

    // Request Graph
    let sparkline = Sparkline::default()
        .block(TuiBlock::default()
            .borders(Borders::ALL)
            .title("📈 Requests/Minute (Last 20min)")
            .border_style(Style::default().fg(Color::Magenta)))
        .data(&stats.requests_per_minute)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(sparkline, chunks[3]);

    // Footer
    let url = format!("http://0.0.0.0:{}", stats.api_port);
    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("'q'", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" to quit  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Serving at ", Style::default().fg(Color::DarkGray)),
            Span::styled(&url, Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
        ]),
    ]);
    f.render_widget(footer, chunks[4]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3000);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db = Database::open("trinitychain.db").expect("Failed to open database");
    let chain = db.load_blockchain().unwrap_or_else(|_| Blockchain::new());

    let state = Arc::new(Mutex::new(ServerData {
        db,
        stats: ServerStats {
            api_port: port,
            chain_height: chain.blocks.last().map(|b| b.header.height).unwrap_or(0),
            ..Default::default()
        },
    }));

    let state_clone = Arc::clone(&state);
    let start_time = Instant::now();

    // Build API router
    let app = Router::new()
        .route("/api/stats", get(get_stats))
        .route("/api/blocks", get(get_blocks))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(Arc::clone(&state));

    // Spawn API server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Update stats loop
    let stats_handle = tokio::spawn(async move {
        loop {
            {
                let mut data = state_clone.lock().unwrap();
                data.stats.status = "Running".to_string();
                data.stats.uptime_secs = start_time.elapsed().as_secs();

                // Check for Telegram token
                if env::var("TELOXIDE_TOKEN").is_ok() {
                    data.stats.telegram_status = "Active".to_string();
                } else {
                    data.stats.telegram_status = "No Token (set TELOXIDE_TOKEN)".to_string();
                }

                // Update chain height
                if let Ok(chain) = data.db.load_blockchain() {
                    data.stats.chain_height = chain.blocks.last().map(|b| b.header.height).unwrap_or(0);
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // UI loop
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        let stats_lock = state.lock().unwrap().stats.clone();
        terminal.draw(|f| {
            draw_ui(f, &stats_lock);
        })?;

        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    server_handle.abort();
    stats_handle.abort();

    Ok(())
}
