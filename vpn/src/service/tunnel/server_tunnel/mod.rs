use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    util::{TargetAddr, ToTargetAddr},
    RemoteMsg, RemoteToServer, ServerMsg, TunnelReader, TunnelWriter,
};

pub async fn tunnel_port_task(
    target_addr: TargetAddr,
    reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    trace!("runnel_port_task: start");
    let id = reader_tunnel.get_id();
    let target_addr_clone = target_addr.clone();
    let stream = match target_addr {
        TargetAddr::Domain(domain_addr, port) => TcpStream::connect((domain_addr, port)).await.ok(),
        TargetAddr::Ip(ip) => TcpStream::connect(ip).await.ok(),
    };

    let Some(stream) = stream else {
        error!("Connect {:?} failed", target_addr_clone);
        let _ = writer_tunnel
            .send(RemoteToServer::TcpConnectFailed(id).into())
            .await;
        let _ = writer_tunnel
            .send(RemoteToServer::ClosePort(id).into())
            .await;
        return;
    };

    match stream.local_addr() {
        Ok(bind_addr) => {
            debug!("Tcp connect local bind addr: {:?}", bind_addr);
            match bind_addr.to_target_addr() {
                Ok(bind_addr) => {
                    debug!(
                        "Tcp connect local bind addr to target addr: {:?}",
                        bind_addr
                    );
                    let _ = writer_tunnel
                        .send(RemoteToServer::TcpConnectSuccess(id, bind_addr).into())
                        .await;
                }
                Err(_) => {
                    error!("Bind addr to target addr failed");
                    let _ = writer_tunnel
                        .send(RemoteToServer::TcpConnectFailed(id).into())
                        .await;
                    let _ = writer_tunnel
                        .send(RemoteToServer::ClosePort(id).into())
                        .await;
                    return;
                }
            }
        }
        Err(_) => {
            error!("Get local addr failed");
            let _ = writer_tunnel
                .send(RemoteToServer::TcpConnectFailed(id).into())
                .await;
            let _ = writer_tunnel
                .send(RemoteToServer::ClosePort(id).into())
                .await;
            return;
        }
    }

    let (reader, writer) = stream.into_split();
    let r = read_remote_tcp(id, reader, writer_tunnel);
    let w = write_remote_tcp(writer, reader_tunnel);

    tokio::select! {
        _ = r => {
            info!("Read remote tcp task end");
        }
        _ = w => {
            info!("Write remote tcp task end");
        }
    };
    // join!(w, r);
    info!("Server tunnel port task end id: {}", id);
}

async fn read_remote_tcp(
    id: u32,
    mut stream: OwnedReadHalf,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    let mut buf = vec![0u8; 1024 * 4];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                info!("Tcp remote read 0");
                break;
            }

            Ok(n) => {
                let mut buf2 = Vec::with_capacity(n);
                buf2.extend_from_slice(&buf[0..n]);
                let msg = RemoteToServer::Data(id, Box::new(buf2)).into();
                info!("Read remote tcp data len: {:?}", n);
                if writer_tunnel.send(msg).await.is_err() {
                    error!("Write tunnel error");
                    break;
                }
            }

            Err(_) => {
                error!("Server Tcp Stream read error");
                break;
            }
        }
    }

    info!("Read remote tcp end");
    let msg = RemoteToServer::ClosePort(id).into();
    if writer_tunnel.send(msg).await.is_err() {
        error!("Write tunnel error");
    }
}

async fn write_remote_tcp(
    mut stream: OwnedWriteHalf,
    mut reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
) {
    loop {
        match reader_tunnel.read().await {
            Some(msg) => match msg {
                RemoteMsg::Data(data) => {
                    info!("Write remote tcp len: {:?}", data.len());
                    if stream.write_all(&data).await.is_err() {
                        warn!("Write remote tcp error");
                        break;
                    }
                }
                RemoteMsg::ClosePort(_) => {
                    warn!("Remote tcp close port");
                    break;
                }
            },
            None => {
                warn!("Read tunnel error");
                break;
            }
        }
    }

    reader_tunnel.rx = None;
    info!("Write remote tcp end");
    let id = reader_tunnel.get_id();
    let msg = RemoteToServer::ClosePort(id).into();
    if reader_tunnel.send(msg).await.is_err() {
        error!("Write tunnel error");
    }
}
