use hyperliquid_rust_sdk::Trade;
use tokio::sync::broadcast;

use crate::{FullOrder, hyperliquid::ws::WsFill};

pub async fn start(rx: broadcast::Receiver<WsFill>, sender: broadcast::Sender<FullOrder>) {
    todo!()
}
