use axum::{Router, extract::State, http::StatusCode, routing::get, Json};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Server {
    pub port: u16,
    pub db_url: String,
    pub pool: Option<PgPool>,
}

impl Server {
    pub fn new(port: u16, db_url: String) -> Self {
        Self {
            port,
            db_url,
            pool: None,
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        println!("Starting API server on port {}", self.port);

        // Create database pool
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.db_url)
            .await?;

        let mut server = self.clone();
        server.pool = Some(pool);

        let app = server.router();
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
        Ok(())
    }

    fn router(&self) -> Router {
        let state = Arc::new(self.clone());
        Router::new()
            .route("/", get(|| async { "Hyperliquid Copy Trading Engine API" }))
            .route("/health", get(|| async { "OK" }))
            .route("/traders", get(get_traders))
            .with_state(state)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Trader {
    name: String,
    addr: String,
}

async fn get_traders(
    State(state): State<Arc<Server>>,
) -> Result<Json<Vec<Trader>>, StatusCode> {
    let pool = state
        .pool
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT address, name FROM traders",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let traders: Vec<Trader> = rows
        .into_iter()
        .map(|(addr, name)| Trader { name, addr })
        .collect();

    Ok(Json(traders))
}
