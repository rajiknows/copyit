use crate::routes;
use axum::{
    routing::{get},
    Router,
};
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
            .nest("/traders", routes::traders::create_router().with_state(state.clone()))
            .nest("/followers", routes::followers::create_router().with_state(state.clone()))
            .nest("/copy_configs", routes::copy_configs::create_router().with_state(state.clone()))
            .nest("/leaderboard", routes::leaderboard::create_router().with_state(state.clone()))
            .nest("/trades", routes::trades::create_router().with_state(state))
    }
}

