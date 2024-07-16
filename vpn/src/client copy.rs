use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::{Socks5Config, Socks5ServerStream};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(Socks5Config::parse());

    let layer = Layer::new().with_filter(LevelFilter::DEBUG);
    tracing_subscriber::registry().with(layer).init();

    let socket_addr = format!("0.0.0.0:{}", config.port);

    let listener = TcpListener::bind(socket_addr).await?;
    info!("Socks5 server listening on {}", listener.local_addr()?);

    loop {
        let socks5_config = config.clone();
        let (stream, addr) = listener.accept().await?;
        info!("Socks5 client {:?} connected", addr);
        tokio::spawn(async move {
            let stream = Socks5ServerStream::new(stream, socks5_config);
            stream.process().await.unwrap();
        });
    }
}
