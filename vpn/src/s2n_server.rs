use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use s2n_quic::Server;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::{get_crypt, server::run_server, util::server_config::ServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(ServerConfig::parse());

    let layer = Layer::new().with_filter(LevelFilter::TRACE);
    tracing_subscriber::registry().with(layer).init();

    let addr = format!("0.0.0.0:{}", config.port);

    let mut server = Server::builder()
        .with_tls((
            Path::new("vpn/fixtures/cert.pem"),
            Path::new("vpn/fixtures/key.pem"),
        ))?
        .with_io(addr.as_str())?
        .start()?;
    info!("[server] listening on {}", addr);

    let crypt = get_crypt(&config.crypt_file)?;

    while let Some(mut connection) = server.accept().await {
        // spawn a new task for the connection
        let crypt_clone = crypt.clone();
        tokio::spawn(async move {
            while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
                let crypt_clone2 = crypt_clone.clone();
                // spawn a new task for the stream
                tokio::spawn(async move {
                    match run_server(stream, crypt_clone2).await {
                        Ok(_) => {
                            info!("[server] end");
                        }
                        Err(e) => {
                            info!("[server] error: {:?}", e);
                        }
                    }
                    info!("[server] disconnected");
                });
            }
        });
    }

    Ok(())
}
