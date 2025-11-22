//! Network node for TrinityChain - TUI Edition

use trinitychain::blockchain::Blockchain;
use trinitychain::persistence::Database;
use trinitychain::network::NetworkNode;
use std::env;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block as TuiBlock, Borders, List, ListItem, Paragraph},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

#[derive(Clone)]
struct NodeStats {
    port: u16,
    chain_height: u64,
    utxo_count: usize,
    peer_count: usize,
    uptime_secs: u64,
    blocks_received: u64,
    blocks_sent: u64,
    status: String,
    peers: Vec<String>,
}

impl Default for NodeStats {
    fn default() -> Self {
        Self {
            port: 0,
            chain_height: 0,
            utxo_count: 0,
            peer_count: 0,
            uptime_secs: 0,
            blocks_received: 0,
            blocks_sent: 0,
            status: "Initializing...".to_string(),
            peers: Vec::new(),
        }
    }
}

fn draw_ui(f: &mut ratatui::Frame, stats: &NodeStats) {
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(8),  // Status
            Constraint::Length(8),  // Stats
            Constraint::Min(5),     // Peers
            Constraint::Length(3),  // Footer
        ])
        .split(size);

    // Title
    let port_str = format!(" :{}", stats.port);
    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("🌐  ", Style::default().fg(Color::Cyan)),
            Span::styled("TRINITY NODE", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(&port_str, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("  🌐", Style::default().fg(Color::Cyan)),
        ]),
    ])
    .block(TuiBlock::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Status
    let status_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Status: ", Style::default().fg(Color::Gray)),
            Span::styled(&stats.status, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   Uptime: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}m {}s", stats.uptime_secs / 60, stats.uptime_secs % 60), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("   Network Activity: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("↓{} blocks  ", stats.blocks_received), Style::default().fg(Color::Blue)),
            Span::styled(format!("↑{} blocks", stats.blocks_sent), Style::default().fg(Color::Magenta)),
        ]),
    ];

    let status = Paragraph::new(status_text)
        .block(TuiBlock::default()
            .borders(Borders::ALL)
            .title("⚡ Node Status")
            .border_style(Style::default().fg(Color::Cyan)));
    f.render_widget(status, chunks[1]);

    // Stats
    let stats_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Chain Height: ", Style::default().fg(Color::Gray)),
            Span::styled(format!(" {} ", stats.chain_height), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   UTXO Set: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", stats.utxo_count), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("    Connected Peers: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", stats.peer_count), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let stats_widget = Paragraph::new(stats_text)
        .block(TuiBlock::default()
            .borders(Borders::ALL)
            .title("📊 Blockchain Stats")
            .border_style(Style::default().fg(Color::Blue)));
    f.render_widget(stats_widget, chunks[2]);

    // Peers List
    let peer_items: Vec<ListItem> = if stats.peers.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled("   No peers connected", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]))]
    } else {
        stats.peers.iter().enumerate().map(|(i, peer)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("   {}. ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled("🔗 ", Style::default().fg(Color::Green)),
                Span::styled(peer, Style::default().fg(Color::White)),
            ]))
        }).collect()
    };

    let peers_list = List::new(peer_items)
        .block(TuiBlock::default()
            .borders(Borders::ALL)
            .title("👥 Connected Peers")
            .border_style(Style::default().fg(Color::Magenta)));
    f.render_widget(peers_list, chunks[3]);

    // Footer
    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("'q'", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" to quit", Style::default().fg(Color::DarkGray)),
        ]),
    ]);
    f.render_widget(footer, chunks[4]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: trinity-node <port> [--peer <host:port>]");
        println!("\nExamples:");
        println!("  trinity-node 8333");
        println!("  trinity-node 8334 --peer 192.168.1.100:8333");
        return Ok(());
    }

    let port: u16 = args[1].parse().expect("Invalid port number");
    let db_path = "trinitychain.db".to_string();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db = Database::open(&db_path).expect("Failed to open database");
    let blockchain = db.load_blockchain().unwrap_or_else(|_| Blockchain::new());

    let stats = Arc::new(Mutex::new(NodeStats {
        port,
        chain_height: blockchain.blocks.last().map(|b| b.header.height).unwrap_or(0),
        utxo_count: blockchain.state.count(),
        status: "Starting...".to_string(),
        ..Default::default()
    }));

    let stats_clone = Arc::clone(&stats);
    let start_time = Instant::now();

    // Spawn node server
    let node = NetworkNode::new(blockchain, db_path);

    if args.len() >= 4 && args[2] == "--peer" {
        let peer_addr = &args[3];
        stats.lock().unwrap().peers.push(peer_addr.clone());
        stats.lock().unwrap().peer_count = 1;
    }

    let node_handle = tokio::spawn(async move {
        // Update status periodically
        loop {
            {
                let mut s = stats_clone.lock().unwrap();
                s.status = "Running".to_string();
                s.uptime_secs = start_time.elapsed().as_secs();
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

        let stats_lock = stats.lock().unwrap().clone();
        terminal.draw(|f| {
            draw_ui(f, &stats_lock);
        })?;

        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    node_handle.abort();

    Ok(())
}
