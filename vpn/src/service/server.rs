use std::sync::Arc;

use anyhow::Result;
use futures::join;
use tracing::info;

use crate::{
    util::channel_bus, DataCrypt, Processor, ServerMsg, ServerReadProcessor, ServerWriteProcessor,
    StreamSplit,
};

pub async fn run_server(conn: impl StreamSplit, crypt: Arc<Box<dyn DataCrypt>>) -> Result<()> {
    let (main_sender, sub_senders, receivers) = channel_bus::<ServerMsg>(10, 1000);
    let (reader, writer) = conn.stream_split().await;
    let mut reader_process = ServerReadProcessor::new(reader, crypt.clone(), main_sender);
    let mut write_process =
        ServerWriteProcessor::new(writer, crypt.clone(), sub_senders, receivers);

    let _r = tokio::spawn(async move {
        match reader_process.process().await {
            Ok(_) => {
                info!("run_tcp_server reader end");
            }
            Err(e) => {
                info!("run_tcp_server reader error: {:?}", e);
            }
        }
    });

    let w = async {
        match write_process.process().await {
            Ok(_) => {
                info!("run_tcp_server writer end");
            }
            Err(e) => {
                info!("run_tcp_server writer error: {:?}", e);
            }
        }
    };

    join!(w);

    // 与客户端心跳超时也会导致服务端断开连接，客户端应该要循环重连
    // tokio::select! {
    //     _ = r => {
    //         info!("run_tcp_server reader end");
    //     }
    //     _ = w => {
    //         info!("run_tcp_server writer end");
    //     }
    // }

    Ok(())
}
