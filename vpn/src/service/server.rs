use anyhow::Result;
use futures::join;
use tokio::net::TcpStream;

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

    join!(r, w);

    Ok(())
}
