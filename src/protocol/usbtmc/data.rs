const MSGID_DEV_DEP_MSG_OUT: u8 = 1;
const MSGID_DEV_DEP_MSG_IN: u8 = 2;

pub trait UsbPack {
    /// Consume self, appending USB message representation into out
    fn pack_usb(self, out: &mut Vec<u8>);
}

impl UsbPack for u32 {
    fn pack_usb(self, out: &mut Vec<u8>) {
        out.extend(self.to_le_bytes());
    }
}

pub mod bulk_out {
    use super::{MSGID_DEV_DEP_MSG_IN, MSGID_DEV_DEP_MSG_OUT, UsbPack};

    pub struct Header {
        /// bTag
        pub b_tag: u8,
        /// Message contents
        pub msg: Message,
    }
    impl UsbPack for Header {
        fn pack_usb(self, out: &mut Vec<u8>) {
            out.extend([self.msg.message_id(), self.b_tag, !self.b_tag, 0x00]);

            self.msg.pack_usb(out);
        }
    }

    pub struct DevDepMsgOutAttr {
        /// This is the last message
        pub eom: bool,
    }
    impl UsbPack for DevDepMsgOutAttr {
        fn pack_usb(self, out: &mut Vec<u8>) {
            let mut flags = 0;
            if self.eom {
                flags |= 1 << 0;
            }
            out.push(flags);
        }
    }

    pub struct DevDepMsgOut {
        /// Transfer attributes
        pub attr: DevDepMsgOutAttr,
        /// Payload
        pub data: Vec<u8>,
    }
    impl UsbPack for DevDepMsgOut {
        fn pack_usb(self, out: &mut Vec<u8>) {
            let len = self.data.len() as u32;
            len.pack_usb(out);
            self.attr.pack_usb(out);
            out.extend([0x00, 0x00, 0x00]);
            out.extend(self.data);

            /* Padding */
            if (len & 0x03) != 0 {
                for _ in 0..(4 - (len & 0x03)) {
                    out.push(0x00);
                }
            }
        }
    }

    pub struct ReqDevDepMsgInAttr {
        /// Enable termination character
        pub termch: bool,
    }
    impl UsbPack for ReqDevDepMsgInAttr {
        fn pack_usb(self, out: &mut Vec<u8>) {
            let mut flags = 0;
            if self.termch {
                flags |= 1 << 1;
            }
            out.push(flags);
        }
    }

    pub struct ReqDevDepMsgIn {
        /// Maximum size of returned data
        pub max_size: u32,
        /// Transfer attributes
        pub attr: ReqDevDepMsgInAttr,
        /// Termination character, if desired
        pub termch: u8,
    }
    impl UsbPack for ReqDevDepMsgIn {
        fn pack_usb(self, out: &mut Vec<u8>) {
            self.max_size.pack_usb(out);
            self.attr.pack_usb(out);
            out.extend([self.termch, 0x00, 0x00]);
        }
    }

    pub enum Message {
        DevDepMsgOut(DevDepMsgOut),
        ReqDevDepMsgIn(ReqDevDepMsgIn),
    }
    impl Message {
        fn message_id(&self) -> u8 {
            return match self {
                Self::DevDepMsgOut(_) => MSGID_DEV_DEP_MSG_OUT,
                Self::ReqDevDepMsgIn(_) => MSGID_DEV_DEP_MSG_IN,
            };
        }
    }
    impl UsbPack for Message {
        fn pack_usb(self, out: &mut Vec<u8>) {
            match self {
                Self::DevDepMsgOut(msg) => msg.pack_usb(out),
                Self::ReqDevDepMsgIn(msg) => msg.pack_usb(out),
            }
        }
    }
}

pub mod bulk_in {
    use crate::{
        error::{Error, Result},
        protocol::usbtmc::data::MSGID_DEV_DEP_MSG_IN,
    };

    fn unpack_u8(src: &mut Vec<u8>) -> Result<u8> {
        /* TODO: This is not very efficient */
        let val = *src.get(0).ok_or(Error::BadResponse(
            "Not enough bytes to read u8".to_string(),
        ))?;
        src.remove(0);
        Ok(val)
    }

    fn unpack_u32(src: &mut Vec<u8>) -> Result<u32> {
        let bytes = src
            .drain(0..4)
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| Error::BadResponse("Not enough bytes to read u32".to_string()))?;

        Ok(u32::from_le_bytes(bytes))
    }

    pub struct Header {
        /// bTag
        pub b_tag: u8,
        /// Message contents
        pub msg: Message,
    }
    impl Header {
        pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
            let msg_id = unpack_u8(src)?;
            let b_tag = unpack_u8(src)?;
            let b_tag_inv = unpack_u8(src)?;

            if b_tag != !b_tag_inv {
                return Err(Error::BadResponse(format!(
                    "Bad inverse bTag ({b_tag}, {b_tag_inv})"
                )));
            }

            /* Padding */
            unpack_u8(src)?;

            Ok(Self {
                b_tag,
                msg: Message::unpack(src, msg_id)?,
            })
        }
    }

    pub struct DevDepMsgInAttr {
        /// This message contains the last byte of the full message
        pub eom: bool,
        /// Termination character found and terminates this message
        pub termch_term: bool,
    }
    impl DevDepMsgInAttr {
        pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
            let flags = unpack_u8(src)?;

            Ok(Self {
                eom: (flags & (1 << 0)) != 0,
                termch_term: (flags & (1 << 1)) != 0,
            })
        }
    }

    pub struct DevDepMsgIn {
        /// Transfer attributes
        pub attr: DevDepMsgInAttr,
        /// Payload
        pub data: Vec<u8>,
    }
    impl DevDepMsgIn {
        pub fn unpack(src: &mut Vec<u8>) -> Result<Self> {
            let size = unpack_u32(src)?;
            let attr = DevDepMsgInAttr::unpack(src)?;
            /* Padding */
            src.drain(0..3);
            let data: Vec<_> = src.drain(0..(size as usize)).collect();
            if data.len() != size as usize {
                return Err(Error::BadResponse("Not enough bytes for payload".into()));
            }

            Ok(Self { attr, data })
        }
    }

    pub enum Message {
        DevDepMsgIn(DevDepMsgIn),
    }
    impl Message {
        pub fn unpack(src: &mut Vec<u8>, msg_id: u8) -> Result<Self> {
            match msg_id {
                MSGID_DEV_DEP_MSG_IN => Ok(Self::DevDepMsgIn(DevDepMsgIn::unpack(src)?)),
                _ => Err(Error::Unspecified(format!(
                    "Unhandled message ID: {msg_id}"
                ))),
            }
        }
    }
}
