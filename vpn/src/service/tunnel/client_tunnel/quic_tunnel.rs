use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::{channel::mpsc::Sender, Stream};
use quinn::{crypto::rustls::QuicClientConfig, ClientConfig, Endpoint};
use rustls::pki_types::{ServerName, UnixTime};
use tokio_stream::StreamExt;
use tonic::transport::CertificateDer;
use tracing::{error, info, trace};

use crate::{
    client::run_client, interval, util::channel_bus, ClientMsg, ClientQuicConn, DataCrypt, Tunnel,
    VpnError, HEARTBEAT_INTERVAL_MS,
};

pub struct QuicTunnel;

impl QuicTunnel {
    pub fn generate(server_addr: String, crypt: Arc<Box<dyn DataCrypt>>) -> Tunnel {
        let (main_sender, sub_senders, receivers) = channel_bus(10, 1000);
        let main_sender_clone = main_sender.clone();

        tokio::spawn(async move {
            let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
            let timer_stream = interval(duration, ClientMsg::Heartbeat);
            let mut msg_stream = timer_stream.merge(receivers);
            loop {
                match quic_tunnel_core_task(
                    server_addr.clone(),
                    crypt.clone(),
                    &mut msg_stream,
                    main_sender_clone.clone(),
                )
                .await
                {
                    Ok(_) => info!("Quic tunnel core task finished"),
                    Err(e) => error!("Quic tunnel core task error: {:?}", e),
                }
            }
        });

        Tunnel {
            connect_id: 0,
            senders: sub_senders,
            main_sender,
        }
    }
}

async fn get_quic_stream(server_addr: String) -> Result<ClientQuicConn, VpnError> {
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;

    endpoint.set_default_client_config(ClientConfig::new(Arc::new(QuicClientConfig::try_from(
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(SkipServerVerification::new())
            .with_no_client_auth(),
    )?)));

    // connect to server
    let connection = endpoint
        .connect(server_addr.parse().unwrap(), "kaka")?
        .await?;

    info!("[client] connected: addr={}", connection.remote_address());

    Ok(ClientQuicConn::new(connection))
}

async fn quic_tunnel_core_task<S>(
    server_addr: String,
    crypt: Arc<Box<dyn DataCrypt>>,
    msg_stream: &mut S,
    main_sender_tx: Sender<ClientMsg>,
) -> Result<(), VpnError>
where
    S: Stream<Item = ClientMsg> + Unpin + Send + Sync + 'static,
{
    trace!("Quic tunnel core task start");
    let connection = get_quic_stream(server_addr).await?;
    run_client(connection, crypt, msg_stream, main_sender_tx).await?;
    info!("tunnel core task finished");

    Ok(())
}

/// Dummy certificate verifier that treats any certificate as valid.
/// NOTE, such verification is vulnerable to MITM attacks, but convenient for testing.
#[derive(Debug)]
struct SkipServerVerification(Arc<rustls::crypto::CryptoProvider>);

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(rustls::crypto::ring::default_provider())))
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}
