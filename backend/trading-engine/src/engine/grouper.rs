use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

use crate::channel::WsFillChannel;

#[derive(Debug, Clone)]
pub struct FullOrder {
    pub user:String,
    pub coin: String,
    pub dir: String,           // "Open Long", "Close Short", etc.
    pub total_sz: Decimal,
    pub avg_px: Decimal,
    pub timestamp: u64,
    pub hash: String,
    pub oid: u64,
}

#[derive(Debug, Clone)]
struct PendingOrder {
    user: String,
    coin: String,
    dir: String,
    total_sz: Decimal,
    weighted_px: Decimal,  // sum(px * sz)
    timestamp: u64,
    last_seen: Instant,
    hash: String,
    oid: u64,
}

pub async fn start(mut rx: broadcast::Receiver<WsFillChannel>, tx: broadcast::Sender<FullOrder>) {
    let pending = Arc::new(Mutex::new(HashMap::new()));

    loop {
        tokio::select! {
            Ok(wsfill) = rx.recv() => {
                let oid = wsfill.fill.oid;
                let sz = Decimal::from_str(&wsfill.fill.sz).unwrap_or(dec!(0));
                let px = Decimal::from_str(&wsfill.fill.px).unwrap_or(dec!(0));
                let weighted = px * sz;

                let mut pending_guard = pending.lock().await;

                let entry = pending_guard.entry(oid).or_insert_with(|| PendingOrder {
                    user:wsfill.user.clone(),
                    coin: wsfill.fill.coin.clone(),
                    dir: wsfill.fill.dir.clone().unwrap_or("Unknown".to_string()),
                    total_sz: dec!(0),
                    weighted_px: dec!(0),
                    timestamp: wsfill.fill.time,
                    last_seen: Instant::now(),
                    hash: wsfill.fill.hash.clone(),
                    oid,
                });

                entry.total_sz += sz;
                entry.weighted_px += weighted;
                entry.last_seen = Instant::now();

                // If this is the first fill for this oid, spawn a debouncer
                if entry.total_sz == sz {
                    let oid_copy = oid;
                    let tx_clone = tx.clone();
                    let pending_clone = Arc::clone(&pending);

                    tokio::spawn(async move {
                        sleep(Duration::from_millis(420)).await;

                        let mut pending_guard = pending_clone.lock().await;
                        if let Some(final_order) = pending_guard.remove(&oid_copy) {
                            if final_order.last_seen.elapsed() >= Duration::from_millis(400) {
                                let avg_px = if final_order.total_sz > dec!(0) {
                                    final_order.weighted_px / final_order.total_sz
                                } else {
                                    dec!(0)
                                };

                                let full = FullOrder {
                                    user:final_order.user,
                                    coin: final_order.coin,
                                    dir: final_order.dir,
                                    total_sz: final_order.total_sz,
                                    avg_px,
                                    timestamp: final_order.timestamp,
                                    hash: final_order.hash,
                                    oid: final_order.oid,
                                };
                                println!("{:?}", full.clone());

                                let _ = tx_clone.send(full);
                            }
                        }
                    });
                }
            }

            _ = sleep(Duration::from_secs(5)) => {
                let mut pending_guard = pending.lock().await;
                let now = Instant::now();
                pending_guard.retain(|_, p| {
                    if now.duration_since(p.last_seen) > Duration::from_millis(600) {
                        let avg_px = if p.total_sz > dec!(0) { p.weighted_px / p.total_sz } else { dec!(0) };
                        let full = FullOrder {
                            user: p.user.clone(),
                            coin: p.coin.clone(),
                            dir: p.dir.clone(),
                            total_sz: p.total_sz,
                            avg_px,
                            timestamp: p.timestamp,
                            hash: p.hash.clone(),
                            oid: p.oid,
                        };
                        let _ = tx.send(full.clone());
                        false // remove
                    } else {
                        true // keep
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::WsFillChannel;
    use crate::hyperliquid::ws::WsFill;
    use tokio::sync::broadcast;
    use tokio::time::{self, Duration};
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_order_grouping() {
        time::pause();

        let (fill_tx, fill_rx) = broadcast::channel::<WsFillChannel>(16);
        let (order_tx, mut order_rx) = broadcast::channel::<FullOrder>(16);

        tokio::spawn(start(fill_rx, order_tx.clone()));

        let oid = 123;
        let user = "test_user".to_string();

        let fill1 = WsFill {
            coin: "BTC".to_string(),
            px: "50000.0".to_string(),
            sz: "1.0".to_string(),
            side: "B".to_string(),
            time: 1,
            hash: "hash1".to_string(),
            oid,
            startPosition: None,
            closedPnl: None,
            dir: Some("Open Long".to_string()),
            crossed: false,
            fee: "10.0".to_string(),
            feeToken: "USDC".to_string(),
        };

        let fill2 = WsFill {
            coin: "BTC".to_string(),
            px: "51000.0".to_string(),
            sz: "2.0".to_string(),
            side: "B".to_string(),
            time: 2,
            hash: "hash2".to_string(),
            oid,
            startPosition: None,
            closedPnl: None,
            dir: Some("Open Long".to_string()),
            crossed: false,
            fee: "20.0".to_string(),
            feeToken: "USDC".to_string(),
        };

        fill_tx.send(WsFillChannel { fill: fill1, user: user.clone() }).unwrap();
        // Brief pause to ensure the first fill is processed and the debouncer task is spawned
        time::sleep(Duration::from_millis(10)).await;
        fill_tx.send(WsFillChannel { fill: fill2, user: user.clone() }).unwrap();

        // Advance time to trigger the debouncer
        time::advance(Duration::from_millis(500)).await;

        let full_order = order_rx.recv().await.unwrap();

        let expected_total_sz = dec!(3.0);
        let expected_avg_px = (dec!(50000.0) * dec!(1.0) + dec!(51000.0) * dec!(2.0)) / dec!(3.0);

        assert_eq!(full_order.oid, oid);
        assert_eq!(full_order.total_sz, expected_total_sz);
        assert_eq!(full_order.avg_px.round_dp(2), expected_avg_px.round_dp(2));
    }
}
