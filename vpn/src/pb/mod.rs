mod vpn;

pub use vpn::*;
// use crate::network::FrameCoder;

// use bytes::BytesMut;

// impl FrameCoder for Socks5Request {
//     fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), crate::VpnError> {
//         Ok(())
//     }

//     fn decode_frame(buf: &mut BytesMut) -> Result<Self, crate::VpnError> {

//         Ok(Socks5Request::default())
//     }
// }
