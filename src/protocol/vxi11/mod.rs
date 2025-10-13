//! VXI-11 protocol, referencing VXI-11 1.0 specification

use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    error::{Error, Result},
    model::ModelInfo,
    protocol::vxi11::{onc::OncClient, rpc::RpcRequestDeviceRead},
};

use self::{rpc::RpcRequestDeviceWrite, xdr::XdrPack};

use super::{Protocol, ScpiProtocol};

mod onc;
pub mod portmap;
mod rpc;
mod xdr;

const VXI_CORE_PROG: u32 = 395183;
const VXI_CORE_VERS: u32 = 1;
const VXI_ABORT_PROG: u32 = 395184;
const VXI_ABORT_VERS: u32 = 1;
const VXI_INTERRUPT_PROG: u32 = 395185;
const VXI_INTERRUPT_VERS: u32 = 1;

/// Client ID to use, seems arbitrary?
const CLIENT_ID: i32 = 1;
/// Device lock timeout
const LOCK_TIMEOUT: u32 = 10000;
/// Max amount to read in a single transaction
const READ_SIZE: u32 = 65536;

pub struct ScpiVxiProtocol {
    vxi: VxiClient,
    link: Option<VxiClientLink>,
}
impl ScpiVxiProtocol {
    pub fn new(socket: SocketAddr) -> Self {
        Self {
            vxi: VxiClient::new(socket),
            link: None,
        }
    }
}
#[async_trait]
impl Protocol for ScpiVxiProtocol {
    async fn connect(&mut self) -> Result<()> {
        if self.link.is_some() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        self.vxi.connect().await?;

        self.link = Some(self.vxi.create_link().await?);

        Ok(())
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
impl ScpiProtocol for ScpiVxiProtocol {
    async fn int_send(&mut self, data: &[u8]) -> Result<()> {
        let Some(link) = &mut self.link else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        link.write(data).await
    }

    async fn int_recv(&mut self) -> Result<Vec<u8>> {
        let Some(link) = &mut self.link else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        link.recv().await
    }

    async fn int_query(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.int_send(data).await?;
        self.int_recv().await
    }

    async fn recv_raw(
        &mut self,
        _length: Option<usize>,
        _timeout: Option<Duration>,
    ) -> Result<Vec<u8>> {
        Err(Error::Unimplemented("TODO".into()))
    }

    async fn recv_until(&mut self, _byte: u8, _timeout: Duration) -> Result<Vec<u8>> {
        Err(Error::Unimplemented("TODO".into()))
    }

    async fn flush_rx(&mut self, _timeout: Duration) -> Result<()> {
        self.int_recv().await?;

        /* TODO: Use timeout */

        Ok(())
    }
}

struct VxiClientLink {
    onc_client: Arc<Mutex<OncClient>>,
    link_id: i32,
    max_recv_size: u32,
}
impl VxiClientLink {
    fn new(onc_client: Arc<Mutex<OncClient>>, link_id: i32, max_recv_size: u32) -> Self {
        Self {
            onc_client,
            link_id,
            max_recv_size,
        }
    }

    /// Write a chunk via the LXI device link, data must not exceed the max
    /// reported write size
    async fn write_packet(&mut self, data: &[u8], is_last: bool) -> Result<()> {
        if data.len() > (self.max_recv_size as usize) {
            return Err(Error::Unspecified(format!(
                "Request to write {} bytes, which is greater than reported max of {}",
                data.len(),
                self.max_recv_size
            )));
        }

        let req = RpcRequestDeviceWrite {
            lid: self.link_id,
            io_timeout: LOCK_TIMEOUT,
            lock_timeout: LOCK_TIMEOUT,
            flags: rpc::RpcOperationFlags {
                wait_lock: false,
                end: is_last,
                termchr_set: false,
            },
            data: data.to_vec(),
        };

        let mut client = self.onc_client.lock().await;

        let req = gen_call_packet(
            &client,
            VxiPortType::Core,
            rpc::RpcRequest::DeviceWrite,
            req,
        );
        let resp = client.request(req).await?;

        let mut result = resp.get_success_result()?.to_vec();
        let result = rpc::RpcResponseDeviceWrite::unpack(&mut result)?;

        if result.error != rpc::RpcDeviceErrorCode::NoError {
            return Err(Error::Unspecified(format!(
                "Failed to write to device: {:?}",
                result.error
            )));
        }

        Ok(())
    }

    /// Write data via the LXI device link, splitting it up into multiple
    /// writes if the size exceeds the maximum reported chunk size
    async fn write(&mut self, data: &[u8]) -> Result<()> {
        let n_chunks = data.len().div_ceil(self.max_recv_size as usize);
        for (index, chunk) in data.chunks(self.max_recv_size as usize).enumerate() {
            let last = index == (n_chunks - 1);
            self.write_packet(chunk, last).await?;
        }

        Ok(())
    }

    /// Perform single read transaction, returning both the data that was
    /// received and whether this is the last of the data (END condition was
    /// set)
    async fn recv_packet(&mut self) -> Result<(Vec<u8>, bool)> {
        let req = RpcRequestDeviceRead {
            lid: self.link_id,
            request_size: READ_SIZE,
            io_timeout: LOCK_TIMEOUT,
            lock_timeout: LOCK_TIMEOUT,
            flags: rpc::RpcOperationFlags {
                wait_lock: false,
                end: false,
                termchr_set: false,
            },
            termchr: 0,
        };

        let mut client = self.onc_client.lock().await;

        let req = gen_call_packet(&client, VxiPortType::Core, rpc::RpcRequest::DeviceRead, req);
        let resp = client.request(req).await?;

        let mut result = resp.get_success_result()?.to_vec();
        let result = rpc::RpcResponseDeviceRead::unpack(&mut result)?;

        if result.error != rpc::RpcDeviceErrorCode::NoError {
            return Err(Error::Unspecified(format!(
                "Failed to read from device: {:?}",
                result.error
            )));
        }

        Ok((result.data, result.reason.end))
    }

    /// Perform device read, will continue performing read calls until device
    /// indicates that we have reached the end.
    async fn recv(&mut self) -> Result<Vec<u8>> {
        let mut result = vec![];

        loop {
            let (mut res, is_last) = self.recv_packet().await?;
            result.append(&mut res);
            if is_last {
                break;
            }
        }

        Ok(result)
    }
}

struct VxiClient {
    /// Initial socket to use for portmapper
    pmap_socket: SocketAddr,
    /// ONC client for core VXI communication
    core_client: Option<Arc<Mutex<OncClient>>>,
}
impl VxiClient {
    pub fn new(socket: SocketAddr) -> Self {
        Self {
            pmap_socket: socket,
            core_client: None,
        }
    }

    async fn get_port(&self, ptype: VxiPortType) -> Result<u16> {
        portmap::connect_and_request_port(
            self.pmap_socket,
            ptype.get_prog(),
            ptype.get_vers(),
            portmap::RpcIpProto::Tcp,
        )
        .await
    }

    async fn connect(&mut self) -> Result<()> {
        let port = self.get_port(VxiPortType::Core).await?;

        let mut socket = self.pmap_socket;
        socket.set_port(port);

        let mut client = OncClient::new(socket);
        client.connect().await?;

        self.core_client = Some(Arc::new(Mutex::new(client)));

        Ok(())
    }

    async fn create_link(&mut self) -> Result<VxiClientLink> {
        let Some(onc_lock) = &self.core_client else {
            return Err(Error::Unspecified("Not connected".into()));
        };
        let mut onc = onc_lock.lock().await;

        let req = rpc::RpcRequestCreateDeviceLink {
            client_id: CLIENT_ID,
            lock_device: false,
            lock_timeout: LOCK_TIMEOUT,
            /* VXI11.3 B.1.2 */
            device: "inst0".into(),
        };

        let req = gen_call_packet(&onc, VxiPortType::Core, rpc::RpcRequest::CreateLink, req);
        let resp = onc.request(req).await?;

        let mut result = resp.get_success_result()?.to_vec();
        let result = rpc::RpcResponseCreateDeviceLink::unpack(&mut result)?;

        if result.error != rpc::RpcDeviceErrorCode::NoError {
            return Err(Error::Unspecified(format!(
                "Failed to create device link: {:?}",
                result.error
            )));
        }

        Ok(VxiClientLink::new(
            onc_lock.clone(),
            result.lid,
            result.max_recv_size,
        ))
    }
}

fn gen_call_packet(
    onc: &OncClient,
    ptype: VxiPortType,
    proc: rpc::RpcRequest,
    req: impl XdrPack,
) -> onc::RpcMessage {
    onc.gen_call_packet(ptype.get_prog(), ptype.get_vers(), proc as u32, req)
}

#[derive(Clone, Copy)]
#[allow(unused)]
enum VxiPortType {
    Core,
    Abort,
    Interrupt,
}
impl VxiPortType {
    fn get_prog(&self) -> u32 {
        match self {
            Self::Core => VXI_CORE_PROG,
            Self::Abort => VXI_ABORT_PROG,
            Self::Interrupt => VXI_INTERRUPT_PROG,
        }
    }

    fn get_vers(&self) -> u32 {
        match self {
            Self::Core => VXI_CORE_VERS,
            Self::Abort => VXI_ABORT_VERS,
            Self::Interrupt => VXI_INTERRUPT_VERS,
        }
    }
}
