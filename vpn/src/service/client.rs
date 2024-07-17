use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::info;

use crate::{socks5::Socks5ServerStream, ClientConfig, Tunnel, VpnError};

pub async fn proxy_tunnels(
    mut tunnels: Vec<Tunnel>,
    config: Arc<ClientConfig>,
) -> Result<(), VpnError> {
    let socket_addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(socket_addr).await?;
    info!("Socks5 server listening on {}", listener.local_addr()?);

    let mut index = 0;

    loop {
        let socks5_config = config.clone();
        let (stream, addr) = listener.accept().await?;
        info!("Socks5 client {:?} connected", addr);

        // 这里如果获取不到 tunnel 说明代码逻辑有问题，直接 panic
        let tunnel: &mut Tunnel = tunnels.get_mut(index).expect("Get tunnel failed");
        let (write_port, read_port) = tunnel.generate().await?;
        tokio::spawn(async move {
            let stream = Socks5ServerStream::new(stream, socks5_config);
            match stream.process(write_port, read_port).await {
                Ok(_) => info!("Socks5 client {:?} end", addr),
                Err(e) => info!("Socks5 client {:?} error: {:?}", addr, e),
            };

            info!("Socks5 client {:?} disconnected", addr);
        });

        index = (index + 1) % tunnels.len();
    }
}
