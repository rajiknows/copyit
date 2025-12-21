use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    error::AppError,
    models::Trade,
    api::Server,
};

pub fn create_router() -> Router<Arc<Server>> {
    Router::new()
        .route("/trader/:trader_address", get(get_trades_by_trader))
        .route("/follower/:follower_id", get(get_trades_by_follower))
}

async fn get_trades_by_trader(
    State(state): State<Arc<Server>>,
    Path(trader_address): Path<String>,
) -> Result<Json<Vec<Trade>>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let trades = sqlx::query_as::<_, Trade>(
        "SELECT * FROM executed_trades WHERE trader_address = $1 ORDER BY timestamp DESC",
    )
    .bind(trader_address)
    .fetch_all(pool)
    .await?;

    Ok(Json(trades))
}

async fn get_trades_by_follower(
    State(state): State<Arc<Server>>,
    Path(follower_id): Path<i32>,
) -> Result<Json<Vec<Trade>>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let follower_address =
        sqlx::query_scalar::<_, String>("SELECT address FROM followers WHERE id = $1")
            .bind(follower_id)
            .fetch_one(pool)
            .await?;

    let trades = sqlx::query_as::<_, Trade>(
        "SELECT * FROM executed_trades WHERE follower_address = $1 ORDER BY timestamp DESC",
    )
    .bind(follower_address)
    .fetch_all(pool)
    .await?;

    Ok(Json(trades))
}
