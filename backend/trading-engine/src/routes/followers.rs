use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;


use crate::{
    error::AppError,
    models::{CopyConfig, Follower},
    api::Server,
};


pub fn create_router() -> Router<Arc<Server>> {
    Router::new()
        .route("/", get(get_followers).post(register_follower))
        .route("/:id", get(get_follower).delete(delete_follower))
}


#[derive(Debug, Serialize, FromRow)]
struct FullFollower {
    id: i32,
    address: String,
    agent_signature: String,
    trader_address: String,
    ratio: Decimal,
    is_active: bool,
    max_risk_per_trade: Option<Decimal>,
}

async fn get_followers(
    State(state): State<Arc<Server>>,
) -> Result<Json<Vec<FullFollower>>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let followers = sqlx::query_as::<_, FullFollower>(
        "SELECT f.id, f.address, f.agent_signature, c.trader_address, c.ratio, c.is_active, c.max_risk_per_trade FROM followers f JOIN copy_configs c ON f.id = c.follower_id",
    )
    .fetch_all(pool)
    .await?;

    Ok(Json(followers))
}

async fn get_follower(
    State(state): State<Arc<Server>>,
    Path(id): Path<i32>,
) -> Result<Json<FullFollower>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let follower = sqlx::query_as::<_, FullFollower>(
        "SELECT f.id, f.address, f.agent_signature, c.trader_address, c.ratio, c.is_active, c.max_risk_per_trade FROM followers f JOIN copy_configs c ON f.id = c.follower_id WHERE f.id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(Json(follower))
}

async fn delete_follower(
    State(state): State<Arc<Server>>,
    Path(id): Path<i32>,
) -> Result<Json<Follower>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let follower =
        sqlx::query_as::<_, Follower>("DELETE FROM followers WHERE id = $1 RETURNING *")
            .bind(id)
            .fetch_one(pool)
            .await?;

    Ok(Json(follower))
}

#[derive(Debug, Serialize)]
struct FollowerDetails {
    follower: Follower,
    copy_config: CopyConfig,
}

#[derive(Debug, Deserialize)]
struct RegisterFollower {
    address: String,
    agent_signature: String,
    trader_address: String,
    ratio: Decimal,
    max_risk_per_trade: Option<Decimal>,
}


async fn register_follower(
    State(state): State<Arc<Server>>,
    Json(payload): Json<RegisterFollower>,
) -> Result<Json<FollowerDetails>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let follower: Follower = sqlx::query_as(
        "INSERT INTO followers (address, agent_signature) VALUES ($1, $2) RETURNING *",
    )
    .bind(&payload.address)
    .bind(&payload.agent_signature)
    .fetch_one(pool)
    .await?;

    let copy_config: CopyConfig = sqlx::query_as(
        "INSERT INTO copy_configs (follower_id, trader_address, ratio, max_risk_per_trade) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(follower.id)
    .bind(&payload.trader_address)
    .bind(payload.ratio)
    .bind(payload.max_risk_per_trade)
    .fetch_one(pool)
    .await?;

    Ok(Json(FollowerDetails {
        follower,
        copy_config,
    }))
}
