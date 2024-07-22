use std::{sync::Arc, time::Duration};

use futures::{channel::mpsc::Sender, Stream};
use quinn::{crypto::rustls::QuicClientConfig, ClientConfig, Endpoint};
use rustls::pki_types::{ServerName, UnixTime};
use tokio_stream::StreamExt;
use tonic::transport::CertificateDer;
use tracing::{error, info, trace};

use crate::{
    interval, util::channel_bus, ClientMsg, ClientQuicConn, ClientReadProcessor,
    ClientWriteProcessor, DataCrypt, Processor, StreamSplit, Tunnel, VpnError,
    HEARTBEAT_INTERVAL_MS,
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
                    &mut msg_stream,
                    main_sender_clone.clone(),
                    crypt.clone(),
                )
                .await
                {
                    Ok(_) => info!("Tcp tunnel core task finished"),
                    Err(e) => error!("Tcp tunnel core task error: {:?}", e),
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

async fn quic_tunnel_core_task<S: Stream<Item = ClientMsg> + Unpin + Send + Sync + 'static>(
    server_addr: String,
    msg_stream: &mut S,
    main_sender_tx: Sender<ClientMsg>,
    crypt: Arc<Box<dyn DataCrypt>>,
) -> Result<(), VpnError> {
    trace!("Tcp tunnel core task start");
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;

    endpoint.set_default_client_config(ClientConfig::new(Arc::new(QuicClientConfig::try_from(
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(SkipServerVerification::new())
            .with_no_client_auth(),
    )?)));

    // connect to server
    let connection = endpoint
        .connect(server_addr.parse().unwrap(), "kaka")
        .unwrap()
        .await
        .unwrap();
    info!("[client] connected: addr={}", connection.remote_address());

    // Split client to Server stream
    let (reader, writer) = ClientQuicConn::new(connection).stream_split().await;
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

    info!("Tcp tunnel core task finished");

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
