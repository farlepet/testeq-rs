use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::error::{Error, Result};

use super::BaseEquipment;

#[derive(Clone, Debug)]
pub struct PowerSupplyChannelDetails {
    /* TODO: Should we use an integer with millivolts instead? */
    pub min_voltage: f32,
    pub max_voltage: f32,
    pub max_current: f32,
}
impl PowerSupplyChannelDetails {
    pub fn new(min_v: f32, max_v: f32, max_c: f32) -> Self {
        Self {
            min_voltage: min_v,
            max_voltage: max_v,
            max_current: max_c,
        }
    }
}

#[async_trait]
pub trait PowerSupplyEquipment: BaseEquipment {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn PowerSupplyChannel>>>;

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn PowerSupplyChannel>>>>;
}

#[async_trait]
/* Don't warn about unused arguments for default implementations */
#[allow(unused_variables)]
pub trait PowerSupplyChannel: Send + Sync {
    fn details(&self) -> Result<PowerSupplyChannelDetails>;

    fn name(&self) -> Result<String>;

    /// Read channel enabled state
    async fn get_enabled(&self) -> Result<bool> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Set channel enabled state
    async fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Read channel set voltage
    async fn get_voltage(&self) -> Result<f32> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Set channel set voltage
    async fn set_voltage(&mut self, voltage: f32) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Read channel set current
    async fn get_current(&self) -> Result<f32> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Set channel set current
    async fn set_current(&mut self, current: f32) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Read channel readback voltage
    async fn read_voltage(&self) -> Result<f32> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Read channel readback current
    async fn read_current(&self) -> Result<f32> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    /// Read channel readback power
    async fn read_power(&self) -> Result<f32> {
        /* If driver does not directly support reading power, derive from
         * voltage and current */
        Ok(self.read_voltage().await? * self.read_current().await?)
    }
}
