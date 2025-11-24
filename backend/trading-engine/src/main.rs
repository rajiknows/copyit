// this is the main entrypoint of our trading engine
//
// we will have these modules or should we say services
// 1. Detector ( constantly detectes trades from every batch of txns)
// 2. Calculator ( calculates position of the follower )
// 3. Executor ( executes a tx and sends it to the blockhain )
// 4. Monitor ( position monitoring module )
// 5. Hyperliquid ( hyperliquid ws connection , all apis)

fn main() {
    println!("Hello, world!");
}
