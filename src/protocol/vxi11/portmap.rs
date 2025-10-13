//! Portmap client, RFC1833

use std::net::SocketAddr;

use crate::error::{Error, Result};

use super::{
    onc::OncClient,
    xdr::{self, XdrPack},
};

const PORTMAP_PROG: u32 = 100000;
const PORTMAP_VERS: u32 = 2;

pub const PORTMAP_PORT: u16 = 111;

/// Connect to an ONC server, and request a port for a program
pub async fn connect_and_request_port(
    socket: SocketAddr,
    prog: u32,
    vers: u32,
    prot: RpcIpProto,
) -> Result<u16> {
    let mut client = OncClient::new(socket);
    client.connect().await?;

    request_port(&mut client, prog, vers, prot).await
}

/// Request a port for a program, using an existing ONC client
pub async fn request_port(
    client: &mut OncClient,
    prog: u32,
    vers: u32,
    prot: RpcIpProto,
) -> Result<u16> {
    let mapping = RpcMapping {
        prog,
        vers,
        prot,
        port: 0,
    };

    let packet = client.gen_call_packet(
        PORTMAP_PROG,
        PORTMAP_VERS,
        RpcRequest::GetPort as u32,
        mapping,
    );

    let resp = client.request(packet).await?;

    if resp.len() > 1 {
        println!("Received more than one response!");
    }

    let Some(res) = resp.first() else {
        return Err(Error::Unspecified("No responses to port request?".into()));
    };

    let mut results = res.get_success_result()?.to_vec();

    xdr::unpack_u16(&mut results)
}

#[allow(unused)]
#[repr(u8)]
enum RpcRequest {
    Null = 0,
    Set = 1,
    Unset = 2,
    GetPort = 3,
    CallIt = 4,
}

#[derive(Debug)]
struct RpcMapping {
    /// Program number
    prog: u32,
    /// Version number
    vers: u32,
    /// Protocol number
    prot: RpcIpProto,
    /// Port
    port: u32,
}
impl XdrPack for RpcMapping {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        self.prog.pack_xdr(out);
        self.vers.pack_xdr(out);
        (self.prot as u32).pack_xdr(out);
        self.port.pack_xdr(out);
    }
}

#[allow(unused)]
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum RpcIpProto {
    Tcp = 6,
    Udp = 17,
}
