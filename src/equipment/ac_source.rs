use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    equipment::BaseEquipment,
    error::{Error, Result},
};

#[async_trait]
pub trait AcSourceEquipment: BaseEquipment {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn AcSourceChannel>>>;

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn AcSourceChannel>>>>;

    async fn trigger_now(&mut self) -> Result<()>;
}

#[async_trait]
pub trait AcSourceChannel: Send + Sync {
    fn name(&self) -> Result<String>;

    async fn read_voltage(&self) -> Result<AcSourceVoltageReadings>;

    async fn read_voltage_harmonic(&self, _num: u32) -> Result<AcSourceHarmonicVoltageReadings> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn read_current(&self) -> Result<AcSourceCurrentReadings>;

    async fn read_current_harmonic(&self, _num: u32) -> Result<AcSourceHarmonicCurrentReadings> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn read_power(&self) -> Result<AcSourcePowerReadings>;

    async fn read_frequency(&self) -> Result<f32>;
}

pub struct AcSourceVoltageReadings {
    pub dc: f32,
    pub ac_rms: f32,
}

pub struct AcSourceHarmonicVoltageReadings {
    pub ac_rms: f32,
    pub phase: f32,
    pub thd: f32,
}

pub struct AcSourceCurrentReadings {
    pub dc: f32,
    pub ac_rms: f32,
    pub max: f32,
}

pub struct AcSourceHarmonicCurrentReadings {
    pub ac_rms: f32,
    pub phase: f32,
    pub thd: f32,
}

pub struct AcSourcePowerReadings {
    pub dc: f32,
    pub real: f32,
    pub apparent: f32,
    pub reactive: f32,
    pub factor: f32,
}
