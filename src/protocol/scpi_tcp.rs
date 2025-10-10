use std::net::SocketAddr;

use async_trait::async_trait;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpSocket, TcpStream},
};

use crate::{
    error::{Error, Result},
    model::ModelInfo,
};

use super::{Protocol, ScpiProtocol};

pub struct ScpiTcpProtocol {
    socket: SocketAddr,
    stream: Option<TcpStream>,
}
impl ScpiTcpProtocol {
    pub fn new(socket: SocketAddr) -> Result<Self> {
        Ok(Self {
            socket,
            stream: None,
        })
    }
}
#[async_trait]
impl Protocol for ScpiTcpProtocol {
    async fn connect(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        /* TODO: Support IPv6 */
        let socket = TcpSocket::new_v4().map_err(|e| Error::Unhandled(e.into()))?;
        self.stream = Some(
            socket
                .connect(self.socket)
                .await
                .map_err(|e| Error::Unhandled(e.into()))?,
        );
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.stream.take();
        Ok(())
    }

    async fn model(&mut self) -> Result<ModelInfo> {
        (self as &mut dyn ScpiProtocol).idn_model().await
    }
}
#[async_trait]
impl ScpiProtocol for ScpiTcpProtocol {
    async fn int_send(&mut self, data: &[u8]) -> Result<()> {
        let Some(stream) = &mut self.stream else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        stream
            .write_all(data)
            .await
            .map_err(|e| Error::Unhandled(e.into()))?;

        Ok(())
    }

    async fn int_recv(&mut self) -> Result<Vec<u8>> {
        let Some(stream) = &mut self.stream else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        let mut resp = vec![];
        let mut stream = BufReader::new(stream);
        /* TODO: Timeout */
        stream
            .read_until(b'\n', &mut resp)
            .await
            .map_err(|e| Error::Unhandled(e.into()))?;

        Ok(resp)
    }

    async fn int_query(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.int_send(data).await?;
        self.int_recv().await
    }
}
