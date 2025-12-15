// in this module we will have a long list of traders in memory we will have thier public adddress
// we will then consume the channel that gives us the trades if we identify a trade by the traders
// we send a trade message to the redis queue for further processing

pub mod executor;
pub mod grouper;
// pub mod monitor;
pub mod parser;
