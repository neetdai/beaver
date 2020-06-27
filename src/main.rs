use beaver;
use beaver::config::Config;
use beaver::global_static::CONFIG;
use beaver::server::Server;
use env_logger;
use log::error;
use tokio;

#[tokio::main]
async fn main() {
    env_logger::init();

    match Server::new() {
        Ok(server) => {
            if let Err(e) = server.run().await {
                error!("{:?}", e);
            }
        }
        Err(e) => {
            error!("{:?}", e);
        }
    }
}
