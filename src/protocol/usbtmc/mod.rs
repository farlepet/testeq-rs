//! USBTMC protocol, referencing USBTMC specification revision 1.0

use std::time::Duration;

use async_trait::async_trait;
use log::{debug, trace};
use rusb;
use tokio::task::block_in_place;

use crate::{
    error::{Error, Result},
    model::ModelInfo,
    protocol::{Protocol, ScpiProtocol, usbtmc::data::UsbPack},
};

mod data;

use data::{bulk_in, bulk_out};

/* TODO: Make configurable */
const MAX_TRANSFER_SIZE: usize = 4096;
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);

pub enum UsbAddress {
    /// Address device by VID, PID, and optional serial number
    VidPid {
        vid: u16,
        pid: u16,
        serial: Option<String>,
    },
    /// Address device by bus and device number
    BusDevice { bus: u8, address: u8 },
}

pub struct ScpiUsbtmcProtocol {
    device: UsbtmcDevice,
}
impl ScpiUsbtmcProtocol {
    pub fn new(addr: UsbAddress) -> Self {
        Self {
            device: UsbtmcDevice::new(addr),
        }
    }
}
#[async_trait]
impl Protocol for ScpiUsbtmcProtocol {
    async fn connect(&mut self) -> Result<()> {
        self.device.connect()
    }

    async fn disconnect(&mut self) -> Result<()> {
        /* TODO */
        Ok(())
    }

    async fn model(&mut self) -> Result<ModelInfo> {
        (self as &mut dyn ScpiProtocol).idn_model().await
    }
}
#[async_trait]
impl ScpiProtocol for ScpiUsbtmcProtocol {
    async fn int_send(&mut self, data: &[u8]) -> Result<()> {
        debug!("int_send {}", String::from_utf8_lossy(data));

        block_in_place(|| self.device.write(data, DEFAULT_TIMEOUT))
    }

    async fn int_recv(&mut self) -> Result<Vec<u8>> {
        let res = block_in_place(|| self.device.read(DEFAULT_TIMEOUT, None, None))?;

        debug!("int_recv {}", String::from_utf8_lossy(&res));

        Ok(res)
    }

    async fn int_query(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.int_send(data).await?;
        self.int_recv().await
    }

