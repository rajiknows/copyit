use log::{info, error};
use sqlx::Row;
use std::{collections::HashMap, str::FromStr};
use hyperliquid_rust_sdk::{BaseUrl, ClientLimit, ClientOrder, ClientOrderRequest, ExchangeClient};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_decimal_macros::dec;
use sqlx::{PgPool, postgres::PgRow};
use tokio::sync::{broadcast, RwLock};
use std::sync::Arc;
use thiserror::Error;


use crate::engine::grouper::FullOrder;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Invalid agent key: {0}")]
    InvalidAgentKey(String),
    #[error("Exchange client initialization failed: {0}")]
    ClientInitialization(String),
    #[error("Order placement failed: {0}")]
    OrderPlacement(String),
    #[error("Order size too small")]
    OrderSizeTooSmall,
    #[error("No data in exchange response")]
    NoDataInResponse,
    #[error("Unexpected status from exchange: {0:?}")]
    UnexpectedStatus(hyperliquid_rust_sdk::ExchangeDataStatus),
    #[error("Database error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("Failed to convert decimal to f64")]
    DecimalConversion,
}


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

pub async fn start(mut rx: broadcast::Receiver<FullOrder>, pool: PgPool, agentkey: &str) -> Result<(), ExecutorError> {
    let cache: SharedCache = Arc::new(RwLock::new(HashMap::new()));

    // Initial preload
    {
        let mut write_lock = cache.write().await;
        preload_followers(&pool, &mut write_lock).await?;
    }

    // Background refresher task
    let cache_clone = cache.clone();
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            let mut new_cache = HashMap::new();
            if let Err(e) = preload_followers(&pool_clone, &mut new_cache).await {
                error!("Failed to refresh follower cache: {}", e);
            }

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
            if let Err(e) = order_worker(worker_id, rx_orders, &agentkey).await {
                error!("Worker {} failed: {}", worker_id, e);
            }
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
    Ok(())
}

async fn order_worker(worker_id: usize, mut rx: broadcast::Receiver<OrderTask>, agentkey: &str) -> Result<(), ExecutorError> {
    let wallet = agentkey.parse().unwrap();
    let exchange_client = ExchangeClient::new(None, wallet, Some(BaseUrl::Testnet), None, None)
        .await
        .map_err(|e| ExecutorError::ClientInitialization(e.to_string()))?;

    info!("Worker {} started", worker_id);

    while let Ok(task) = rx.recv().await {
        let result = handle_follower_order(&exchange_client, &task.order, &task.follower).await;

        match result {
            Ok(oid) => {
                info!("Worker {}: Executed order for {} — OID: {}", worker_id, task.follower.address, oid);
            }
            Err(e) => {
                error!("Worker {}: Failed for {} — {}", worker_id, task.follower.address, e);
            }
        }
    }

    info!("Worker {} shutting down", worker_id);
    Ok(())
}

async fn handle_follower_order(
    exchange_client: &ExchangeClient,
    order: &FullOrder,
    follower: &FollowersCache,
) -> Result<u64, ExecutorError> {
    let mut sz = order.total_sz * follower.ratio;

    if let Some(max_risk) = follower.max_risk {
        let notional = sz * order.avg_px;
        if notional > max_risk {
            sz = max_risk / order.avg_px.max(dec!(0.0000001));
        }
    }

    if sz <= dec!(0.000001) {
        return Err(ExecutorError::OrderSizeTooSmall);
    }

    let is_buy = matches!(order.dir.as_str(), "Open Long" | "Close Short");

    let client_order = ClientOrderRequest {
        asset: order.coin.clone(),
        is_buy,
        reduce_only: false,
        limit_px: order.avg_px.to_f64().ok_or(ExecutorError::DecimalConversion)?,
        sz: sz.round_dp(8).to_f64().ok_or(ExecutorError::DecimalConversion)?,
        cloid: None,
        order_type: ClientOrder::Limit(ClientLimit {
            tif: "Gtc".to_string(),
        }),
    };

    let response = exchange_client
        .order(client_order, None)
        .await
        .map_err(|e| ExecutorError::OrderPlacement(e.to_string()))?;

    match response {
        hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(exchange_response) => {
            let data = exchange_response.data.ok_or(ExecutorError::NoDataInResponse)?;
            match &data.statuses[0] {
                hyperliquid_rust_sdk::ExchangeDataStatus::Filled(o) => Ok(o.oid),
                hyperliquid_rust_sdk::ExchangeDataStatus::Resting(o) => Ok(o.oid),
                status => Err(ExecutorError::UnexpectedStatus(status.clone())),
            }
        }
        hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
            Err(ExecutorError::OrderPlacement(e.to_string()))
        }
    }
}

async fn preload_followers(
    pool: &PgPool,
    cache: &mut HashMap<String, Vec<FollowersCache>>,
) -> Result<(), ExecutorError> {
    let rows: Vec<PgRow> = sqlx::query(
        "SELECT f.address, f.agent_signature, c.trader_address, c.ratio, c.max_risk_per_trade
         FROM copy_configs c
         JOIN followers f ON c.follower_id = f.id
         WHERE c.is_active = true",
    )
    .fetch_all(pool)
    .await?;

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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preload() {}
}
