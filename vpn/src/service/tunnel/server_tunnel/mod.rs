use futures::join;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};
use tracing::{error, info};

use crate::{util::TargetAddr, RemoteMsg, RemoteToServer, ServerMsg, TunnelReader, TunnelWriter};

pub async fn tunnel_port_task(
    id: u32,
    target_addr: TargetAddr,
    reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    let target_addr_clone = target_addr.clone();
    let stream = match target_addr {
        TargetAddr::Domain(domain_addr, port) => TcpStream::connect((domain_addr, port)).await.ok(),
        TargetAddr::Ip(ip) => TcpStream::connect(ip).await.ok(),
    };

    let Some(stream) = stream else {
        error!("Connect {:?} failed", target_addr_clone);
        return;
    };

    match stream.local_addr() {
        Ok(addr) => {
            info!("Tcp connect success: {:?}", addr);
            let _ = writer_tunnel
                .send(RemoteToServer::TcpConnectSuccess(id).into())
                .await;
        }
        Err(_) => {
            error!("Get local addr failed");
            let _ = writer_tunnel
                .send(RemoteToServer::TcpConnectFailed(id).into())
                .await;
        }
    }

    let (mut reader, mut writer) = stream.into_split();
    let w = read_remote_tcp(id, &mut reader, writer_tunnel);
    let r = write_remote_tcp(&mut writer, reader_tunnel);
    join!(w, r);
}

async fn read_remote_tcp(
    id: u32,
    stream: &mut OwnedReadHalf,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    let mut buf = vec![0; 1024 * 30];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                // let _ = stream.shutdown(Shutdown::Read);
                // write_port.shutdown_write().await;
                // write_port.drop().await;
                error!("Tcp read 0");
                break;
            }

            Ok(n) => {
                let msg = RemoteToServer::Data(id, buf[..n].to_vec()).into();
                if writer_tunnel.send(msg).await.is_err() {
                    error!("Write tunnel error");
                    break;
                }
            }

            Err(_) => {
                error!("Stream read error");
                // let _ = stream.shutdown(Shutdown::Both);
                // write_port.close().await;
                break;
            }
        }
    }
}

async fn write_remote_tcp(
    stream: &mut OwnedWriteHalf,
    mut reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
) {
    loop {
        match reader_tunnel.read().await {
            Some(msg) => match msg {
                RemoteMsg::Data(data) => {
                    if stream.write_all(&data).await.is_err() {
                        error!("Write remote tcp error");
                        break;
                    }
                }
            },
            _ => {
                error!("Read tunnel error");
                break;
            }
        }
    }
}
