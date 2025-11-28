use std::{fmt::Formatter, thread::sleep, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use url::form_urlencoded::parse;

use crate::{TradeMessage, WsTrade};

const WS_MAINNET: &str = "wss://api.hyperliquid.xyz/ws";

// #[derive(Debug, Deserialize)]
// struct Incoming {
//     channel: String,
//     data: Vec<WsTrade>,
// }


use anyhow::anyhow;
// use futures_util::{SinkExt, StreamExt};
use serde::{ Serialize};
use tokio::net::TcpStream;
// use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

// const WS_MAINNET: &str = "wss://api.hyperliquid.xyz/ws";

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
    user_addr: String,
    channel_tx: tokio::sync::broadcast::Sender<WsFill>,
) -> anyhow::Result<()> {
    let url = WS_MAINNET.into_client_request().unwrap();
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

    // Correct subscription format
    let sub = SubscriptionRequest {
        method: "subscribe".to_string(),
        subscription: Subscription {
            type_: "userFills".to_string(),
            user: user_addr.clone(),
        },
    };

    let sub_msg = Message::text(serde_json::to_string(&sub)?);
    ws_stream.send(sub_msg).await?;

    // Optional: wait for subscription ack
    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if text.contains("subscriptionResponse") {
                let response: Incoming = serde_json::from_str(&text)?;
                if let Incoming::SubscriptionResponse(_) = response {
                    println!("Successfully subscribed to userFills for {user_addr}");
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

                                let _ = channel_tx.send(fill.clone());
                                // Optional: log or emit metrics
                                println!(
                                    "[{}] {} {} @ {} | Dir: {:?}",
                                    user_addr,
                                    fill.side,
                                    fill.sz,
                                    fill.px,
                                    fill.dir
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
                return Err(anyhow!("WebSocket closed for user {user_addr}"));
            }
            _ => {}
        }
    }

    Err(anyhow!("WebSocket stream ended unexpectedly for {user_addr}"))
}

pub async fn fetch_fills_with_retry(
    user_addr: String,
    channel_tx: tokio::sync::broadcast::Sender<WsFill>,
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
// pub async fn fetch_fills(user_addr: String, channel_tx: broadcast::Sender<TradeMessage>)-> anyhow::Result<()>{
//     let req = WS_MAINNET.into_client_request().unwrap();
//     let (mut socket,_) = connect_async(req).await?;

//     let msg = Message::text(format!(
//         r#"{{
//         "method":"userFills",
//         "user":"{user_addr}"
//         }}"#
//     ));
//     socket.send(msg).await?;
//     loop{
//         let resp = socket.next().await.ok_or_else(|| anyhow::anyhow!("no response"))??;
//         if let Message::Text(text) = resp{
//             if let Ok(parsed) = serde_json::from_str::<Incoming>(&text){
//                 println!("{}", parsed);

//             }
//         }
//     }
//     Ok(())
// }

// pub async fn fetch_trades(
//     coin: &str,
//     channel_tx: broadcast::Sender<TradeMessage>,
// ) -> anyhow::Result<()> {
//     let req = WS_MAINNET.into_client_request().unwrap();
//     let (mut socket, _) = connect_async(req).await?;

//     let msg = Message::text(format!(
//         r#"{{
//             "method": "subscribe",
//             "subscription": {{
//                 "type": "trades",
//                 "coin": "{coin}"
//             }}
//         }}"#
//     ));

//     socket.send(msg).await?;

//     loop {
//         let resp = socket
//             .next()
//             .await
//             .ok_or_else(|| anyhow::anyhow!("no response"))??;

//         if let Message::Text(text) = resp {
//             if let Ok(parsed) = serde_json::from_str::<Incoming>(&text) {
//                 if parsed.channel != "error" {
//                     let out = TradeMessage {
//                         channel: parsed.channel,
//                         data: parsed.data,
//                     };
//                     println!("{:?}", out);

//                     let _ = channel_tx.send(out);
//                 }
//             }
//         }
//     }
// }
