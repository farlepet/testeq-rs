use async_trait::async_trait;

use crate::{error::Result, model::ModelInfo};

use super::{Protocol, ScpiProtocol};

pub struct ScpiVxi11Protocol {}
#[async_trait]
impl Protocol for ScpiVxi11Protocol {
    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn model(&mut self) -> Result<ModelInfo> {
        (self as &mut dyn ScpiProtocol).idn_model().await
    }
}
#[async_trait]
impl ScpiProtocol for ScpiVxi11Protocol {
    async fn int_send(&mut self, data: &[u8]) -> Result<()> {
        Ok(())
    }

    async fn int_recv(&mut self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn int_query(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(vec![])
    }
}
