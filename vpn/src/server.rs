use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
// use vpn::{ProstServerStream, TlsServerAcceptor, YamuxCtrl};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::TRACE);
    tracing_subscriber::registry().with(layer).init();

    let addr = format!("0.0.0.0:{}", 9527);
    let listener = TcpListener::bind(addr).await?;
    info!("Vpn server listening on {}", listener.local_addr()?);

    loop {
        let (_stream, addr) = listener.accept().await?;
        info!("Vpn server {:?} connected", addr);
        tokio::spawn(async move {
            // let stream = ProstServerStream::new(stream.compat(), streams_clone);
            // stream.process().await.unwrap();
            // Ok(())
        });
    }
}
