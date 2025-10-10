use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    equipment::psu::{
        PowerSupplyChannel, PowerSupplyChannelDetails, PowerSupplyDetails, PowerSupplyEquipment,
    },
    error::{Error, Result},
    model::ModelInfo,
    protocol::ScpiProtocol,
};

pub struct RigolPsu {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    model: Option<ModelInfo>,
    n_channels: u8,
}
impl RigolPsu {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        Ok(Self {
            proto: Arc::new(Mutex::new(proto)),
            model: None,
            n_channels: 0,
        })
    }
}
#[async_trait::async_trait]
impl PowerSupplyEquipment for RigolPsu {
    async fn get_details(&mut self) -> Result<PowerSupplyDetails> {
        let model = match &self.model {
            Some(model) => model,
            None => {
                self.model = Some(self.proto.lock().await.model().await?);
                self.model.as_ref().unwrap()
            }
        };

        let channels = match model.model.as_str() {
            "DP832" => vec![
                PowerSupplyChannelDetails {
                    min_voltage: 0.010,
                    max_voltage: 33.0,
                    max_current: 3.3,
                },
                PowerSupplyChannelDetails {
                    min_voltage: 0.010,
                    max_voltage: 33.0,
                    max_current: 3.3,
                },
                PowerSupplyChannelDetails {
                    min_voltage: 0.010,
                    max_voltage: 5.5,
                    max_current: 3.3,
                },
            ],
            _ => vec![],
        };

        self.n_channels = channels.len() as u8;

        Ok(PowerSupplyDetails { channels })
    }

    async fn get_channel(&mut self, idx: u8) -> Result<Box<dyn PowerSupplyChannel>> {
        if idx >= self.n_channels {
            return Err(Error::Unspecified("Index out of range".into()));
        }

        Ok(Box::new(RigolPsuChannel::new(self.proto.clone(), idx)))
    }

    async fn get_channels(&mut self) -> Result<Vec<Box<dyn PowerSupplyChannel>>> {
        let mut channels = vec![];
        for i in 0..self.n_channels {
            channels.push(self.get_channel(i).await?);
        }
        Ok(channels)
    }
}

struct RigolPsuChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
}
impl RigolPsuChannel {
    fn new(proto: Arc<Mutex<Box<dyn ScpiProtocol>>>, idx: u8) -> Self {
        Self { proto, idx }
    }
}
#[async_trait::async_trait]
impl PowerSupplyChannel for RigolPsuChannel {
    async fn get_enabled(&self) -> Result<bool> {
        let resp = self
            .proto
            .lock()
            .await
            .query(format!(":OUTP? CH{}", self.idx + 1))
            .await?;
        let resp = String::from_utf8_lossy(&resp);

        if resp.starts_with("ON") || resp.starts_with('1') {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        let mode = if enabled { "ON" } else { "OFF" };
        self.proto
            .lock()
            .await
            .send(format!(":OUTP CH{},{}", self.idx + 1, mode))
            .await
    }

    async fn get_voltage(&self) -> Result<f32> {
        let resp = self
            .proto
            .lock()
            .await
            .query(format!(":SOUR{}:VOLT?", self.idx + 1))
            .await?;
        let resp = String::from_utf8_lossy(&resp).to_string();
        resp.trim()
            .parse()
            .map_err(|e| Error::BadResponse(format!("Could not parse response `{}`: {}", resp, e)))
    }

    async fn set_voltage(&mut self, voltage: f32) -> Result<()> {
        self.proto
            .lock()
            .await
            .send(format!(":SOUR{}:VOLT {}", self.idx + 1, voltage))
            .await
    }

    async fn get_current(&self) -> Result<f32> {
        let resp = self
            .proto
            .lock()
            .await
            .query(format!(":SOUR{}:CURR?", self.idx + 1))
            .await?;
        let resp = String::from_utf8_lossy(&resp).to_string();
        resp.trim()
            .parse()
            .map_err(|e| Error::BadResponse(format!("Could not parse response `{}`: {}", resp, e)))
    }

    async fn set_current(&mut self, current: f32) -> Result<()> {
        self.proto
            .lock()
            .await
            .send(format!(":SOUR{}:CURR {}", self.idx + 1, current))
            .await
    }
}
