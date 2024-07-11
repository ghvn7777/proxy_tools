use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use tokio::net::TcpListener;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use vpn::{ProstServerStream, TlsServerAcceptor, YamuxCtrl};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::TRACE);
    tracing_subscriber::registry().with(layer).init();

    let streams = Arc::new(DashMap::new());

    let addr = format!("0.0.0.0:{}", 9527);

    let server_cert = include_str!("../fixtures/server.cert");
    let server_key = include_str!("../fixtures/server.key");
    let acceptor = TlsServerAcceptor::new(server_cert, server_key, None)?;

    let listener = TcpListener::bind(addr).await?;
    info!("Vpn server listening on {}", listener.local_addr()?);

    loop {
        let tls = acceptor.clone();
        let (stream, addr) = listener.accept().await?;
        info!("Vpn client {:?} connected", addr);
        let streams1 = streams.clone();
        tokio::spawn(async move {
            let stream = tls.accept(stream).await.unwrap();
            YamuxCtrl::new_server(stream, None, move |stream| {
                let streams_clone = streams1.clone();
                async move {
                    let stream = ProstServerStream::new(stream.compat(), streams_clone);
                    stream.process().await.unwrap();
                    Ok(())
                }
            });
        });
    }
}
