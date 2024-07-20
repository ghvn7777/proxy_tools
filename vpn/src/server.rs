use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::{get_crypt, server::run_tcp_server, util::server_config::ServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(ServerConfig::parse());

    let layer = Layer::new().with_filter(LevelFilter::ERROR);
    tracing_subscriber::registry().with(layer).init();

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(addr).await?;
    info!("Vpn server listening on {}", listener.local_addr()?);

    let crypt = get_crypt(&config.crypt_file)?;

    loop {
        let crypt_clone = crypt.clone();
        let (stream, addr) = listener.accept().await?;
        info!("Vpn server {:?} connected", addr);
        tokio::spawn(async move {
            match run_tcp_server(stream, crypt_clone).await {
                Ok(_) => {
                    info!("Vpn server {:?} end", addr);
                }
                Err(e) => {
                    info!("Vpn server {:?} error: {:?}", addr, e);
                }
            }
            info!("Vpn server {:?} disconnected", addr);
        });
    }
}
