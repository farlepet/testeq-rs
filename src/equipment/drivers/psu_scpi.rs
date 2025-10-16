use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    equipment::{
        BaseEquipment,
        psu::{PowerSupplyChannel, PowerSupplyChannelDetails, PowerSupplyEquipment},
    },
    error::{Error, Result},
    model::{Manufacturer, ModelInfo},
    protocol::ScpiProtocol,
};

pub struct GenericScpiPsu {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    model: Option<ModelInfo>,
    channels: Vec<Arc<Mutex<GenericScpiPsuChannel>>>,
}
impl GenericScpiPsu {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        Ok(Self {
            proto: Arc::new(Mutex::new(proto)),
            model: None,
            channels: vec![],
        })
    }
}
#[async_trait::async_trait]
impl BaseEquipment for GenericScpiPsu {
    async fn connect(&mut self) -> Result<()> {
        if !self.channels.is_empty() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        let model = self.proto.lock().await.model().await?;
        let psu_model = ScpiPsuModel::from_model(&model)?;
        self.model = Some(model);

        let details = psu_model.channel_details();

        self.channels = details
            .into_iter()
            .enumerate()
            .map(|(idx, ch)| {
                Arc::new(Mutex::new(GenericScpiPsuChannel::new(
                    self.proto.clone(),
                    idx as u8,
                    ch,
                    psu_model.get_proto(),
                )))
            })
            .collect();

        Ok(())
    }
}
#[async_trait::async_trait]
impl PowerSupplyEquipment for GenericScpiPsu {
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

struct GenericScpiPsuChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
    details: PowerSupplyChannelDetails,
    protocol: ScpiPsuProto,
}
impl GenericScpiPsuChannel {
    fn new(
        proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
        idx: u8,
        details: PowerSupplyChannelDetails,
        protocol: ScpiPsuProto,
    ) -> Self {
        Self {
            proto,
            idx,
            details,
            protocol,
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
            .map_err(|e| Error::BadResponse(format!("Could not parse response `{resp}`: {e}")))
    }
}
#[async_trait::async_trait]
impl PowerSupplyChannel for GenericScpiPsuChannel {
    fn name(&self) -> Result<String> {
        Ok(format!("CH{}", self.idx + 1))
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
        let cmd = match self.protocol {
            ScpiPsuProto::Rigol => format!(":SOUR{}:VOLT?", self.idx + 1),
            ScpiPsuProto::Siglent => format!(":SOUR:VOLT? CH{}", self.idx + 1),
        };
        self.query_f32(cmd).await
    }

    async fn set_voltage(&mut self, voltage: f32) -> Result<()> {
        let cmd = match self.protocol {
            ScpiPsuProto::Rigol => format!(":SOUR{}:VOLT {}", self.idx + 1, voltage),
            ScpiPsuProto::Siglent => format!(":SOUR:VOLT CH{},{}", self.idx + 1, voltage),
        };
        self.send(cmd).await
    }

    async fn get_current(&self) -> Result<f32> {
        let cmd = match self.protocol {
            ScpiPsuProto::Rigol => format!(":SOUR{}:CURR?", self.idx + 1),
            ScpiPsuProto::Siglent => format!(":SOUR:CURR? CH{}", self.idx + 1),
        };
        self.query_f32(cmd).await
    }

