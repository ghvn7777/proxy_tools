use std::{pin::Pin, sync::Arc};

use anyhow::Result;
use futures::Stream;

use crate::{
    pb::{VpnCommandRequest, VpnCommandResponse},
    VpnError,
};

pub type StreamingResponse = Pin<Box<dyn Stream<Item = Arc<VpnCommandResponse>> + Send>>;

impl VpnCommandRequest {
    pub async fn execute(&self) -> Result<StreamingResponse, VpnError> {
        // let _res = match &self.command {
        //     Some(cmd) => match cmd {
        //         Command::Connect(addr) => {
        //             let _addr: TargetAddr = addr.try_into()?;
        //             todo!()
        //         }
        //         Command::Disconnect(_addr) => {
        //             todo!()
        //         }
        //         Command::Data(_data) => {
        //             todo!()
        //         }
        //     },
        //     None => return Err(VpnError::InternalError("Command error".to_string())),
        // };

        Err(VpnError::InternalError("Command error".to_string()))
        // Ok(Box::pin(stream::once(async { Arc::new(res) })))
    }
}
