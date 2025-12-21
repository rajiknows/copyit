use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Trader {
    pub address: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub added_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Follower {
    pub id: i32,
    pub address: String,
    pub agent_signature: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
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

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Trade {
    pub id: i64,
    pub follower_address: String,
    pub trader_address: String,
    pub coin: String,
    pub side: String,
    pub size: Decimal,
    pub price: Decimal,
    pub order_hash: Option<String>,
    pub hl_oid: Option<i64>,
    pub timestamp: Option<NaiveDateTime>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct LeaderboardEntry {
    pub trader_address: String,
    pub pnl_percent_30d: Option<Decimal>,
    pub win_rate: Option<Decimal>,
    pub sharpe: Option<Decimal>,
    pub max_drawdown: Option<Decimal>,
    pub followers_count: Option<i32>,
    pub volume_7d: Option<Decimal>,
    pub updated_at: Option<NaiveDateTime>,
}
