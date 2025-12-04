use tokio::sync::broadcast;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

use crate::hyperliquid::ws::WsFill;

#[derive(Debug, Clone)]
pub struct FullOrder {
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
    coin: String,
    dir: String,
    total_sz: Decimal,
    weighted_px: Decimal,  // sum(px * sz)
    timestamp: u64,
    last_seen: Instant,
    hash: String,
    oid: u64,
}

pub async fn start(mut rx: broadcast::Receiver<WsFill>, tx: broadcast::Sender<FullOrder>) {
    let mut pending: HashMap<u64, PendingOrder> = HashMap::new();

    loop {
        tokio::select! {
            Ok(fill) = rx.recv() => {
                let oid = fill.oid;
                let sz = Decimal::from_str(&fill.sz).unwrap_or(dec!(0));
                let px = Decimal::from_str(&fill.px).unwrap_or(dec!(0));
                let weighted = px * sz;

                let entry = pending.entry(oid).or_insert_with(|| PendingOrder {
                    coin: fill.coin.clone(),
                    dir: fill.dir.clone().unwrap_or("Unknown".to_string()),
                    total_sz: dec!(0),
                    weighted_px: dec!(0),
                    timestamp: fill.time,
                    last_seen: Instant::now(),
                    hash: fill.hash.clone(),
                    oid,
                });

                entry.total_sz += sz;
                entry.weighted_px += weighted;
                entry.last_seen = Instant::now();

                // If this is the first fill for this oid, spawn a debouncer
                if entry.total_sz == sz {
                    let oid_copy = oid;
                    let tx_clone = tx.clone();
                    let mut pending_clone = pending.clone();

                    tokio::spawn(async move {
                        sleep(Duration::from_millis(420)).await;

                        if let Some(final_order) = pending_clone.remove(&oid_copy) {
                            if final_order.last_seen.elapsed() >= Duration::from_millis(400) {
                                let avg_px = if final_order.total_sz > dec!(0) {
                                    final_order.weighted_px / final_order.total_sz
                                } else {
                                    dec!(0)
                                };

                                let full = FullOrder {
                                    coin: final_order.coin,
                                    dir: final_order.dir,
                                    total_sz: final_order.total_sz,
                                    avg_px,
                                    timestamp: final_order.timestamp,
                                    hash: final_order.hash,
                                    oid: final_order.oid,
                                };

                                let _ = tx_clone.send(full);
                            }
                        }
                    });
                }
            }

            _ = sleep(Duration::from_secs(5)) => {
                let now = Instant::now();
                pending.retain(|_, p| {
                    if now.duration_since(p.last_seen) > Duration::from_millis(600) {
                        let avg_px = if p.total_sz > dec!(0) { p.weighted_px / p.total_sz } else { dec!(0) };
                        let full = FullOrder {
                            coin: p.coin.clone(),
                            dir: p.dir.clone(),
                            total_sz: p.total_sz,
                            avg_px,
                            timestamp: p.timestamp,
                            hash: p.hash.clone(),
                            oid: p.oid,
                        };
                        let _ = tx.send(full);
                        false // remove
                    } else {
                        true // keep
                    }
                });
            }
        }
    }
}
