use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::server::run_tcp_server;

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::TRACE);
    tracing_subscriber::registry().with(layer).init();

    let addr = format!("0.0.0.0:{}", 9527);
    let listener = TcpListener::bind(addr).await?;
    info!("Vpn server listening on {}", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Vpn server {:?} connected", addr);
        tokio::spawn(async move {
            run_tcp_server(stream).await.expect("run tcp server error");
            info!("Vpn server {:?} disconnected", addr);
        });
    }
}
