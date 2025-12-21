use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    error::AppError,
    models::LeaderboardEntry,
    api::Server,
};

pub fn create_router() -> Router<Arc<Server>> {
    Router::new().route("/", get(get_leaderboard))
}

async fn get_leaderboard(
    State(state): State<Arc<Server>>,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let pool = state.pool.as_ref().ok_or(AppError::InternalServerError)?;

    let leaderboard =
        sqlx::query_as::<_, LeaderboardEntry>("SELECT * FROM leaderboard ORDER BY pnl_percent_30d DESC")
            .fetch_all(pool)
            .await?;

    Ok(Json(leaderboard))
}
