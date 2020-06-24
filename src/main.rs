use beaver;
use beaver::config::Config;
use beaver::server::Server;
use beaver::global_static_config::CONFIG;
use env_logger;
use log::error;
use tokio;

#[tokio::main]
async fn main() {
    env_logger::init();

    match Server::new(&CONFIG) {
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
