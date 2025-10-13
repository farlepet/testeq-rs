//! Open Network Computing (ONC) RPC protocol, as defined by RFC5531

use std::{mem, net::SocketAddr, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpSocket, TcpStream},
    sync::Mutex,
};

use crate::error::{Error, Result};

use super::xdr::{self, XdrPack};

pub const RPC_VERSION: u32 = 2;

pub const LAST_MESSAGE_MARKER: u32 = 0x80000000;

pub struct OncClient {
    socket: SocketAddr,
    stream: Option<Arc<Mutex<TcpStream>>>,
    last_xid: u32,
}
impl OncClient {
    pub fn new(socket: SocketAddr) -> Self {
        Self {
            socket,
            stream: None,
            last_xid: 0,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Err(Error::Unspecified("Already connected!".into()));
        }

        /* TODO: Support IPv6 */
        let socket = TcpSocket::new_v4().map_err(|e| Error::Unhandled(e.into()))?;
        self.stream = Some(Arc::new(Mutex::new(
            socket
                .connect(self.socket)
                .await
                .map_err(|e| Error::Unhandled(e.into()))?,
        )));

        Ok(())
    }

    pub async fn request(&mut self, req: impl XdrPack) -> Result<Vec<RpcMessage>> {
        let Some(stream) = &self.stream else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        let mut packed = vec![];
        req.pack_xdr(&mut packed);

        let header = packed.len() as u32 | LAST_MESSAGE_MARKER;
        let mut packet = vec![];
        packet.extend_from_slice(&header.to_be_bytes());
        packet.append(&mut packed);

        let mut stream = stream.lock().await;
        stream.write_all(&packet).await?;

        /* TODO: Timeout */
        let resp = self.read_response(&mut stream).await;

        self.last_xid += 1;

        resp
    }

    async fn read_response(&self, stream: &mut TcpStream) -> Result<Vec<RpcMessage>> {
        let mut responses = vec![];

        loop {
            let header = stream.read_u32().await?;
            let size = (header & !LAST_MESSAGE_MARKER) as usize;

            let mut packet = vec![0; size];
            stream.read_exact(&mut packet).await?;

            let unpacked = RpcMessage::unpack(&mut packet)?;
            if unpacked.xid == self.last_xid {
                responses.push(unpacked);

                if (header & LAST_MESSAGE_MARKER) != 0 {
                    break;
                }
            } else {
                println!("Received non-matching xid: {}", unpacked.xid);
            }
        }

        Ok(responses)
    }

    pub fn gen_call_packet(
        &self,
        prog: u32,
        vers: u32,
        proc: u32,
        req: impl XdrPack,
    ) -> RpcMessage {
        let mut args = vec![];
        req.pack_xdr(&mut args);

        RpcMessage {
            xid: self.last_xid,
            body: MessageBody::Call(CallBody {
                rpc_version: RPC_VERSION,
                prog,
                vers,
                proc,
                cred: OpaqueAuth::new_null(),
                verf: OpaqueAuth::new_null(),
                args,
            }),
        }
    }
}

#[derive(Debug)]
pub enum AuthStat {
    AuthOk = 0,
    AuthBadCred = 1,
    AuthRejectedCred = 2,
    AuthBadVerf = 3,
    AuthRejectedVerf = 4,
    AuthTooWeak = 5,
    AuthInvalidResp = 6,
    AuthFailed = 7,
    AuthKerbGeneric = 8,
    AuthTimeExpire = 9,
    AuthTktFile = 10,
    AuthDecode = 11,
    AuthNetAddr = 12,
    RpcSecGssCredProblem = 13,
    RpcSecGssCtxProblem = 14,
}
impl AuthStat {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        match xdr::unpack_u32(src)? {
            0 => Ok(Self::AuthOk),
            1 => Ok(Self::AuthBadCred),
            2 => Ok(Self::AuthRejectedCred),
            3 => Ok(Self::AuthBadVerf),
            4 => Ok(Self::AuthRejectedVerf),
            5 => Ok(Self::AuthTooWeak),
            6 => Ok(Self::AuthInvalidResp),
            7 => Ok(Self::AuthFailed),
            8 => Ok(Self::AuthKerbGeneric),
            9 => Ok(Self::AuthTimeExpire),
            10 => Ok(Self::AuthTktFile),
            11 => Ok(Self::AuthDecode),
            12 => Ok(Self::AuthNetAddr),
            13 => Ok(Self::RpcSecGssCredProblem),
            14 => Ok(Self::RpcSecGssCtxProblem),
            i => Err(Error::BadResponse(format!("Unknown message type {}", i))),
        }
    }
}

#[derive(Debug)]
pub enum MessageBody {
    Call(CallBody),
    Reply(ReplyBody),
}
impl MessageBody {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        match xdr::unpack_u32(src)? {
            0 => Ok(Self::Call(CallBody::unpack(src)?)),
            1 => Ok(Self::Reply(ReplyBody::unpack(src)?)),
            i => Err(Error::BadResponse(format!("Unknown message type {}", i))),
        }
    }
}
impl XdrPack for MessageBody {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        match self {
            Self::Call(call) => {
                0u32.pack_xdr(out);
                call.pack_xdr(out);
            }
            Self::Reply(reply) => {
                1u32.pack_xdr(out);
                reply.pack_xdr(out);
            }
        }
    }
}

