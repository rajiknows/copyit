// this is the main entrypoint of our trading engine
//
// we will have these modules or should we say services
// 1. Detector ( constantly detectes trades from every batch of txns)
// 2. Calculator ( calculates position of the follower )
// 3. Executor ( executes a tx and sends it to the blockhain )
// 4. Monitor ( position monitoring module )
// 5. Hyperliquid ( hyperliquid ws connection , all apis)

use std::env;

// use futures_util::future::ok;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{api::Server, hyperliquid::ws::fetch_fills_with_retry};

mod api;
mod cron;
mod engine;
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

// #[derive(Serialize, Debug, Clone)]
// pub struct FullOrder {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting Hyperliquid Copy Trading Engine...\n");
    dotenvy::dotenv().ok();

    let db_url = env::var("DB_URL").expect("DB_URL must be set");

    let (trade_tx, trade_rx) = broadcast::channel::<TradeMessage>(100);
    let (detected_tx, detected_rx1) = broadcast::channel::<engine::DetectedTrade>(100);
    let detected_rx2 = detected_tx.subscribe();

    let monitored_traders = vec![
        "0x5b5d51203a0f9079f8aeb098a6523a13f298c060".to_string(),
        "0x7fdafde5cfb5465924316eced2d3715494c517d1".to_string(),
    ];

    let (tx, mut rx) = tokio::sync::broadcast::channel(10_000);

    for trader in monitored_traders {
        let tx = tx.clone();
        tokio::spawn(async move {
            fetch_fills_with_retry(trader, tx).await;
        });
    }

    let (full_order_tx, mut full_order_reciever) = tokio::sync::broadcast::channel(10_000);
    let grouper_rx = rx.resubscribe();
    tokio::spawn(async move {
        println!("grouper starts");
        engine::grouper::start(grouper_rx, full_order_tx).await;
    });
    while let Ok(full_order) = full_order_reciever.recv().await{
            println!("{:?}", full_order);
    }

    // grouper -> executor
    tokio::spawn(async move {
        println!("executor started");
        engine::executor::start(full_order_reciever).await;
    });
    let server = Server::new(3000, db_url);
    server.start().await?;

    loop {
        match rx.recv().await {
            Ok(trade) => {
                println!("{:?}", trade);
            }
            Err(e) => {
                eprintln!("Error receiving detected trade: {}", e);
            }
        }
    }
}
