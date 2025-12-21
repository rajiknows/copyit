use axum::{
    extract::{Path, State},
    routing::put,
    Json, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    error::AppError,
    models::CopyConfig,
    api::Server,
};

pub fn create_router() -> Router<Arc<Server>> {
    Router::new().route("/:id", put(update_copy_config))
}

#[derive(Debug, Deserialize)]
struct UpdateCopyConfig {
    ratio: Option<Decimal>,
    is_active: Option<bool>,
    max_risk_per_trade: Option<Decimal>,
}

async fn update_copy_config(
    State(state): State<Arc<Server>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateCopyConfig>,
) -> Result<Json<CopyConfig>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let mut set_clauses = Vec::new();
    if payload.ratio.is_some() {
        set_clauses.push("ratio = $2");
    }
    if payload.is_active.is_some() {
        set_clauses.push("is_active = $3");
    }
    if payload.max_risk_per_trade.is_some() {
        set_clauses.push("max_risk_per_trade = $4");
    }

    if set_clauses.is_empty() {
        // Or return the current config without changes
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let query_str = format!(
        "UPDATE copy_configs SET {} WHERE id = $1 RETURNING *",
        set_clauses.join(", ")
    );

    let mut query = sqlx::query_as::<_, CopyConfig>(&query_str).bind(id);

    if let Some(ratio) = payload.ratio {
        query = query.bind(ratio);
    }
    if let Some(is_active) = payload.is_active {
        query = query.bind(is_active);
    }
    if let Some(max_risk) = payload.max_risk_per_trade {
        query = query.bind(max_risk);
    }

    let updated_config = query.fetch_one(pool).await?;

    Ok(Json(updated_config))
}
