use async_trait::async_trait;

mod scpi;
mod scpi_serial;
mod scpi_tcp;
mod vxi11;

pub use scpi::ScpiProtocol;
pub use scpi_serial::ScpiSerialProtocol;
pub use scpi_tcp::ScpiTcpProtocol;
pub use vxi11::ScpiVxiProtocol;
pub use vxi11::portmap::PORTMAP_PORT;

use crate::{error::Result, model::ModelInfo};

#[async_trait]
pub trait Protocol: Send + Sync {
    async fn connect(&mut self) -> Result<()>;

    async fn disconnect(&mut self) -> Result<()>;

    async fn model(&mut self) -> Result<ModelInfo>;
}