    async fn recv_raw(
        &mut self,
        length: Option<usize>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>> {
        block_in_place(|| {
            self.device
                .read(timeout.unwrap_or(DEFAULT_TIMEOUT), length, None)
        })
    }

    async fn recv_until(&mut self, byte: u8, timeout: Duration) -> Result<Vec<u8>> {
        block_in_place(|| self.device.read(timeout, None, Some(byte)))
    }

    async fn flush_rx(&mut self, timeout: Duration) -> Result<()> {
        match block_in_place(|| self.device.read(timeout, None, None)) {
            Ok(_) | Err(Error::Timeout(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// Describes a USB endpoint
struct UsbEndpoint {
    /// Endpoint interface ID
    iface: u8,
    /// Endpoint address
    address: u8,
}

struct UsbDevice {
    /// USB device handle
    handle: rusb::DeviceHandle<rusb::GlobalContext>,
    /// Bulk-IN endpoint
    endpoint_in: UsbEndpoint,
    /// Bulk-OUT endpoint
    endpoint_out: UsbEndpoint,
}

struct UsbtmcDevice {
    /// USB device address
    address: UsbAddress,
    /// Last used bTag value
    btag: u8,
    /// USB device data, populated on connect()
    device: Option<UsbDevice>,
}
impl UsbtmcDevice {
    pub fn new(address: UsbAddress) -> Self {
        Self {
            address,
            btag: 0,
            device: None,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        if self.device.is_some() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        /* TODO: Better handle USB errors */
        let devices = rusb::DeviceList::new()
            .map_err(|e| Error::Unspecified(format!("Failed to list USB devices: {e}")))?;

        let mut found_device = None;
        let mut found_device_desc = None;

        for device in devices.iter() {
            let Ok(desc) = device.device_descriptor() else {
                continue;
            };

            match self.address {
                UsbAddress::VidPid {
                    vid,
                    pid,
                    ref serial,
                } => {
                    if (desc.vendor_id() != vid) || (desc.product_id() != pid) {
                        continue;
                    }
                    if let Some(ser) = serial {
                        let Ok(hand) = device.open() else {
                            continue;
                        };
                        let Ok(dev_ser) = hand.read_serial_number_string_ascii(&desc) else {
                            continue;
                        };
                        /* Serial number string may have trailing NULL bytes */
                        if dev_ser.trim_end_matches('\0') != ser {
                            continue;
                        }
                    }
                }
                UsbAddress::BusDevice { bus, address } => {
                    if (device.bus_number() != bus) || (device.address() != address) {
                        continue;
                    }
                }
            }

            found_device_desc = Some(desc);
            found_device = Some(device);

            break;
        }

        let Some(device_desc) = found_device_desc else {
            return Err(Error::Unspecified(format!("Failed to find USB device")));
        };
        let Some(device_hand) = found_device else {
            return Err(Error::Unspecified(format!("Failed to find USB device")));
        };
        let device = device_hand
            .open()
            .map_err(|e| Error::Unspecified(format!("Failed to open USB device: {e}")))?;

        self.device = Some(UsbDevice {
            handle: device,
            endpoint_in: self.find_endpoint(
                &device_hand,
                &device_desc,
                rusb::TransferType::Bulk,
                rusb::Direction::In,
            )?,
            endpoint_out: self.find_endpoint(
                &device_hand,
                &device_desc,
                rusb::TransferType::Bulk,
                rusb::Direction::Out,
            )?,
        });

        let dev = self.device.as_ref().unwrap();

        self.claim_interface(dev.endpoint_in.iface)
            .map_err(|e| Error::Unspecified(format!("Failed to claim input endpoint: {e}")))?;
        self.claim_interface(dev.endpoint_out.iface)
            .map_err(|e| Error::Unspecified(format!("Failed to claim output endpoint: {e}")))?;

        Ok(())
    }

    fn claim_interface(&self, iface: u8) -> Result<()> {
        debug!("claim_interface {iface}");

        let Some(ref dev) = self.device else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        /* TODO: Relinquish interface on disconnect */
        if dev.handle.kernel_driver_active(iface) == Ok(true) {
            dev.handle
                .detach_kernel_driver(iface)
                .map_err(|e| Error::Unspecified(format!("Failed to detach kernel driver: {e}")))?;
        }

        dev.handle
            .claim_interface(iface)
            .map_err(|e| Error::Unspecified(format!("Failed to claim input endpoint: {e}")))?;

        Ok(())
    }

    fn find_endpoint(
        &self,
        device: &rusb::Device<rusb::GlobalContext>,
        desc: &rusb::DeviceDescriptor,
        trans_type: rusb::TransferType,
        dir: rusb::Direction,
    ) -> Result<UsbEndpoint> {
        for i in 0..desc.num_configurations() {
            let Ok(cfg_desc) = device.config_descriptor(i) else {
                continue;
            };

            for iface in cfg_desc.interfaces() {
                for iface_desc in iface.descriptors() {
                    if let Some(endp) = iface_desc
                        .endpoint_descriptors()
                        .find(|d| (d.direction() == dir) && (d.transfer_type() == trans_type))
                    {
                        return Ok(UsbEndpoint {
                            iface: iface_desc.interface_number(),
                            address: endp.address(),
                        });
                    }
                }
            }
        }

        Err(Error::Unspecified(format!(
            "Could not find USB endpoint fitting parameters"
        )))
    }

    fn write(&mut self, data: &[u8], timeout: Duration) -> Result<()> {
        debug!("write {} bytes, {} ms", data.len(), timeout.as_millis());

        let mut pos = 0;

        self.btag = self.btag.wrapping_add(1);

        while pos < data.len() {
            let transfer_size = MAX_TRANSFER_SIZE.min(data.len() - pos);

            let msg = bulk_out::Header {
                b_tag: self.btag,
                msg: bulk_out::Message::DevDepMsgOut(bulk_out::DevDepMsgOut {
                    attr: bulk_out::DevDepMsgOutAttr {
                        eom: (pos + transfer_size) >= data.len(),
                    },
                    data: data[pos..(pos + transfer_size)].to_vec(),
                }),
            };

            let mut msg_vec = vec![];
            msg.pack_usb(&mut msg_vec);

            self.write_raw(&msg_vec, timeout)?;

            pos += transfer_size;
        }

        Ok(())
    }

    fn read(
        &mut self,
        timeout: Duration,
        size: Option<usize>,
        termchr: Option<u8>,
    ) -> Result<Vec<u8>> {
        debug!("read {} ms", timeout.as_millis());

        self.btag = self.btag.wrapping_add(1);

        let msg = bulk_out::Header {
            b_tag: self.btag,
            msg: bulk_out::Message::ReqDevDepMsgIn(bulk_out::ReqDevDepMsgIn {
                max_size: size.unwrap_or(MAX_TRANSFER_SIZE - 12) as u32,
                attr: bulk_out::ReqDevDepMsgInAttr {
                    termch: termchr.is_some(),
                },
                termch: termchr.unwrap_or(0),
            }),
        };

        let mut msg_vec = vec![];
        msg.pack_usb(&mut msg_vec);

        self.write_raw(&msg_vec, timeout)?;

        let mut rx_data = vec![];
        loop {
            let mut res = self.read_raw(timeout)?;

            let msg = bulk_in::Header::unpack(&mut res)?;

            if msg.b_tag != self.btag {
                return Err(Error::BadResponse(format!(
                    "bTag does not match expected ({}, {})",
                    msg.b_tag, self.btag
                )));
            }

            let bulk_in::Message::DevDepMsgIn(msg) = msg.msg;

            rx_data.extend(msg.data);

            if msg.attr.eom || msg.attr.termch_term {
                break;
            }
        }

        return Ok(rx_data);
    }

    fn write_raw(&mut self, data: &[u8], timeout: Duration) -> Result<()> {
        debug!("write_raw {} bytes, {} ms", data.len(), timeout.as_millis());
        trace!("{:02x?}", data);

        let Some(ref dev) = self.device else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        dev.handle
            .write_bulk(dev.endpoint_out.address, data, timeout)
            .map_err(|e| Error::Unspecified(format!("Failed to write to USB device: {e}")))?;

        Ok(())
    }

    fn read_raw(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        debug!("read_raw {} ms", timeout.as_millis());

        let Some(ref dev) = self.device else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        let mut buf = [0u8; MAX_TRANSFER_SIZE];

        let len = dev
            .handle
            .read_bulk(dev.endpoint_in.address, &mut buf, timeout)
            .map_err(|e| Error::Unspecified(format!("Failed to write to USB device: {e}")))?;

        debug!("received {len} bytes");
        trace!("{:02x?}", &buf[0..len]);

        Ok(buf[0..len].to_vec())
    }
}
