use log::info;
use sqlx::Row;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use hyperliquid_rust_sdk::{BaseUrl, ClientLimit, ClientOrder, ClientOrderRequest, ExchangeClient};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
use sqlx::{PgPool, postgres::PgRow};
use tokio::sync::{broadcast};

use crate::engine::grouper::FullOrder;

#[derive(Debug, Clone)]
pub struct FollowersCache {
    address: String,
    signature: String,
    ratio: Decimal,
    max_risk: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct OrderTask {
    order: FullOrder,
    follower: FollowersCache,
}

const WORKER_COUNT: usize = 10; // Number of concurrent workers
const CHANNEL_CAPACITY: usize = 1000; // Max pending orders in queue

pub async fn start(mut rx: broadcast::Receiver<FullOrder>, pool: PgPool, agentkey: &str) {
    // let wallet = agentkey.parse().unwrap();

    let cache: Arc<Mutex<HashMap<String, Vec<FollowersCache>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    preload_followers(&pool, &cache).await;

    let pool_clone = pool.clone();
    let cache_clone = cache.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            preload_followers(&pool_clone, &cache_clone).await;
        }
    });

    let (tx, rx_orders) = broadcast::channel::<OrderTask>(CHANNEL_CAPACITY);

    // Spawn worker pool
    for worker_id in 0..WORKER_COUNT {
        let rx_orders = rx_orders.resubscribe();
        let agentkey = agentkey.to_string();

        tokio::spawn(async move {
            order_worker(worker_id, rx_orders, &agentkey).await;
        });
    }
    drop(rx_orders);

    while let Ok(order) = rx.recv().await {
        let cache_lock = cache.lock().unwrap();
        let followers = match cache_lock.get(&order.user) {
            Some(f) => f.clone(),
            None => continue,
        };
        drop(cache_lock);

        for follower in followers {
            let task = OrderTask {
                order: order.clone(),
                follower,
            };

            match tx.send(task) {
                Ok(_) => {}
                Err(broadcast::error::SendError(_)) => {
                    log::warn!("Order queue is full, dropping order (backpressure)");
                }
            }
        }
    }

    todo!()
}

async fn order_worker(worker_id: usize, mut rx: broadcast::Receiver<OrderTask>, agentkey: &str) {
    // Each worker has its own exchange client
    let wallet = agentkey.parse().unwrap();
    let exchange_client =
        match ExchangeClient::new(None, wallet, Some(BaseUrl::Testnet), None, None).await {
            Ok(client) => client,
            Err(e) => {
                log::error!(
                    "Worker {} failed to create exchange client: {}",
                    worker_id,
                    e
                );
                return;
            }
        };

    info!("Worker {} started", worker_id);

    while let Some(task) = rx.recv().await.ok() {
        let result = handle_follower_order(&exchange_client, &task.order, &task.follower).await;

        match result {
            Ok(oid) => {
                info!(
                    "Worker {}: Order executed for {} - OID: {}",
                    worker_id, task.follower.address, oid
                );
            }
            Err(e) => {
                log::error!(
                    "Worker {}: Failed to execute order for {}: {}",
                    worker_id,
                    task.follower.address,
                    e
                );
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
    // Calculate size with risk management
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
                _ => Err(format!("Unexpected order status: {:?}", data.statuses[0])),
            }
        }
        hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
            Err(format!("Exchange error: {}", e))
        }
    }
}

async fn preload_followers(
    pool: &PgPool,
    cache: &Arc<Mutex<HashMap<String, Vec<FollowersCache>>>>,
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

    let mut cache_lock = cache.lock().unwrap();
    cache_lock.clear();

    for row in rows {
        let trader: String = row.get("trader_address");
        let follower = FollowersCache {
            address: row.get("address"),
            signature: row.get("agent_signature"),
            ratio: row.get("ratio"),
            max_risk: row.get("max_risk_per_trade"),
        };
        cache_lock.entry(trader).or_default().push(follower);
    }
}
