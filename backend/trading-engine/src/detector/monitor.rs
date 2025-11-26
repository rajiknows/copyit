use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::time::{Duration, interval};

#[derive(Debug, Clone)]
pub struct TraderStats {
    pub address: String,
    pub total_trades: u64,
    pub total_volume: f64,
    pub last_trade_time: Option<usize>,
}

impl TraderStats {
    pub fn new(address: String) -> Self {
        Self {
            address,
            total_trades: 0,
            total_volume: 0.0,
            last_trade_time: None,
        }
    }

    pub fn update(&mut self, trade_value: f64, timestamp: usize) {
        self.total_trades += 1;
        self.total_volume += trade_value;
        self.last_trade_time = Some(timestamp);
    }
}

pub struct TraderMonitor {
    stats: HashMap<String, TraderStats>,
}

impl TraderMonitor {
    pub fn new(traders: Vec<String>) -> Self {
        let mut stats = HashMap::new();
        for trader in traders {
            stats.insert(trader.clone(), TraderStats::new(trader));
        }
        Self { stats }
    }

    pub fn record_trade(&mut self, trader: &str, trade_value: f64, timestamp: usize) {
        if let Some(stats) = self.stats.get_mut(trader) {
            stats.update(trade_value, timestamp);
        }
    }

    pub fn get_stats(&self, trader: &str) -> Option<&TraderStats> {
        self.stats.get(trader)
    }

    pub fn print_all_stats(&self) {
        println!("\n📊 Trader Statistics:");
        println!("{:<50} | {:>10} | {:>15}", "Address", "Trades", "Volume");
        println!("{:-<80}", "");

        for (_, stats) in &self.stats {
            println!(
                "{:<50} | {:>10} | {:>15.2}",
                &stats.address[..50.min(stats.address.len())],
                stats.total_trades,
                stats.total_volume
            );
        }
    }
}

pub async fn start_stats_reporter(
    traders: Vec<String>,
    mut detected_rx: broadcast::Receiver<super::DetectedTrade>,
) {
    let mut monitor = TraderMonitor::new(traders);
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        tokio::select! {
            Ok(trade) = detected_rx.recv() => {
                if let Ok(value) = super::parser::calculate_trade_value(&trade.price, &trade.size) {
                    monitor.record_trade(&trade.trader_address, value, trade.timestamp);
                }
            }
            _ = ticker.tick() => {
                monitor.print_all_stats();
            }
        }
    }
}
