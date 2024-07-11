use thiserror::Error;

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

    #[error("Unsupported socks version: {0}")]
    UnsupportedSocksVersion(u8),

    #[error("Auth Method nnacceptable: {0:?}")]
    AuthMethodUnacceptable(Vec<u8>),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Authentication rejected: {0}")]
    AuthenticationRejected(String),
}
