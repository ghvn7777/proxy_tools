use thiserror::Error;

use crate::pb::VpnCommandResponse;

#[derive(Error, Debug)]
pub enum VpnError {
    #[error("general error: {0}")]
    AnyError(#[from] anyhow::Error),

    #[error("Failed to encode protobuf message")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Failed to decode protobuf message")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Frame is larger than max size")]
    FrameError,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Socks5 error: {0}")]
    Socks5Error(#[from] Socks5Error),

    #[error("AddError: {0}")]
    AddrError(#[from] AddrError),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Yamux connect error: {0}")]
    YamuxConnectionError(#[from] yamux::ConnectionError),

    #[error("Send stream data error: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<VpnCommandResponse>),

    #[error("Tcp connect error: {0}")]
    TcpConnectkError(String),

    #[error("Tcp disconnect error: {0}")]
    TcpDisconnectError(String),

    #[error("Tokio join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("TLS error")]
    TlsError(#[from] tokio_rustls::rustls::TLSError),

    #[error("Certificate parse error: error to load {0} {0}")]
    CertificateParseError(&'static str, &'static str),
}

#[derive(Error, Debug)]
pub enum Socks5Error {
    #[error("Domain exceeded max sequence length")]
    ExceededMaxDomainLen(usize),

    #[error("Unsupported socks version: {0}")]
    UnsupportedSocksVersion(u8),

    #[error("Auth Method nnacceptable: {0:?}")]
    AuthMethodUnacceptable(Vec<u8>),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Authentication rejected: {0}")]
    AuthenticationRejected(String),

    #[error("Socks command not supported")]
    SocksCommandNotSupported,
}

#[derive(Error, Debug)]
pub enum AddrError {
    #[error("DNS Resolution failed")]
    DNSResolutionFailed,

    #[error("Can't read IPv4")]
    IPv4Unreadable,

    #[error("Can't parse IPv4")]
    ParseIpv4Error,

    #[error("Can't read IPv6")]
    IPv6Unreadable,

    #[error("Can't parse IPv6")]
    ParseIpv6Error,

    #[error("Can't read port number")]
    PortNumberUnreadable,

    #[error("Can't read domain len")]
    DomainLenUnreadable,

    #[error("Can't read Domain content")]
    DomainContentUnreadable,

    #[error("Malformed UTF-8")]
    Utf8,

    #[error("Unknown address type")]
    IncorrectAddressType,

    #[error("Empty address")]
    Empty,

    #[error("{0}")]
    Custom(String),
}

impl From<VpnError> for VpnCommandResponse {
    fn from(value: VpnError) -> Self {
        VpnCommandResponse::new_error(value.to_string())
    }
}
