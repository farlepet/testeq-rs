use std::sync::Arc;

use async_trait::async_trait;
use strum_macros::EnumIter;
use tokio::sync::Mutex;

use crate::{
    data::{Reading, Unit},
    error::{Error, Result},
};

#[derive(Clone, Debug)]
pub struct MultimeterDetails {}

#[async_trait]
pub trait MultimeterEquipment {
    async fn get_details(&mut self) -> Result<MultimeterDetails>;

    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn MultimeterChannel>>>;

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn MultimeterChannel>>>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
pub enum MultimeterMode {
    DcVoltage,
    AcVoltage,
    DcCurrent,
    AcCurrent,
    Resistance,
    Resistance4W,
    Continuity,
    Diode,
    Temperature,
    Frequency,
    Period,
    Capacitance,
    Inductance,
}
impl From<MultimeterMode> for Unit {
    fn from(value: MultimeterMode) -> Self {
        match value {
            MultimeterMode::DcVoltage | MultimeterMode::AcVoltage | MultimeterMode::Diode => {
                Self::Voltage
            }
            MultimeterMode::DcCurrent | MultimeterMode::AcCurrent => Self::Current,
            MultimeterMode::Resistance
            | MultimeterMode::Resistance4W
            | MultimeterMode::Continuity => Self::Resistance,
            MultimeterMode::Temperature => Self::Temperature,
            MultimeterMode::Frequency => Self::Frequency,
            MultimeterMode::Period => Self::Period,
            MultimeterMode::Capacitance => Self::Capacitance,
            MultimeterMode::Inductance => Self::Inductance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MultimeterRange {
    /// ID of range, used when setting range
    pub id: u8,
    pub name: String,
    pub min_value: f64,
    pub max_value: f64,
}

#[async_trait]
/* Don't warn about unused arguments for default implementations */
#[allow(unused_variables)]
pub trait MultimeterChannel: Send + Sync {
    fn name(&self) -> Result<String>;

    async fn get_reading(&self) -> Result<Reading> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_modes(&self) -> Result<Vec<MultimeterMode>> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_mode(&self) -> Result<MultimeterMode> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn set_mode(&mut self, mode: MultimeterMode, range: Option<u8>) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_ranges(&self, mode: MultimeterMode) -> Result<Vec<MultimeterRange>> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_range(&self) -> Result<MultimeterRange> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn set_range(&mut self, range: u8) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }
}
