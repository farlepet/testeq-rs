use std::{net::ToSocketAddrs, time::Duration};

use async_trait::async_trait;

use crate::{
    error::{Error, Result},
    model::ModelInfo,
    protocol,
};

use super::Protocol;

#[async_trait]
pub trait ScpiProtocol: Protocol + Send + Sync {
    async fn int_send(&mut self, data: &[u8]) -> Result<()>;

    async fn int_recv(&mut self) -> Result<Vec<u8>>;

    async fn int_query(&mut self, data: &[u8]) -> Result<Vec<u8>>;

    async fn recv_raw(
        &mut self,
        length: Option<usize>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>>;

    async fn recv_until(&mut self, byte: u8, timeout: Duration) -> Result<Vec<u8>>;

    async fn flush_rx(&mut self, timeout: Duration) -> Result<()>;
}
impl dyn ScpiProtocol {
    pub async fn send(&mut self, data: impl AsRef<[u8]>) -> Result<()> {
        let mut to_send = vec![];
        to_send.extend_from_slice(data.as_ref());
        to_send.extend_from_slice("\r\n".as_bytes());
        self.int_send(&to_send).await
    }

    pub async fn query(&mut self, data: impl AsRef<[u8]>) -> Result<Vec<u8>> {
        let mut to_send = vec![];
        to_send.extend_from_slice(data.as_ref());
        to_send.extend_from_slice("\r\n".as_bytes());
        self.int_query(&to_send).await
    }

    pub async fn query_str(&mut self, data: impl AsRef<[u8]>) -> Result<String> {
        Ok(String::from_utf8_lossy(&self.query(data).await?)
            .trim()
            .into())
    }

    pub async fn query_f32(&mut self, data: impl AsRef<[u8]>) -> Result<f32> {
        let res = self.query_str(data).await?;
        /* Some equipment may wrap values in quotes */
        let res = res.trim_start_matches('"').trim_end_matches('"');
        res.parse()
            .map_err(|e| Error::BadResponse(format!("Could not parse response `{res}`: {e}")))
    }

    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        self.int_recv().await
    }

    pub async fn identify(&mut self) -> Result<String> {
        self.query_str("*IDN?").await
    }

    pub async fn idn_model(&mut self) -> Result<ModelInfo> {
        let idn = self.identify().await?;

        ModelInfo::from_idn(&idn)
    }
}

pub async fn scpi_from_uri(uri: impl AsRef<str>) -> Result<Box<dyn ScpiProtocol>> {
    /* TODO: Centralize URI parsing */

    let uri = uri.as_ref();
    if let Some(socket) = uri.strip_prefix("vxi11://") {
        let socket = if socket.contains(':') {
            socket.to_string()
        } else {
            format!("{}:{}", socket, protocol::PORTMAP_PORT)
        };
        let Some(socket) = socket.to_socket_addrs()?.next() else {
            return Err(Error::Unspecified(format!("Could not resolve '{socket}'")));
        };

        let mut client = protocol::ScpiVxiProtocol::new(socket);
        client.connect().await?;

        Ok(Box::new(client))
    } else if let Some(socket) = uri.strip_prefix("tcp://") {
        let Some(socket) = socket.to_socket_addrs()?.next() else {
            return Err(Error::Unspecified(format!("Could not resolve '{socket}'")));
        };

        let mut scpi = protocol::ScpiTcpProtocol::new(socket)?;
        scpi.connect().await?;

        Ok(Box::new(scpi))
    } else if let Some(path) = uri.strip_prefix("serial:") {
        let (path, args) = match path.split_once('?') {
            Some((path, args)) => (path, args.split('&').collect()),
            None => (path, vec![]),
        };
        let mut baud = 9600;

        for arg in args {
            let Some((key, value)) = arg.split_once('=') else {
                return Err(Error::InvalidArgument(format!(
                    "Improperly formatted URI argument '{arg}'"
                )));
            };

            match key {
                "baud" => {
                    baud = value.parse().map_err(|_| {
                        Error::InvalidArgument(format!("Invalid value for baud rate: {value}"))
                    })?
                }
                _ => {
                    return Err(Error::InvalidArgument(format!(
                        "Unsupported argument '{key}' in URI"
                    )));
                }
            }
        }

        let mut client = protocol::ScpiSerialProtocol::new(path, baud);
        client.connect().await?;

        Ok(Box::new(client))
    } else {
        Err(Error::InvalidArgument(format!("Unknown scheme in '{uri}'")))
    }
}
