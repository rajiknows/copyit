use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    error::AppError,
    models::Trader,
    api::Server,
};

use serde::Deserialize;


pub fn create_router() -> Router<Arc<Server>> {
    Router::new()
        .route("/", get(get_traders).post(register_trader))
        .route("/:address", get(get_trader).delete(delete_trader))
}

async fn get_traders(
    State(state): State<Arc<Server>>,
) -> Result<Json<Vec<Trader>>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let traders =
        sqlx::query_as::<_, Trader>("SELECT address, name, is_active, added_at FROM traders")
            .fetch_all(pool)
            .await?;

    Ok(Json(traders))
}

async fn get_trader(
    State(state): State<Arc<Server>>,
    Path(address): Path<String>,
) -> Result<Json<Trader>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let trader = sqlx::query_as::<_, Trader>(
        "SELECT address, name, is_active, added_at FROM traders WHERE address = $1",
    )
    .bind(address)
    .fetch_one(pool)
    .await?;

    Ok(Json(trader))
}

async fn delete_trader(
    State(state): State<Arc<Server>>,
    Path(address): Path<String>,
) -> Result<Json<Trader>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let trader = sqlx::query_as::<_, Trader>(
        "UPDATE traders SET is_active = false WHERE address = $1 RETURNING *",
    )
    .bind(address)
    .fetch_one(pool)
    .await?;

    Ok(Json(trader))
}

#[derive(Debug, Deserialize)]
struct RegisterTrader {
    address: String,
    name: Option<String>,
}

async fn register_trader(
    State(state): State<Arc<Server>>,
    Json(payload): Json<RegisterTrader>,
) -> Result<Json<Trader>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let trader =
        sqlx::query_as::<_, Trader>("INSERT INTO traders (address, name) VALUES ($1, $2) RETURNING *")
            .bind(payload.address)
            .bind(payload.name)
            .fetch_one(pool)
            .await?;

    Ok(Json(trader))
}
