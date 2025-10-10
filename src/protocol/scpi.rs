use async_trait::async_trait;

use crate::{error::Result, model::ModelInfo};

use super::Protocol;

#[async_trait]
pub trait ScpiProtocol: Protocol + Send + Sync {
    async fn int_send(&mut self, data: &[u8]) -> Result<()>;

    async fn int_recv(&mut self) -> Result<Vec<u8>>;

    async fn int_query(&mut self, data: &[u8]) -> Result<Vec<u8>>;
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

    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        self.int_recv().await
    }

    pub async fn identify(&mut self) -> Result<String> {
        let res = self.query("*IDN?").await?;

        Ok(String::from_utf8_lossy(&res).into())
    }

    pub async fn idn_model(&mut self) -> Result<ModelInfo> {
        let idn = self.identify().await?;

        ModelInfo::from_idn(&idn)
    }
}
