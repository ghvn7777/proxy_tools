use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::{client::proxy_tunnels, get_crypt, ClientConfig, TcpTunnel};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(ClientConfig::parse());

    let layer = Layer::new().with_filter(LevelFilter::ERROR);
    tracing_subscriber::registry().with(layer).init();

    let crypt = get_crypt(&config.crypt_file)?;

    let cnt = config.tunnel_cnt.unwrap_or(10);
    let mut tunnels = Vec::with_capacity(cnt as _);
    for _ in 0..cnt {
        tunnels.push(TcpTunnel::generate(
            config.server_url.clone(),
            crypt.clone(),
        ));
    }

    proxy_tunnels(tunnels, config).await?;

    Ok(())
}
