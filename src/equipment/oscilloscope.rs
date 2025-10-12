use std::{collections::HashMap, sync::Arc};

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

#[derive(Clone, Debug, Default)]
pub struct OscilloscopeCapture {
    pub analog: HashMap<String, AnalogWaveform>,
    pub digital: HashMap<String, DigitalWaveform>,
}

#[async_trait]
#[allow(unused_variables)]
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

    async fn get_memory_depths(&self) -> Result<Vec<u64>>;

    async fn get_memory_depth(&self) -> Result<u64>;

    async fn set_memory_depth(&self, depth: u64) -> Result<()>;

    async fn get_trigger_mode(&self) -> Result<scope_trig::TriggerMode>;

    async fn set_trigger_mode(&mut self, mode: scope_trig::TriggerMode) -> Result<()>;

    async fn trigger_now(&mut self) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn read_capture(&mut self) -> Result<OscilloscopeCapture>;
}

#[async_trait]
pub trait OscilloscopeChannel: Send + Sync {
    fn name(&self) -> Result<String>;

    async fn read_waveform(&self) -> Result<AnalogWaveform>;

    async fn get_enabled(&self) -> Result<bool>;

    async fn set_enabled(&mut self, enabled: bool) -> Result<()>;
}

#[async_trait]
pub trait OscilloscopeDigitalChannel: Send + Sync {
    fn name(&self) -> Result<String>;
}

pub mod scope_trig {
    use strum_macros::AsRefStr;

    #[derive(Copy, Clone, AsRefStr, Debug)]
    pub enum TriggerMode {
        Auto,
        Normal,
        Single,
    }

    #[derive(Copy, Clone, AsRefStr, Debug)]
    pub enum TriggerType {
        Edge,
        Slope,
        Pulse,
        Video,
        Window,
        Interval,
        Dropout,
        Runt,
        Pattern,
        Qualified,
        NthEdge,
        Delay,
        SetupHold,
        Decode(TriggerDecodeProtocol),
    }

    #[derive(Copy, Clone, AsRefStr, Debug)]
    pub enum TriggerDecodeProtocol {
        I2C,
        Spi,
        Uart,
        Can,
        Lin,
        FlexRay,
        CanFd,
        I2S,
        Sent,
    }

    #[derive(Copy, Clone, AsRefStr, Debug)]
    pub enum TriggerCoupling {
        Dc,
        Ac,
        HfReject,
        LfReject,
    }

    #[derive(Copy, Clone, AsRefStr, Debug)]
    pub enum TriggerHoldoff {
        None,
        Time(f64),
        Events(u64),
    }

    #[derive(Copy, Clone, AsRefStr, Debug)]
    pub enum TriggerEdge {
        Rising,
        Falling,
        Alternating,
    }

    #[derive(Copy, Clone, AsRefStr, Debug)]
    pub enum TriggerSource {
        Analog(u8),
        Digital(u8),
        External(u8),
        ExternalDiv(u8, u8),
        Line,
    }

    #[derive(Clone, Debug)]
    pub struct TriggerConfig {
        pub coupling: Option<TriggerCoupling>,
        pub holdoff: Option<TriggerHoldoff>,
        pub source: Option<TriggerSource>,
        pub ttype: TriggerTypeConfig,
    }

    #[derive(Clone, Debug)]
    pub enum TriggerTypeConfig {
        Edge(TriggerTypeEdgeConfig),
    }

    #[derive(Clone, Debug)]
    pub struct TriggerTypeEdgeConfig {
        pub level: f64,
        pub edge: TriggerEdge,
    }
}
