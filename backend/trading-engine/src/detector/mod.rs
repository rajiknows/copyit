// in this module we will have a long list of traders in memory we will have thier public adddress
// we will then consume the channel that gives us the trades if we identify a trade by the traders
// we send a trade message to the redis queue for further processing
use crate::TradeMessage;
use std::collections::HashSet;
use tokio::sync::broadcast;

pub mod monitor;
pub mod parser;

/// Detected trade that matches a monitored trader
#[derive(Debug, Clone)]
pub struct DetectedTrade {
    pub trader_address: String,
    pub coin: String,
    pub side: String, // "A" (ask/sell) or "B" (bid/buy)
    pub price: String,
    pub size: String,
    pub timestamp: usize,
    pub trade_id: usize,
    pub tx_hash: String,
}

/// Initialize the trade detector
/// Listens to incoming trades and filters for monitored traders
pub async fn init(
    mut trade_channel: broadcast::Receiver<TradeMessage>,
    traders: Vec<String>,
    detected_tx: broadcast::Sender<DetectedTrade>,
) -> anyhow::Result<()> {
    let monitored_traders: HashSet<String> = traders.into_iter().collect();

    println!(
        "Detector initialized. Monitoring {} traders",
        monitored_traders.len()
    );

    loop {
        match trade_channel.recv().await {
            Ok(trade_msg) => {
                for ws_trade in trade_msg.data {
                    // Check if any user in this trade is monitored
                    for user in &ws_trade.users {
                        if monitored_traders.contains(user) {
                            println!("Detected trade from monitored trader: {}", user);

                            let detected = DetectedTrade {
                                trader_address: user.clone(),
                                coin: ws_trade.coin.clone(),
                                side: ws_trade.side.clone(),
                                price: ws_trade.px.clone(),
                                size: ws_trade.sz.clone(),
                                timestamp: ws_trade.time,
                                trade_id: ws_trade.tid,
                                tx_hash: ws_trade.hash.clone(),
                            };

                            if let Err(e) = detected_tx.send(detected) {
                                eprintln!(" Failed to send detected trade: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(" Error receiving trade: {}", e);
            }
        }
    }
}
