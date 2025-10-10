use async_trait::async_trait;

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct PowerSupplyDetails {
    pub channels: Vec<PowerSupplyChannelDetails>,
}

#[derive(Clone, Debug)]
pub struct PowerSupplyChannelDetails {
    /* TODO: Should we use an integer with millivolts instead? */
    pub min_voltage: f32,
    pub max_voltage: f32,
    pub max_current: f32,
}

#[async_trait]
pub trait PowerSupplyEquipment {
    async fn get_details(&mut self) -> Result<PowerSupplyDetails>;

    async fn get_channel(&mut self, idx: u8) -> Result<Box<dyn PowerSupplyChannel>>;

    async fn get_channels(&mut self) -> Result<Vec<Box<dyn PowerSupplyChannel>>>;
}

#[async_trait]
/* Don't warn about unused arguments for default implementations */
#[allow(unused_variables)]
pub trait PowerSupplyChannel: Send + Sync {
    async fn get_enabled(&self) -> Result<bool> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_voltage(&self) -> Result<f32> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn set_voltage(&mut self, voltage: f32) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_current(&self) -> Result<f32> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn set_current(&mut self, current: f32) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }
}
