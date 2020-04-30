use beaver;
use beaver::config::Config;
use beaver::server::Server;
use env_logger;
use log::error;
use tokio;

#[tokio::main]
async fn main() {
    env_logger::init();

    match Config::new("Config.toml") {
        Ok(config) => match Server::new(&config) {
            Ok(server) => {
                if let Err(e) = server.run().await {
                    error!("{:?}", e);
                }
            }
            Err(e) => {
                error!("{:?}", e);
            }
        },
        Err(e) => {
            error!("{:?}", e);
        }
    }
}
