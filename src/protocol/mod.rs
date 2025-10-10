use async_trait::async_trait;

mod scpi;
mod scpi_tcp;
mod scpi_vxi11;

pub use scpi::ScpiProtocol;
pub use scpi_tcp::ScpiTcpProtocol;
pub use scpi_vxi11::ScpiVxi11Protocol;

use crate::{error::Result, model::ModelInfo};

#[async_trait]
pub trait Protocol: Send + Sync {
    async fn connect(&mut self) -> Result<()>;

    async fn disconnect(&mut self) -> Result<()>;

    async fn model(&mut self) -> Result<ModelInfo>;
}
