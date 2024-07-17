use anyhow::Result;
use tokio::net::TcpStream;
use tracing::info;

use crate::{util::channel_bus, ServerMsg, VpnServerStreamGenerator};

pub async fn run_tcp_server(stream: TcpStream) -> Result<()> {
    let (main_sender, sub_senders, receivers) = channel_bus::<ServerMsg>(10, 1000);

    let (mut reader, mut writer) = VpnServerStreamGenerator::generate(stream);

    let r = async move {
        reader
            .process(main_sender)
            .await
            .expect("run_tcp_server reader failed");
    };

    let w = async {
        let _ = writer.process(sub_senders, receivers).await;
    };

    // join!(r, w);

    // 与客户端心跳超时也会导致服务端断开连接，客户端应该要循环重连
    tokio::select! {
        _ = r => {
            info!("run_tcp_server reader end");
        }
        _ = w => {
            info!("run_tcp_server writer end");
        }
    }

    Ok(())
}
