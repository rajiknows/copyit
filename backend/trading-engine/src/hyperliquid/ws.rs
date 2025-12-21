use std::{ thread::sleep, time::Duration};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use serde::Serialize;
use thiserror::Error;

use crate::channel::WsFillChannel;

#[derive(Error, Debug)]
pub enum WsError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Failed to serialize subscription request: {0}")]
    SerializationFailed(#[from] serde_json::Error),
    #[error("WebSocket message processing error: {0}")]
    MessageProcessing(tokio_tungstenite::tungstenite::Error),
    #[error("WebSocket closed for user {0}")]
    WebSocketClosed(String),
    #[error("WebSocket stream ended unexpectedly for {0}")]
    StreamEnded(String),
}


const WS_MAINNET: &str = "wss://api.hyperliquid.xyz/ws";

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionRequest {
    method: String,
    subscription: Subscription,
}

#[derive(Debug, Serialize, Deserialize)]
struct Subscription {
    #[serde(rename = "type")]
    type_: String,
    user: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "channel")]
enum Incoming {
    #[serde(rename = "userFills")]
    UserFills(UserFillsResponse),
    #[serde(rename = "subscriptionResponse")]
    SubscriptionResponse(SubscriptionResponse),
    // Add other channels if needed
}

#[derive(Debug, Deserialize, Serialize)]
struct UserFillsResponse {
    data: WsUserFills,
}

#[derive(Debug, Deserialize, Serialize)]
struct WsUserFills {
    #[serde(default)]
    isSnapshot: bool,
    user: String,
    fills: Vec<WsFill>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WsFill {
    pub coin: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub time: u64,
    pub hash: String,
    pub oid: u64,
    pub startPosition: Option<String>,
    pub closedPnl: Option<String>,
    pub dir: Option<String>,
    pub crossed: bool,
    pub fee: String,
    pub feeToken: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SubscriptionResponse {
    data: serde_json::Value,
}

pub async fn fetch_fills(
    trader_addr: String,
    channel_tx: tokio::sync::broadcast::Sender<WsFillChannel>,
) -> Result<(), WsError> {
    let url = WS_MAINNET.into_client_request().unwrap();
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

    let sub = SubscriptionRequest {
        method: "subscribe".to_string(),
        subscription: Subscription {
            type_: "userFills".to_string(),
            user: trader_addr.clone(),
        },
    };

    let sub_msg = Message::text(serde_json::to_string(&sub)?);
    ws_stream.send(sub_msg).await?;

    while let Some(msg) = ws_stream.next().await {
        let msg = msg.map_err(WsError::MessageProcessing)?;
        if let Message::Text(text) = msg {
            if text.contains("subscriptionResponse") {
                let response: Incoming = serde_json::from_str(&text)?;
                if let Incoming::SubscriptionResponse(_) = response {
                    println!("Successfully subscribed to userFills for {trader_addr}");
                    break;
                }
            }
        }
    }

    // Main event loop
    while let Some(result) = ws_stream.next().await {
        match result {
            Ok(Message::Text(text)) => {
                if text.contains("userFills") {
                    match serde_json::from_str::<Incoming>(&text) {
                        Ok(Incoming::UserFills(resp)) => {
                            for fill in resp.data.fills {
                                // Ignore snapshot fills if you already have historical state
                                if resp.data.isSnapshot {
                                    continue; // or handle snapshot once at startup
                                }

                                let channelfill = WsFillChannel{
                                    fill: fill.clone(),
                                    user : resp.data.user.clone(),
                                };

                                let _ = channel_tx.send(channelfill);
                                // Optional: log or emit metrics
                                println!(
                                    "[{}] {} {} @ {} | Dir: {:?}",
                                    trader_addr, fill.side, fill.sz, fill.px, fill.dir
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to parse userFills: {e}\nText: {text}");
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = ws_stream.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(_)) | Err(_) => {
                return Err(WsError::WebSocketClosed(trader_addr));
            }
            _ => {}
        }
    }

    Err(WsError::StreamEnded(trader_addr))
}

pub async fn fetch_fills_with_retry(
    user_addr: String,
    channel_tx: tokio::sync::broadcast::Sender<WsFillChannel>,
) -> ! {
    loop {
        match fetch_fills(user_addr.clone(), channel_tx.clone()).await {
            Ok(_) => println!("fetch_fills exited cleanly (should not happen)"),
            Err(e) => {
                eprintln!("Lost connection for {user_addr}: {e}");
            }
        }

        // Exponential backoff or fixed delay
        sleep(Duration::from_secs(3));
        println!("Reconnecting to userFills for {user_addr}...");
    }
}
