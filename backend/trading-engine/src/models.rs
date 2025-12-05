use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Trader{
    tid: u64,
    name: String,
}
