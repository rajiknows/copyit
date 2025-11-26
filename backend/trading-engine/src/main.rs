// this is the main entrypoint of our trading engine
//
// we will have these modules or should we say services
// 1. Detector ( constantly detectes trades from every batch of txns)
// 2. Calculator ( calculates position of the follower )
// 3. Executor ( executes a tx and sends it to the blockhain )
// 4. Monitor ( position monitoring module )
// 5. Hyperliquid ( hyperliquid ws connection , all apis)

// use futures_util::future::ok;
use serde::Deserialize;
use tokio::sync::broadcast;

mod detector;
mod hyperliquid;

#[derive(Debug, Clone, Deserialize)]
pub struct WsTrade {
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
    pub hash: String,
    pub time: usize,
    pub tid: usize,
    pub users: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TradeMessage {
    pub channel: String,
    pub data: Vec<WsTrade>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting Hyperliquid Copy Trading Engine...\n");

    let (trade_tx, trade_rx) = broadcast::channel::<TradeMessage>(100);
    let (detected_tx, detected_rx1) = broadcast::channel::<detector::DetectedTrade>(100);
    let detected_rx2 = detected_tx.subscribe();

    let monitored_traders = vec![
        "0x5b5d51203a0f9079f8aeb098a6523a13f298c060".to_string(),
        "0x7fdafde5cfb5465924316eced2d3715494c517d1".to_string(), // Add more trader addresses here
    ];

    println!("Monitoring {} traders", monitored_traders.len());
    for (i, trader) in monitored_traders.iter().enumerate() {
        println!("  {}. {}", i + 1, trader);
    }
    println!();

    let btc_tx = trade_tx.clone();
    tokio::spawn(async move {
        println!("Connecting to Hyperliquid WebSocket...");
        match hyperliquid::ws::fetch_trades("BTC", btc_tx).await {
            Ok(_) => println!("WebSocket connected"),
            Err(e) => eprintln!("WebSocket error: {}", e),
        }
    });

    let sol_tx = trade_tx.clone();
    tokio::spawn(async move {
        println!("connecting ws for sol trades");
        match hyperliquid::ws::fetch_trades("SOL", sol_tx).await {
            Ok(_) => println!("WebSocket connected"),
            Err(e) => eprintln!("WebSocket error: {}", e),
        }
    });

    let detector_traders = monitored_traders.clone();
    tokio::spawn(async move {
        if let Err(e) = detector::init(trade_rx, detector_traders, detected_tx).await {
            eprintln!("Detector error: {}", e);
        }
    });

    tokio::spawn(async move {
        detector::monitor::start_stats_reporter(monitored_traders, detected_rx1).await;
    });

    let mut detected_rx = detected_rx2;
    loop {
        match detected_rx.recv().await {
            Ok(trade) => {
                println!(
                    "DETECTED TRADE: {} {} {} @ {} (value: ~${:.2})",
                    detector::parser::parse_side(&trade.side),
                    trade.size,
                    trade.coin,
                    trade.price,
                    detector::parser::calculate_trade_value(&trade.price, &trade.size)
                        .unwrap_or(0.0)
                );
                println!("   Trader: {}", trade.trader_address);
                println!("   Hash: {}\n", trade.tx_hash);
            }
            Err(e) => {
                eprintln!("Error receiving detected trade: {}", e);
            }
        }
    }
}
