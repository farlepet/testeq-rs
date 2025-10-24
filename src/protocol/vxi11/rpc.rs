//! RPC methods defined by VXI-11

use super::xdr::{self, XdrPack};

use crate::error::Result;

#[allow(unused)]
#[repr(u8)]
pub enum RpcRequest {
    DeviceAbort = 1,
    CreateLink = 10,
    DeviceWrite = 11,
    DeviceRead = 12,
    DeviceReadStb = 13,
    DeviceTrigger = 14,
    DeviceClear = 15,
    DeviceError = 16,
    DeviceLocal = 17,
    DeviceLock = 18,
    DeviceUnlock = 19,
    DeviceEnableSrq = 20,
    DeviceDoCmd = 22,
    DestroyLink = 23,
    CreateIntrChan = 25,
    DestroyIntrChan = 26,
    DeviceIntrSrq = 30,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RpcDeviceErrorCode {
    NoError,
    SyntaxError,
    DeviceNotAccessible,
    InvalidLinkIdentifier,
    ParameterError,
    ChannelNotEstablished,
    OperationNotSupported,
    OutOfResources,
    DeviceLockedByAnotherLink,
    NoLockHeldByThisLink,
    IoTimeout,
    IoError,
    InvalidAddress,
    Abort,
    ChannelAlreadyEstablished,
    Unknown(u32),
}
impl RpcDeviceErrorCode {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(match xdr::unpack_u32(src)? {
            0 => Self::NoError,
            1 => Self::SyntaxError,
            3 => Self::DeviceNotAccessible,
            4 => Self::InvalidLinkIdentifier,
            5 => Self::ParameterError,
            6 => Self::ChannelNotEstablished,
            8 => Self::OperationNotSupported,
            9 => Self::OutOfResources,
            11 => Self::DeviceLockedByAnotherLink,
            12 => Self::NoLockHeldByThisLink,
            15 => Self::IoTimeout,
            17 => Self::IoError,
            21 => Self::InvalidAddress,
            23 => Self::Abort,
            29 => Self::ChannelAlreadyEstablished,
            i => Self::Unknown(i),
        })
    }
}

#[derive(Debug)]
pub struct RpcOperationFlags {
    /// Wait for lock even if lock timeout elapses
    pub wait_lock: bool,
    /// On write, send last byte with END indicator
    pub end: bool,
    /// On read, use termchr as termination characer
    pub termchr_set: bool,
}
impl XdrPack for RpcOperationFlags {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        let mut flags = 0;
        if self.wait_lock {
            flags |= 1 << 0;
        }
        if self.end {
            flags |= 1 << 3;
        }
        if self.termchr_set {
            flags |= 1 << 7;
        }
        flags.pack_xdr(out);
    }
}

#[derive(Debug)]
pub struct RpcRequestCreateDeviceLink {
    /// ID representing client
    pub client_id: i32,
    /// Whether to lock the device
    pub lock_device: bool,
    /// How long to wait for a lock to be released
    pub lock_timeout: u32,
    /// Name of device
    pub device: String,
}
impl XdrPack for RpcRequestCreateDeviceLink {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        self.client_id.pack_xdr(out);
        self.lock_device.pack_xdr(out);
        self.lock_timeout.pack_xdr(out);
        self.device.pack_xdr(out);
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct RpcResponseCreateDeviceLink {
    /// Error code
    pub error: RpcDeviceErrorCode,
    /// New link ID
    pub lid: i32,
    /// Abort RPC port
    pub abort_port: u16,
    /// Max data size device will accept on write
    pub max_recv_size: u32,
}
impl RpcResponseCreateDeviceLink {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            error: RpcDeviceErrorCode::unpack(src)?,
            lid: xdr::unpack_i32(src)?,
            abort_port: xdr::unpack_u16(src)?,
            max_recv_size: xdr::unpack_u32(src)?,
        })
    }
}

#[derive(Debug)]
pub struct RpcRequestDeviceWrite {
    /// Link ID
    pub lid: i32,
    /// Time to wait for I/O
    pub io_timeout: u32,
    /// Time to wait for lock
    pub lock_timeout: u32,
    /// Flags
    pub flags: RpcOperationFlags,
    /// Data to write
    pub data: Vec<u8>,
}
impl XdrPack for RpcRequestDeviceWrite {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        self.lid.pack_xdr(out);
        self.io_timeout.pack_xdr(out);
        self.lock_timeout.pack_xdr(out);
        self.flags.pack_xdr(out);
        self.data.pack_xdr(out);
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct RpcResponseDeviceWrite {
    /// Error code
    pub error: RpcDeviceErrorCode,
    /// Number of bytes written
    pub size: u32,
}
impl RpcResponseDeviceWrite {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            error: RpcDeviceErrorCode::unpack(src)?,
            size: xdr::unpack_u32(src)?,
        })
    }
}

#[derive(Debug)]
pub struct RpcRequestDeviceRead {
    /// Link ID
    pub lid: i32,
    /// Bytes requested
    pub request_size: u32,
    /// Time to wait for I/O
    pub io_timeout: u32,
    /// Time to wait for lock
    pub lock_timeout: u32,
    /// Flags
    pub flags: RpcOperationFlags,
    /// Optional termination character
    pub termchr: u8,
}
impl XdrPack for RpcRequestDeviceRead {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        self.lid.pack_xdr(out);
        self.request_size.pack_xdr(out);
        self.io_timeout.pack_xdr(out);
        self.lock_timeout.pack_xdr(out);
        self.flags.pack_xdr(out);
        (self.termchr as u32).pack_xdr(out);
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct RpcDeviceReadReason {
    /// request_size bytes have been transferred
    pub reqcnt: bool,
    /// Match on termchr
    pub chr: bool,
    /// END indicator has been read
    pub end: bool,
}
impl RpcDeviceReadReason {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        let flags = xdr::unpack_u32(src)?;
        Ok(Self {
            reqcnt: (flags & (1 << 0)) != 0,
            chr: (flags & (1 << 1)) != 0,
            end: (flags & (1 << 2)) != 0,
        })
    }
}

#[derive(Debug)]
pub struct RpcResponseDeviceRead {
    /// Error code
    pub error: RpcDeviceErrorCode,
    /// Why the read finished
    pub reason: RpcDeviceReadReason,
    /// Data read
    pub data: Vec<u8>,
}
impl RpcResponseDeviceRead {
    pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            error: RpcDeviceErrorCode::unpack(src)?,
            reason: RpcDeviceReadReason::unpack(src)?,
            data: xdr::unpack_opaque(src)?,
        })
    }
}