#[derive(Debug)]
pub struct RpcMessage {
    pub xid: u32,
    pub body: MessageBody,
}
impl RpcMessage {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            xid: xdr::unpack_u32(src)?,
            body: MessageBody::unpack(src)?,
        })
    }

    pub fn get_success_result(&self) -> Result<&[u8]> {
        let MessageBody::Reply(reply) = &self.body else {
            return Err(Error::Unspecified("Not a reply message".into()));
        };

        let reply = match reply {
            ReplyBody::Rejected(reject) => {
                return Err(Error::Unspecified(format!(
                    "Reply is an rejection: {:?}",
                    reject
                )));
            }
            ReplyBody::Accepted(accept) => accept,
        };

        let reply = match &reply.body {
            AcceptedReplyBodyType::Success(success) => success,
            resp => {
                return Err(Error::Unspecified(format!(
                    "Reply is an accept with error: {:?}",
                    resp
                )));
            }
        };

        Ok(&reply.results)
    }
}
impl XdrPack for RpcMessage {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        self.xid.pack_xdr(out);
        self.body.pack_xdr(out);
    }
}

#[derive(Debug)]
pub struct CallBody {
    pub rpc_version: u32,
    pub prog: u32,
    pub vers: u32,
    pub proc: u32,
    pub cred: OpaqueAuth,
    pub verf: OpaqueAuth,
    pub args: Vec<u8>,
}
impl CallBody {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            rpc_version: xdr::unpack_u32(src)?,
            prog: xdr::unpack_u32(src)?,
            vers: xdr::unpack_u32(src)?,
            proc: xdr::unpack_u32(src)?,
            cred: OpaqueAuth::unpack(src)?,
            verf: OpaqueAuth::unpack(src)?,
            args: xdr::unpack_opaque(src)?,
        })
    }
}
impl XdrPack for CallBody {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        self.rpc_version.pack_xdr(out);
        self.prog.pack_xdr(out);
        self.vers.pack_xdr(out);
        self.proc.pack_xdr(out);
        self.cred.pack_xdr(out);
        self.verf.pack_xdr(out);
        out.extend(self.args);
    }
}

#[derive(Debug)]
pub enum ReplyBody {
    Accepted(AcceptedReplyBody),
    Rejected(RejectedReplyBody),
}
impl ReplyBody {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        match xdr::unpack_u32(src)? {
            0 => Ok(Self::Accepted(AcceptedReplyBody::unpack(src)?)),
            1 => Ok(Self::Rejected(RejectedReplyBody::unpack(src)?)),
            i => Err(Error::BadResponse(format!("Unknown message type {}", i))),
        }
    }
}
impl XdrPack for ReplyBody {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        match self {
            Self::Accepted(_accepted) => {
                0u32.pack_xdr(out);
                unimplemented!()
            }
            Self::Rejected(_rejected) => {
                1u32.pack_xdr(out);
                unimplemented!()
            }
        }
    }
}

#[derive(Debug)]
pub struct AcceptedReplyBody {
    pub verf: OpaqueAuth,
    pub body: AcceptedReplyBodyType,
}
impl AcceptedReplyBody {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            verf: OpaqueAuth::unpack(src)?,
            body: AcceptedReplyBodyType::unpack(src)?,
        })
    }
}

#[derive(Debug)]
pub enum AcceptedReplyBodyType {
    Success(SuccessAcceptedReplyBody),
    ProgUnavail(),
    ProgMismatch(ProgMismatchBody),
    ProcUnavail(),
    GarbageArgs(),
    SystemErr(),
}
impl AcceptedReplyBodyType {
    fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        match xdr::unpack_u32(src)? {
            0 => Ok(Self::Success(SuccessAcceptedReplyBody::unpack(src)?)),
            1 => Ok(Self::ProgUnavail()),
            2 => Ok(Self::ProgMismatch(ProgMismatchBody::unpack(src)?)),
            3 => Ok(Self::ProcUnavail()),
            4 => Ok(Self::GarbageArgs()),
            5 => Ok(Self::SystemErr()),
            i => Err(Error::BadResponse(format!(
                "Unknown accepted reply type {}",
                i
            ))),
        }
    }
}

#[derive(Debug)]
pub struct SuccessAcceptedReplyBody {
    pub results: Vec<u8>,
}
impl SuccessAcceptedReplyBody {
    fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            results: mem::take(src),
        })
    }
}

#[derive(Debug)]
pub struct ProgMismatchBody {
    pub low: u32,
    pub high: u32,
}
impl ProgMismatchBody {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            low: xdr::unpack_u32(src)?,
            high: xdr::unpack_u32(src)?,
        })
    }
}

#[derive(Debug)]
pub enum RejectedReplyBody {
    Mismatch(ProgMismatchBody),
    AuthError(AuthStat),
}
impl RejectedReplyBody {
    fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        match xdr::unpack_u32(src)? {
            0 => Ok(Self::Mismatch(ProgMismatchBody::unpack(src)?)),
            1 => Ok(Self::AuthError(AuthStat::unpack(src)?)),
            i => Err(Error::BadResponse(format!(
                "Unknown rejected reply type {}",
                i
            ))),
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum AuthFlavor {
    Null = 0,
    Sys = 1,
    Short = 2,
    Dh = 3,
    RpcSecGss = 4,
}

#[derive(Debug)]
pub struct OpaqueAuth {
    pub flavor: AuthFlavor,
    pub body: Vec<u8>,
}
impl OpaqueAuth {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            flavor: match xdr::unpack_u32(src)? {
                0 => AuthFlavor::Null,
                1 => AuthFlavor::Sys,
                2 => AuthFlavor::Short,
                3 => AuthFlavor::Dh,
                4 => AuthFlavor::RpcSecGss,
                i => return Err(Error::BadResponse(format!("Unknown auth flavor {}", i))),
            },
            body: xdr::unpack_opaque(src)?,
        })
    }

    pub fn new_null() -> Self {
        Self {
            flavor: AuthFlavor::Null,
            body: vec![],
        }
    }
}
impl XdrPack for OpaqueAuth {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        (self.flavor as u32).pack_xdr(out);
        self.body.pack_xdr(out);
    }
}
