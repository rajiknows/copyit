use crate::hyperliquid::ws::WsFill;


#[derive(Clone, Debug)]
pub struct WsFillChannel{
    pub fill:WsFill,
    pub user: String,
}
