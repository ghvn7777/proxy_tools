use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpStream;
use tracing::info;

use crate::{util::channel_bus, DataCrypt, ServerMsg, TcpServerStreamGenerator};

pub async fn run_tcp_server(stream: TcpStream, crypt: Arc<Box<dyn DataCrypt>>) -> Result<()> {
    let (main_sender, sub_senders, receivers) = channel_bus::<ServerMsg>(10, 1000);

    let (mut reader, mut writer) = TcpServerStreamGenerator::generate(stream, crypt);

    let r = async move {
        match reader.process(main_sender).await {
            Ok(_) => {
                info!("run_tcp_server reader end");
            }
            Err(e) => {
                info!("run_tcp_server reader error: {:?}", e);
            }
        }
    };

    let w = async {
        match writer.process(sub_senders, receivers).await {
            Ok(_) => {
                info!("run_tcp_server writer end");
            }
            Err(e) => {
                info!("run_tcp_server writer error: {:?}", e);
            }
        }
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
