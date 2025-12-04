use axum::{Router, routing::get};

use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Server {
    pub port: u16,
    pub db_url: String,
}

impl Server {
    pub fn new(port: u16, db_url: String) -> Self {
        Self { port, db_url }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        println!("Starting API server on port {}", self.port);

        // TODO: Use db_url here to connect to database pool
        let app = self.router();

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

        Ok(())
    }

    fn router(&self) -> Router {
        Router::new()
            .route("/", get(|| async { "Hyperliquid Copy Trading Engine API" }))
            .route("/health", get(|| async { "OK" }))
    }
}
