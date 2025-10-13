use std::time::Duration;

use async_trait::async_trait;

use crate::{
    error::{Error, Result},
    model::ModelInfo,
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
            .map_err(|e| Error::BadResponse(format!("Could not parse response `{}`: {}", res, e)))
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
