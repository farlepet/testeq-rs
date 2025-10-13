use std::{fmt::Display, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    data::{Reading, Readings, Unit},
    error::Result,
};

use super::BaseEquipment;

#[async_trait]
pub trait SpectrumAnalyzerEquipment: BaseEquipment {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn SpectrumAnalyzerChannel>>>;

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn SpectrumAnalyzerChannel>>>>;
}

#[async_trait]
pub trait SpectrumAnalyzerChannel {
    fn name(&self) -> Result<String>;

    async fn get_frequency_conf(&self) -> Result<SpectrumAnalyzerFreqConfig>;

    async fn set_frequency_conf(&self, conf: SpectrumAnalyzerFreqConfig) -> Result<()>;

    /* TODO: Should this be per-channel, or a single call for the instrument? */
    async fn read_trace(&self, idx: u8) -> Result<SpectrumTrace>;
}

pub struct SpectrumAnalyzerFreqConfig {
    pub span: SpectrumAnalyzerSpan,
    pub resolution: f32,
}
impl Display for SpectrumAnalyzerFreqConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = Reading {
            unit: Unit::Frequency,
            value: self.resolution as f64,
        };

        write!(f, "span: {}, res: {}", self.span, res)
    }
}

pub enum SpectrumAnalyzerSpan {
    StartStop { start: f32, stop: f32 },
    CenterSpan { center: f32, span: f32 },
}
impl SpectrumAnalyzerSpan {
    pub fn start(&self) -> f32 {
        match self {
            Self::StartStop { start, stop: _ } => *start,
            Self::CenterSpan { center, span } => center - (span / 2.0),
        }
    }

    pub fn stop(&self) -> f32 {
        match self {
            Self::StartStop { start: _, stop } => *stop,
            Self::CenterSpan { center, span } => center + (span / 2.0),
        }
    }

    pub fn center(&self) -> f32 {
        match self {
            Self::StartStop { start, stop } => (stop + start) / 2.0,
            Self::CenterSpan { center, span: _ } => *center,
        }
    }

    pub fn span(&self) -> f32 {
        match self {
            Self::StartStop { start, stop } => stop - start,
            Self::CenterSpan { center: _, span } => *span,
        }
    }
}
impl Display for SpectrumAnalyzerSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.span() < (self.center() * 0.01) {
            /* When span is small relative to the center frequency, prefer
             * center +/- span representation */
            let center = Reading {
                unit: Unit::Frequency,
                value: self.center() as f64,
            };
            let span = Reading {
                unit: Unit::Frequency,
                value: self.span() as f64,
            };
            write!(f, "{center} ± {span}")
        } else {
            let start = Reading {
                unit: Unit::Frequency,
                value: self.start() as f64,
            };
            let stop = Reading {
                unit: Unit::Frequency,
                value: self.stop() as f64,
            };
            write!(f, "{start} - {stop}")
        }
    }
}

pub struct SpectrumTrace {
    pub span: SpectrumAnalyzerSpan,
    pub freq_step: f32,
    pub readings: Readings,
}
