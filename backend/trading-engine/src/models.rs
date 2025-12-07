use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Trader {
    pub address: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Follower {
    pub id: i32,
    pub address: String,
    pub agent_signature: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct CopyConfig {
    pub id: i32,
    pub follower_id: i32,
    pub trader_address: String,
    pub ratio: Decimal,
    pub is_active: bool,
    pub max_risk_per_trade: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FullOrder {
    pub leader: String,
    pub coin: String,
    pub dir: String,
    pub total_sz: Decimal,
    pub avg_px: Decimal,
    pub timestamp: u64,
    pub hash: String,
    pub oid: u64,
}
