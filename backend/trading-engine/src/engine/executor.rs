use tokio::sync::broadcast;

use crate::engine::grouper::FullOrder;


pub async fn start(rx: broadcast::Receiver<FullOrder>) {
    // fetch the followers of this trader ,
    // for each follower make a trade
    todo!()
}
