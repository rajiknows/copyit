#[derive(Clone, Debug)]
pub struct Server {
    port: u16,
    db_url: String,
}

impl Server {
    pub fn new(port: u16, db_url: String) -> Self {
        Self { port, db_url }
    }

    pub fn start(&self) -> anyhow::Result<()> {
        todo!()
    }
}
