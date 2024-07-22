use std::sync::Arc;

use crate::{
    ClientMsg, ClientReadProcessor, ClientWriteProcessor, DataCrypt, Processor, StreamSplit,
    VpnError,
};

use futures::{channel::mpsc::Sender, Stream};
use tracing::{error, info};

pub async fn run_client<S>(
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
