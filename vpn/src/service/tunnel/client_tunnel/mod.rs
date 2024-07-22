mod quic_tunnel;
mod tcp_tunnel;

use std::sync::Arc;

use crate::{
    util::SubSenders, ClientMsg, ClientReadProcessor, ClientWriteProcessor, DataCrypt, Processor,
    Socks5ToClientMsg, SocksMsg, StreamSplit, TunnelReader, TunnelWriter, VpnError,
};

use futures::{
    channel::mpsc::{channel, Sender},
    SinkExt, Stream,
};
pub use quic_tunnel::*;
pub use tcp_tunnel::*;
use tracing::{error, info};

pub struct Tunnel {
    connect_id: u32,
    senders: SubSenders<ClientMsg>,
    main_sender: Sender<ClientMsg>,
}

impl Tunnel {
    pub async fn generate(
        &mut self,
    ) -> Result<(TunnelWriter<ClientMsg>, TunnelReader<ClientMsg, SocksMsg>), VpnError> {
        let connect_id = self.connect_id;
        self.connect_id += 1;

        let (tx, rx) = channel(1000);

        // 这里 main sender 如果发不出去，说明代码逻辑有问题，直接 panic
        self.main_sender
            .send(Socks5ToClientMsg::InitChannel(connect_id, tx).into())
            .await
            .expect("Send init channel failed");

        let sender = self.senders.get_one_sender();

        Ok((
            TunnelWriter {
                id: connect_id,
                tx: sender.clone(),
            },
            TunnelReader {
                id: connect_id,
                tx: sender.clone(),
                rx: Some(rx),
            },
        ))
    }
}

pub async fn tunnel_client_core_task<S>(
    stream: impl StreamSplit,
    crypt: Arc<Box<dyn DataCrypt>>,
    msg_stream: &mut S,
    main_sender_tx: Sender<ClientMsg>,
) -> Result<(), VpnError>
where
    S: Stream<Item = ClientMsg> + Unpin + Send + Sync + 'static,
{
    let (reader, writer) = stream.stream_split().await;
    let mut rader_processor = ClientReadProcessor::new(reader, crypt.clone(), main_sender_tx);
    let mut writer_processor = ClientWriteProcessor::new(writer, crypt.clone(), msg_stream);

    let r = async {
        match rader_processor.process().await {
            Ok(_) => info!("Tcp tunnel core task read stream finished"),
            Err(e) => error!("Tcp tunnel core task read stream error: {:?}", e),
        }
    };

    let w = async {
        match writer_processor.process().await {
            Ok(_) => info!("Tcp tunnel core task write stream finished"),
            Err(e) => error!("Tcp tunnel core task write stream error: {:?}", e),
        }
    };

    // join!(r, w);
    tokio::select! {
        _ = r => {
            info!("Tcp tunnel core task read stream end");
        }
        _ = w => {
            info!("Tcp tunnel core task write stream end");
        }
    };

    Ok(())
}
