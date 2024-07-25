use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use tokio::time::sleep;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::ServerQuicConn;
use vpn::{get_crypt, server::run_server, util::server_config::ServerConfig};

use vpn::util::make_server_endpoint;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(ServerConfig::parse());

    let layer = Layer::new().with_filter(LevelFilter::ERROR);
    tracing_subscriber::registry().with(layer).init();

    let addr = format!("0.0.0.0:{}", config.port);
    let (endpoint, _server_cert) = make_server_endpoint(addr.parse()?).unwrap();
    info!("[server] listening on {}", addr);

    let crypt = get_crypt(&config.crypt_file)?;

    loop {
        let crypt_clone = crypt.clone();
        let Some(incoming_conn) = endpoint.accept().await else {
            error!("Error accepting connection");
            sleep(Duration::from_millis(6000)).await;
            continue;
        };
        let conn = match incoming_conn.await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Error incoming conn: {:?}", e);
                continue;
            }
        };

        let remote_address = conn.remote_address();
        info!("[server] connection accepted: addr={}", remote_address);

        tokio::spawn(async move {
            match run_server(ServerQuicConn::new(conn), crypt_clone).await {
                Ok(_) => {
                    info!("[server] {:?} end", remote_address);
                }
                Err(e) => {
                    info!("[server] {:?} error: {:?}", remote_address, e);
                }
            }
            info!("[server] {:?} disconnected", remote_address);
        });
    }
}
