use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    equipment::{
        psu::{PowerSupplyChannel, PowerSupplyChannelDetails, PowerSupplyEquipment},
        BaseEquipment,
    },
    error::{Error, Result},
    model::ModelInfo,
    protocol::ScpiProtocol,
};

pub struct RigolPsu {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    model: Option<ModelInfo>,
    channels: Vec<Arc<Mutex<RigolPsuChannel>>>,
}
impl RigolPsu {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        Ok(Self {
            proto: Arc::new(Mutex::new(proto)),
            model: None,
            channels: vec![],
        })
    }

    fn create_channels(
        model: &ModelInfo,
        proto: &Arc<Mutex<Box<dyn ScpiProtocol>>>,
    ) -> Vec<Arc<Mutex<RigolPsuChannel>>> {
        let details = Self::create_details(model);

        details
            .into_iter()
            .enumerate()
            .map(|(idx, ch)| {
                Arc::new(Mutex::new(RigolPsuChannel::new(
                    proto.clone(),
                    idx as u8,
                    ch,
                )))
            })
            .collect()
    }

    fn create_details(model: &ModelInfo) -> Vec<PowerSupplyChannelDetails> {
        match model.model.as_str() {
            "DP832" => vec![
                PowerSupplyChannelDetails {
                    min_voltage: 0.0,
                    max_voltage: 33.0,
                    max_current: 3.3,
                },
                PowerSupplyChannelDetails {
                    min_voltage: 0.0,
                    max_voltage: 33.0,
                    max_current: 3.3,
                },
                PowerSupplyChannelDetails {
                    min_voltage: 0.0,
                    max_voltage: 5.5,
                    max_current: 3.3,
                },
            ],
            _ => vec![],
        }
    }
}
#[async_trait::async_trait]
impl BaseEquipment for RigolPsu {
    async fn connect(&mut self) -> Result<()> {
        if !self.channels.is_empty() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        self.model = Some(self.proto.lock().await.model().await?);
        self.channels = Self::create_channels(self.model.as_ref().unwrap(), &self.proto);

        Ok(())
    }
}
#[async_trait::async_trait]
impl PowerSupplyEquipment for RigolPsu {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn PowerSupplyChannel>>> {
        match self.channels.get(idx as usize) {
            None => Err(Error::Unspecified("Index out of range".into())),
            Some(chan) => Ok(chan.clone()),
        }
    }

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn PowerSupplyChannel>>>> {
        Ok(self
            .channels
            .clone()
            .into_iter()
            .map(|ch| ch as _)
            .collect())
    }
}

struct RigolPsuChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
    details: PowerSupplyChannelDetails,
}
impl RigolPsuChannel {
    fn new(
        proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
        idx: u8,
        details: PowerSupplyChannelDetails,
    ) -> Self {
        Self {
            proto,
            idx,
            details,
        }
    }

    async fn send(&self, cmd: impl AsRef<[u8]>) -> Result<()> {
        self.proto.lock().await.send(cmd).await
    }

    async fn query_str(&self, cmd: impl AsRef<[u8]>) -> Result<String> {
        let resp = self.proto.lock().await.query(cmd).await?;
        let resp = String::from_utf8_lossy(&resp);
        Ok(resp
            .trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string())
    }

    async fn query_f32(&self, cmd: impl AsRef<[u8]>) -> Result<f32> {
        let resp = self.query_str(cmd).await?;
        resp.parse()
            .map_err(|e| Error::BadResponse(format!("Could not parse response `{}`: {}", resp, e)))
    }
}
#[async_trait::async_trait]
impl PowerSupplyChannel for RigolPsuChannel {
    fn name(&self) -> Result<String> {
        Ok(format!("CH{}", self.idx))
    }

    fn details(&self) -> Result<PowerSupplyChannelDetails> {
        Ok(self.details.clone())
    }

    async fn get_enabled(&self) -> Result<bool> {
        let resp = self.query_str(format!(":OUTP? CH{}", self.idx + 1)).await?;

        if resp.starts_with("ON") || resp.starts_with('1') {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        let mode = if enabled { "ON" } else { "OFF" };
        self.send(format!(":OUTP CH{},{}", self.idx + 1, mode))
            .await
    }

    async fn get_voltage(&self) -> Result<f32> {
        self.query_f32(format!(":SOUR{}:VOLT?", self.idx + 1)).await
    }

    async fn set_voltage(&mut self, voltage: f32) -> Result<()> {
        self.send(format!(":SOUR{}:VOLT {}", self.idx + 1, voltage))
            .await
    }

    async fn get_current(&self) -> Result<f32> {
        self.query_f32(format!(":SOUR{}:CURR?", self.idx + 1)).await
    }

    async fn set_current(&mut self, current: f32) -> Result<()> {
        self.send(format!(":SOUR{}:CURR {}", self.idx + 1, current))
            .await
    }

    async fn read_voltage(&self) -> Result<f32> {
        self.query_f32(format!(":MEAS:VOLT? CH{}", self.idx + 1))
            .await
    }

    async fn read_current(&self) -> Result<f32> {
        self.query_f32(format!(":MEAS:CURR? CH{}", self.idx + 1))
            .await
    }

    async fn read_power(&self) -> Result<f32> {
        self.query_f32(format!(":MEAS:POWE? CH{}", self.idx + 1))
            .await
    }
}
