use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::{TradeMessage, WsTrade};

const WS_MAINNET: &str = "wss://api.hyperliquid.xyz/ws";

#[derive(Debug, Deserialize)]
struct Incoming {
    channel: String,
    data: Vec<WsTrade>,
}

pub async fn fetch_trades(
    coin: &str,
    channel_tx: broadcast::Sender<TradeMessage>,
) -> anyhow::Result<()> {
    let req = WS_MAINNET.into_client_request().unwrap();
    let (mut socket, _) = connect_async(req).await?;

    let msg = Message::text(format!(
        r#"{{
            "method": "subscribe",
            "subscription": {{
                "type": "trades",
                "coin": "{coin}"
            }}
        }}"#
    ));

    socket.send(msg).await?;

    loop {
        let resp = socket
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("no response"))??;

        if let Message::Text(text) = resp {
            if let Ok(parsed) = serde_json::from_str::<Incoming>(&text) {
                if parsed.channel != "error" {
                    let out = TradeMessage {
                        channel: parsed.channel,
                        data: parsed.data,
                    };

                    let _ = channel_tx.send(out);
                }
            }
        }
    }
}