    async fn set_current(&mut self, current: f32) -> Result<()> {
        let cmd = match self.protocol {
            ScpiPsuProto::Rigol => format!(":SOUR{}:CURR {}", self.idx + 1, current),
            ScpiPsuProto::Siglent => format!(":SOUR:CURR CH{},{}", self.idx + 1, current),
        };
        self.send(cmd).await
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
        self.query_f32(format!(":MEAS:POWER? CH{}", self.idx + 1))
            .await
    }
}

enum ScpiPsuModel {
    /* Rigol DP700 series */
    RigolDP711,
    RigolDP712,
    /* Rigol DP800 series */
    RigolDP811,
    RigolDP813,
    RigolDP821,
    RigolDP822,
    RigolDP831,
    RigolDP832,
    /* Rigol DP900 series */
    RigolDP932,
    RigolDP932E,
    /* Rigol DP2000 series */
    RigolDP2031,
    /* Siglent SPD1000X series */
    SiglentSPD1168X,
    SiglentSPD1305X,
    /* Siglent SDP3000 series */
    SiglentSPD3303,
    /* Siglent SPD4000X series */
    SiglentSPD4121X,
    SiglentSPD4306X,
    SiglentSPD4323X,
}
impl ScpiPsuModel {
    fn channel_details(&self) -> Vec<PowerSupplyChannelDetails> {
        match self {
            Self::RigolDP711 => vec![PowerSupplyChannelDetails::new(0.0, 30.0, 5.0)],
            Self::RigolDP712 => vec![PowerSupplyChannelDetails::new(0.0, 50.0, 5.0)],
            Self::RigolDP811 => vec![PowerSupplyChannelDetails::new(0.0, 40.0, 10.0)],
            Self::RigolDP813 => vec![PowerSupplyChannelDetails::new(0.0, 20.0, 20.0)],
            Self::RigolDP821 => vec![
                PowerSupplyChannelDetails::new(0.0, 60.0, 1.0),
                PowerSupplyChannelDetails::new(0.0, 8.0, 10.0),
            ],
            Self::RigolDP822 => vec![
                PowerSupplyChannelDetails::new(0.0, 20.0, 5.0),
                PowerSupplyChannelDetails::new(0.0, 5.0, 16.0),
            ],
            Self::RigolDP831 => vec![
                PowerSupplyChannelDetails::new(0.0, 8.0, 5.0),
                PowerSupplyChannelDetails::new(0.0, 30.0, 2.0),
                PowerSupplyChannelDetails::new(-30.0, 0.0, 2.0),
            ],
            Self::RigolDP832 => vec![
                PowerSupplyChannelDetails::new(0.0, 30.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 30.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 5.0, 3.0),
            ],
            Self::RigolDP932 => vec![
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 6.0, 3.0),
            ],
            Self::RigolDP932E => vec![
                PowerSupplyChannelDetails::new(0.0, 30.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 30.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 6.0, 3.0),
            ],
            Self::RigolDP2031 => vec![
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.0),
                PowerSupplyChannelDetails::new(0.0, 6.0, 5.0),
            ],
            Self::SiglentSPD1168X => vec![PowerSupplyChannelDetails::new(0.0, 16.0, 8.0)],
            Self::SiglentSPD1305X => vec![PowerSupplyChannelDetails::new(0.0, 30.0, 5.0)],
            Self::SiglentSPD3303 => vec![
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.2),
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.2),
                /* CH3 cannot be controlled via SCPI */
                PowerSupplyChannelDetails::new(0.0, 0.0, 3.2),
            ],
            Self::SiglentSPD4121X => vec![
                PowerSupplyChannelDetails::new(0.0, 15.0, 1.5),
                PowerSupplyChannelDetails::new(0.0, 12.0, 10.0),
                PowerSupplyChannelDetails::new(0.0, 12.0, 10.0),
                PowerSupplyChannelDetails::new(0.0, 15.0, 1.5),
            ],
            Self::SiglentSPD4306X => vec![
                PowerSupplyChannelDetails::new(0.0, 15.0, 1.5),
                PowerSupplyChannelDetails::new(0.0, 30.0, 6.0),
                PowerSupplyChannelDetails::new(0.0, 30.0, 6.0),
                PowerSupplyChannelDetails::new(0.0, 15.0, 1.5),
            ],
            Self::SiglentSPD4323X => vec![
                PowerSupplyChannelDetails::new(0.0, 6.0, 3.2),
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.2),
                PowerSupplyChannelDetails::new(0.0, 32.0, 3.2),
                PowerSupplyChannelDetails::new(0.0, 6.0, 3.2),
            ],
        }
    }

    fn from_model(model: &ModelInfo) -> Result<Self> {
        let res = match &model.man_family {
            Manufacturer::Rigol(_) => {
                let mdl = &model.model;

                if mdl.starts_with("DP711") {
                    Some(Self::RigolDP711)
                } else if mdl.starts_with("DP712") {
                    Some(Self::RigolDP712)
                } else if mdl.starts_with("DP811") {
                    Some(Self::RigolDP811)
                } else if mdl.starts_with("DP813") {
                    Some(Self::RigolDP813)
                } else if mdl.starts_with("DP821") {
                    Some(Self::RigolDP821)
                } else if mdl.starts_with("DP822") {
                    Some(Self::RigolDP822)
                } else if mdl.starts_with("DP831") {
                    Some(Self::RigolDP831)
                } else if mdl.starts_with("DP832") {
                    Some(Self::RigolDP832)
                } else if mdl.starts_with("DP932E") {
                    Some(Self::RigolDP932E)
                } else if mdl.starts_with("DP932") {
                    Some(Self::RigolDP932)
                } else if mdl.starts_with("DP2031") {
                    Some(Self::RigolDP2031)
                } else {
                    None
                }
            }
            Manufacturer::Siglent(_) => {
                let mdl = &model.model;

                if mdl.starts_with("SPD1168") {
                    Some(Self::SiglentSPD1168X)
                } else if mdl.starts_with("SPD1305") {
                    Some(Self::SiglentSPD1305X)
                } else if mdl.starts_with("SPD3303") {
                    Some(Self::SiglentSPD3303)
                } else if mdl.starts_with("SPD4121") {
                    Some(Self::SiglentSPD4121X)
                } else if mdl.starts_with("SPD4306") {
                    Some(Self::SiglentSPD4306X)
                } else if mdl.starts_with("SPD4323") {
                    Some(Self::SiglentSPD4323X)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(res) = res {
            Ok(res)
        } else {
            Err(Error::NotSupported(format!("Model {model} not supported")))
        }
    }

    fn get_proto(&self) -> ScpiPsuProto {
        match self {
            Self::RigolDP711
            | Self::RigolDP712
            | Self::RigolDP811
            | Self::RigolDP813
            | Self::RigolDP821
            | Self::RigolDP822
            | Self::RigolDP831
            | Self::RigolDP832
            | Self::RigolDP932
            | Self::RigolDP932E
            | Self::RigolDP2031 => ScpiPsuProto::Rigol,
            Self::SiglentSPD1168X
            | Self::SiglentSPD1305X
            | Self::SiglentSPD3303
            | Self::SiglentSPD4121X
            | Self::SiglentSPD4306X
            | Self::SiglentSPD4323X => ScpiPsuProto::Siglent,
        }
    }
}

enum ScpiPsuProto {
    Rigol,
    Siglent,
}
