use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    data::{Readings, Unit},
    equipment::{
        BaseEquipment,
        spectrum_analyzer::{
            SpectrumAnalyzerChannel, SpectrumAnalyzerEquipment, SpectrumAnalyzerFreqConfig,
            SpectrumAnalyzerSpan, SpectrumTrace,
        },
    },
    error::{Error, Result},
    model::ModelInfo,
    protocol::ScpiProtocol,
};

pub struct SiglentSpectrumAnalyzer {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    model: Option<ModelInfo>,
    channels: Vec<Arc<Mutex<SiglentSaChannel>>>,
}
impl SiglentSpectrumAnalyzer {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        let proto_arc = Arc::new(Mutex::new(proto));

        Ok(Self {
            channels: vec![],
            proto: proto_arc,
            model: None,
        })
    }
}
#[async_trait]
impl BaseEquipment for SiglentSpectrumAnalyzer {
    async fn connect(&mut self) -> Result<()> {
        if !self.channels.is_empty() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        let model = self.proto.lock().await.model().await?;
        self.model = Some(model);

        self.channels
            .push(Arc::new(Mutex::new(SiglentSaChannel::new(
                self.proto.clone(),
            ))));

        Ok(())
    }
}
#[async_trait]
impl SpectrumAnalyzerEquipment for SiglentSpectrumAnalyzer {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn SpectrumAnalyzerChannel>>> {
        match self.channels.get(idx as usize) {
            None => Err(Error::Unspecified("Index out of range".into())),
            Some(chan) => Ok(chan.clone()),
        }
    }

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn SpectrumAnalyzerChannel>>>> {
        Ok(self
            .channels
            .clone()
            .into_iter()
            .map(|ch| ch as _)
            .collect())
    }
}

struct SiglentSaChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
}
impl SiglentSaChannel {
    fn new(proto: Arc<Mutex<Box<dyn ScpiProtocol>>>) -> Self {
        Self { proto }
    }
}
#[async_trait]
impl SpectrumAnalyzerChannel for SiglentSaChannel {
    fn name(&self) -> Result<String> {
        Ok("main".to_string())
    }

    async fn get_frequency_conf(&self) -> Result<SpectrumAnalyzerFreqConfig> {
        let mut proto = self.proto.lock().await;

        Ok(SpectrumAnalyzerFreqConfig {
            span: SpectrumAnalyzerSpan::CenterSpan {
                center: proto.query_f32(":FREQ:CENT?").await?,
                span: proto.query_f32(":FREQ:SPAN?").await?,
            },
            resolution: proto.query_f32(":BWID?").await?,
        })
    }

    async fn set_frequency_conf(&self, conf: SpectrumAnalyzerFreqConfig) -> Result<()> {
        let mut proto = self.proto.lock().await;

        /* TODO: Set using whatever is specified in th enum to get best precision */
        proto
            .send(format!(":FREQ:CENT {}", conf.span.center()))
            .await?;
        proto
            .send(format!(":FREQ:SPAN {}", conf.span.span()))
            .await?;
        proto.send(format!(":BWID {}", conf.resolution)).await
    }

    async fn read_trace(&self, idx: u8) -> Result<SpectrumTrace> {
        if idx >= 4 {
            return Err(Error::Unspecified("Index out of range".into()));
        }

        let freq = self.get_frequency_conf().await?;

        let mut proto = self.proto.lock().await;
        /* Get current unit */
        let unit = proto.query_str(":UNIT:POW?").await?;
        let unit = SiglentSaUnit::from_query(&unit)?;

        let mut trace = SpectrumTrace {
            span: freq.span,
            freq_step: 0.,
            readings: Readings {
                unit: unit.unit(),
                values: vec![],
            },
        };

        /* Use 64-bit floating-point numbers */
        proto.send(":FORM REAL,64").await?;
        /* Request data */
        proto.send(format!(":TRAC? {}", idx + 1)).await?;
        let data = proto.recv().await?;

        /* Data is a block of 64-bit floating-point values, followed by a newline */
        for chunk in data.chunks(8) {
            if chunk.len() == 8 {
                let value = f64::from_le_bytes(chunk.try_into().unwrap());
                trace.readings.values.push(unit.normalize(value));
            }
        }

        trace.freq_step = trace.span.span() / (trace.readings.values.len() as f32);

        Ok(trace)
    }
}

enum SiglentSaUnit {
    LogMilliWatts,
    LogMilliVolts,
    LogMicroVolts,
    LogMicroAmps,
    Volts,
    Watts,
}
impl SiglentSaUnit {
    fn from_query(value: &str) -> Result<Self> {
        match value {
            "DBM" => Ok(Self::LogMilliWatts),
            "DBMV" => Ok(Self::LogMilliVolts),
            "DBUV" => Ok(Self::LogMicroVolts),
            "DBUA" => Ok(Self::LogMicroAmps),
            "V" => Ok(Self::Volts),
            "W" => Ok(Self::Watts),
            _ => Err(Error::BadResponse(format!(
                "Unknown reported unit type '{value}'"
            ))),
        }
    }

    fn normalize(&self, value: f64) -> f64 {
        match self {
            Self::LogMilliWatts
            | Self::LogMilliVolts
            | Self::LogMicroAmps
            | Self::Volts
            | Self::Watts => value,
            Self::LogMicroVolts => value - 60.0,
        }
    }

    fn unit(&self) -> Unit {
        match self {
            Self::LogMilliWatts => Unit::LogPower,
            Self::LogMicroVolts | Self::LogMilliVolts => Unit::LogVoltage,
            Self::LogMicroAmps => Unit::LogCurrent,
            Self::Volts => Unit::Voltage,
            Self::Watts => Unit::Power,
        }
    }
}
