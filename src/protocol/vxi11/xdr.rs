//! External Data Representation (XDR), as defined by RFC4506

use crate::error::{Error, Result};

pub trait XdrPack {
    /// Consume self, appending XDR representation into out
    fn pack_xdr(self, out: &mut Vec<u8>);
}

impl XdrPack for u32 {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        out.extend(self.to_be_bytes());
    }
}

impl XdrPack for i32 {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        out.extend(self.to_be_bytes());
    }
}

impl XdrPack for bool {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        (self as u32).pack_xdr(out);
    }
}

impl XdrPack for Vec<u8> {
    /// This only applies to "opaque" data buffers that are prefixed by the size
    fn pack_xdr(self, out: &mut Vec<u8>) {
        let len = self.len();
        (self.len() as u32).pack_xdr(out);
        out.extend(self);
        /* Must be padded to multiple of 32-byte words */
        if !len.is_multiple_of(4) {
            for _ in 0..(4 - (len % 4)) {
                out.push(0);
            }
        }
    }
}

impl XdrPack for String {
    fn pack_xdr(self, out: &mut Vec<u8>) {
        self.into_bytes().pack_xdr(out);
    }
}

pub fn unpack_u32(src: &mut Vec<u8>) -> Result<u32> {
    let bytes = src
        .drain(0..4)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| Error::BadResponse("Not enough bytes to read u32".to_string()))?;

    Ok(u32::from_be_bytes(bytes))
}

pub fn unpack_i32(src: &mut Vec<u8>) -> Result<i32> {
    let bytes = src
        .drain(0..4)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| Error::BadResponse("Not enough bytes to read i32".to_string()))?;

    Ok(i32::from_be_bytes(bytes))
}

pub fn unpack_u16(src: &mut Vec<u8>) -> Result<u16> {
    let bytes = src
        .drain(0..4)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| Error::BadResponse("Not enough bytes to read u32".to_string()))?;

    let val = u32::from_be_bytes(bytes);
    val.try_into()
        .map_err(|_| Error::BadResponse(format!("Value {val} does not represent a 16-bit value")))
}

pub fn unpack_opaque(src: &mut Vec<u8>) -> Result<Vec<u8>> {
    let length = unpack_u32(src)? as usize;
    let padding = if !length.is_multiple_of(4) {
        4 - (length % 4)
    } else {
        0
    };

    if src.len() < (length + padding) {
        return Err(Error::BadResponse(
            "Not enough bytes to read opaque type".to_string(),
        ));
    }

    let res = src.drain(0..length).collect();
    src.drain(0..padding);
    Ok(res)
}
