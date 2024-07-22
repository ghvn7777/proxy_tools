mod client_read_process;
mod client_write_process;
mod server_read_process;
mod server_write_process;

use crate::VpnError;
use tonic::async_trait;

pub use client_read_process::ClientReadProcessor;
pub use client_write_process::ClientWriteProcessor;
pub use server_read_process::ServerReadProcessor;
pub use server_write_process::ServerWriteProcessor;

#[async_trait]
pub trait Processor {
    async fn process(&mut self) -> Result<(), VpnError>;
}
