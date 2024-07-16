use anyhow::Result;
use tokio::net::TcpStream;

pub async fn run_tcp_server(_stream: TcpStream) -> Result<()> {
    // let mut incoming = listener.incoming();
    // while let Some(stream) = incoming.next().await {
    //     let stream = stream?;
    //     tokio::spawn(async move {
    //         let (r, w) = stream.into_split();
    //         let read_stream = VpnProstReadStream::new(r);
    //         let write_stream = VpnProstWriteStream::new(w);
    //         let mut server = VpnServer::new(read_stream, write_stream);
    //         server.process().await.unwrap();
    //     });
    // }
    Ok(())
}
