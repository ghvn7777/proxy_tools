use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::{get_crypt, socks5::proxy_socks, ClientConfig, S2nTunnel};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(ClientConfig::parse());

    let layer = Layer::new().with_filter(LevelFilter::DEBUG);
    tracing_subscriber::registry().with(layer).init();

    let crypt = get_crypt(&config.crypt_file)?;

    let cnt = config.tunnel_cnt.unwrap_or(10);
    let mut tunnels = Vec::with_capacity(cnt as _);
    for _ in 0..cnt {
        tunnels.push(S2nTunnel::generate(
            config.server_url.clone(),
            crypt.clone(),
        ));
    }

    proxy_socks(tunnels, config).await?;

    Ok(())
}
