use log::info;
use sqlx::Row;
use std::{
    collections::HashMap,
};
use hyperliquid_rust_sdk::{BaseUrl, ClientLimit, ClientOrder, ClientOrderRequest, ExchangeClient};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_decimal_macros::dec;
use sqlx::{PgPool, postgres::PgRow};
use tokio::sync::{broadcast, RwLock};
use std::sync::Arc;

use crate::engine::grouper::FullOrder;

#[derive(Debug, Clone)]
pub struct FollowersCache {
    pub address: String,
    pub signature: String,
    pub ratio: Decimal,
    pub max_risk: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct OrderTask {
    pub order: FullOrder,
    pub follower: FollowersCache,
}

const WORKER_COUNT: usize = 10;
const CHANNEL_CAPACITY: usize = 1000;

type SharedCache = Arc<RwLock<HashMap<String, Vec<FollowersCache>>>>;

pub async fn start(mut rx: broadcast::Receiver<FullOrder>, pool: PgPool, agentkey: &str) {
    let cache: SharedCache = Arc::new(RwLock::new(HashMap::new()));

    // Initial preload
    {
        let mut write_lock = cache.write().await;
        preload_followers(&pool, &mut write_lock).await;
    }

    // Background refresher task
    let cache_clone = cache.clone();
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            let mut new_cache = HashMap::new();
            preload_followers(&pool_clone, &mut new_cache).await;

            // Atomic replacement under write lock
            let mut write_lock = cache_clone.write().await;
            *write_lock = new_cache;
        }
    });

    // Channel for distributing tasks to workers
    let (tx, rx_orders) = broadcast::channel::<OrderTask>(CHANNEL_CAPACITY);

    // Spawn worker pool
    for worker_id in 0..WORKER_COUNT {
        let rx_orders = rx_orders.resubscribe();
        let agentkey = agentkey.to_string();
        tokio::spawn(async move {
            order_worker(worker_id, rx_orders, &agentkey).await;
        });
    }

    // Main dispatcher loop
    while let Ok(order) = rx.recv().await {
        let followers = {
            let read_lock = cache.read().await;
            match read_lock.get(&order.user) {
                Some(f) => f.clone(),
                None => continue,
            }
        };

        for follower in followers {
            let task = OrderTask {
                order: order.clone(),
                follower,
            };

            if tx.send(task).is_err() {
                log::warn!("Order queue full — dropping task (backpressure)");
            }
        }
    }
}

async fn order_worker(worker_id: usize, mut rx: broadcast::Receiver<OrderTask>, agentkey: &str) {
    let wallet = agentkey.parse().unwrap();
    let exchange_client = match ExchangeClient::new(None, wallet, Some(BaseUrl::Testnet), None, None).await {
        Ok(client) => client,
        Err(e) => {
            log::error!("Worker {} failed to initialize client: {}", worker_id, e);
            return;
        }
    };

    info!("Worker {} started", worker_id);

    while let Ok(task) = rx.recv().await {
        let result = handle_follower_order(&exchange_client, &task.order, &task.follower).await;

        match result {
            Ok(oid) => {
                info!("Worker {}: Executed order for {} — OID: {}", worker_id, task.follower.address, oid);
            }
            Err(e) => {
                log::error!("Worker {}: Failed for {} — {}", worker_id, task.follower.address, e);
            }
        }
    }

    info!("Worker {} shutting down", worker_id);
}

async fn handle_follower_order(
    exchange_client: &ExchangeClient,
    order: &FullOrder,
    follower: &FollowersCache,
) -> Result<u64, String> {
    let mut sz = order.total_sz * follower.ratio;

    if let Some(max_risk) = follower.max_risk {
        let notional = sz * order.avg_px;
        if notional > max_risk {
            sz = max_risk / order.avg_px.max(dec!(0.0000001));
        }
    }

    if sz <= dec!(0.000001) {
        return Err("Order size too small".to_string());
    }

    let is_buy = matches!(order.dir.as_str(), "Open Long" | "Close Short");

    let client_order = ClientOrderRequest {
        asset: order.coin.clone(),
        is_buy,
        reduce_only: false,
        limit_px: order.avg_px.to_f64().unwrap_or_default(),
        sz: sz.round_dp(8).to_f64().unwrap_or_default(),
        cloid: None,
        order_type: ClientOrder::Limit(ClientLimit {
            tif: "Gtc".to_string(),
        }),
    };

    let response = exchange_client
        .order(client_order, None)
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    match response {
        hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(exchange_response) => {
            let data = exchange_response.data.ok_or("No data in response")?;
            match &data.statuses[0] {
                hyperliquid_rust_sdk::ExchangeDataStatus::Filled(o) => Ok(o.oid),
                hyperliquid_rust_sdk::ExchangeDataStatus::Resting(o) => Ok(o.oid),
                _ => Err(format!("Unexpected status: {:?}", data.statuses[0])),
            }
        }
        hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
            Err(format!("Exchange error: {}", e))
        }
    }
}

async fn preload_followers(
    pool: &PgPool,
    cache: &mut HashMap<String, Vec<FollowersCache>>,
) {
    let rows: Vec<PgRow> = sqlx::query(
        "SELECT f.address, f.agent_signature, c.trader_address, c.ratio, c.max_risk_per_trade
         FROM copy_configs c
         JOIN followers f ON c.follower_id = f.id
         WHERE c.is_active = true",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    cache.clear();
    for row in rows {
        let trader: String = row.get("trader_address");
        let follower = FollowersCache {
            address: row.get("address"),
            signature: row.get("agent_signature"),
            ratio: row.get("ratio"),
            max_risk: row.get("max_risk_per_trade"),
        };
        cache.entry(trader).or_default().push(follower);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preload() {}
}
