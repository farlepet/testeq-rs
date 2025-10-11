use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    data::Readings,
    error::{Error, Result},
};

use super::BaseEquipment;

#[derive(Clone, Debug)]
pub struct AnalogWaveform {
    /// Time per point, in seconds
    pub time_per_pt: f64,
    /// Set of readings
    pub readings: Readings,
}

#[derive(Clone, Debug)]
pub struct DigitalWaveform {
    /// Time per point, in seconds
    pub time_per_pt: f64,
    /// Variable-length bitfield of readings, For each entry the LSB is the
    /// first reading
    pub readings: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct OscilloscopeCapture {
    pub analog: Vec<AnalogWaveform>,
    pub digital: Vec<DigitalWaveform>,
}

#[async_trait]
pub trait OscilloscopeEquipment: BaseEquipment {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn OscilloscopeChannel>>>;

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn OscilloscopeChannel>>>>;

    async fn get_digital_channel(
        &mut self,
        idx: u8,
    ) -> Result<Arc<Mutex<dyn OscilloscopeDigitalChannel>>> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_digital_channels(
        &mut self,
    ) -> Result<Vec<Arc<Mutex<dyn OscilloscopeDigitalChannel>>>> {
        Ok(vec![])
    }

    async fn read_capture(&mut self) -> Result<OscilloscopeCapture>;
}

#[async_trait]
pub trait OscilloscopeChannel: Send + Sync {
    fn name(&self) -> Result<String>;

    async fn read_waveform(&self) -> Result<AnalogWaveform>;
}

#[async_trait]
pub trait OscilloscopeDigitalChannel: Send + Sync {
    fn name(&self) -> Result<String>;
}
