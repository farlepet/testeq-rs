use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use crate::{
    data::Reading,
    equipment::{
        multimeter::{MultimeterChannel, MultimeterDetails, MultimeterEquipment, MultimeterMode},
        BaseEquipment,
    },
    error::{Error, Result},
    model::ModelInfo,
    protocol::ScpiProtocol,
};

#[allow(unused)]
pub struct SiglentMultimeter {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    model: Option<ModelInfo>,
    channels: Vec<Arc<Mutex<SiglentMultimeterChannel>>>,
}
impl SiglentMultimeter {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        let proto_arc = Arc::new(Mutex::new(proto));

        Ok(Self {
            /* TODO: Support scanner cards */
            channels: vec![Arc::new(Mutex::new(SiglentMultimeterChannel::new(
                proto_arc.clone(),
                0,
            )))],
            proto: proto_arc,
            model: None,
        })
    }
}
#[async_trait::async_trait]
impl BaseEquipment for SiglentMultimeter {
    async fn connect(&mut self) -> Result<()> {
        /* TODO */
        Ok(())
    }
}
#[async_trait::async_trait]
impl MultimeterEquipment for SiglentMultimeter {
    async fn get_details(&mut self) -> Result<MultimeterDetails> {
        Ok(MultimeterDetails {})
    }

    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn MultimeterChannel>>> {
        match self.channels.get(idx as usize) {
            None => Err(Error::Unspecified("Index out of range".into())),
            Some(chan) => Ok(chan.clone()),
        }
    }

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn MultimeterChannel>>>> {
        Ok(self
            .channels
            .clone()
            .into_iter()
            .map(|ch| ch as _)
            .collect())
    }
}

struct SiglentMultimeterChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
    /// Cached multimeter mode
    mode: Arc<RwLock<Option<MultimeterMode>>>,
}
impl SiglentMultimeterChannel {
    fn new(proto: Arc<Mutex<Box<dyn ScpiProtocol>>>, idx: u8) -> Self {
        Self {
            proto,
            idx,
            mode: Arc::new(RwLock::new(None)),
        }
    }

    async fn send(&self, cmd: &str) -> Result<()> {
        self.proto.lock().await.send(cmd).await
    }

    async fn query_str(&self, cmd: &str) -> Result<String> {
        let resp = self.proto.lock().await.query(cmd).await?;
        let resp = String::from_utf8_lossy(&resp);
        Ok(resp
            .trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string())
    }

    async fn get_mode_or_cache(&self) -> Result<MultimeterMode> {
        match *self.mode.read().await {
            Some(mode) => Ok(mode),
            None => {
                let mode = self.get_mode().await?;
                *self.mode.write().await = Some(mode);
                Ok(mode)
            }
        }
    }
}
#[async_trait::async_trait]
impl MultimeterChannel for SiglentMultimeterChannel {
    fn name(&self) -> Result<String> {
        if self.idx == 0 {
            Ok("Main".to_string())
        } else {
            /* Scanner card channel */
            Ok(format!("CH{}", self.idx))
        }
    }

    async fn set_mode(&mut self, mode: MultimeterMode, _range: Option<u8>) -> Result<()> {
        match mode {
            MultimeterMode::DcVoltage => {
                /* TODO: Support setting range */
                self.send("CONF:VOLT:DC").await?;
            }
            MultimeterMode::AcVoltage => {
                /* TODO: Support setting range */
                self.send("CONF:VOLT:AC").await?;
            }
            MultimeterMode::DcCurrent => {
                /* TODO: Support setting range */
                self.send("CONF:CURR:DC").await?;
            }
            MultimeterMode::AcCurrent => {
                /* TODO: Support setting range */
                self.send("CONF:CURR:AC").await?;
            }
            MultimeterMode::Continuity => {
                self.send("CONF:CONT").await?;
            }
            MultimeterMode::Diode => {
                self.send("CONF:DIOD").await?;
            }
            MultimeterMode::Frequency => {
                self.send("CONF:FREQ").await?;
            }
            MultimeterMode::Period => {
                self.send("CONF:PER").await?;
            }
            MultimeterMode::Temperature => {
                /* TODO: Support setting probe type */
                self.send("CONF:TEMP THER,NITS90").await?;
            }
            MultimeterMode::Resistance => {
                /* TODO: Support setting range */
                self.send("CONF:RES").await?;
            }
            MultimeterMode::Resistance4W => {
                /* TODO: Support setting range */
                self.send("CONF:FRES").await?;
            }
            MultimeterMode::Capacitance => {
                /* TODO: Support setting range */
                self.send("CONF:CAP").await?;
            }
            _ => {
                return Err(Error::NotSupported(format!(
                    "Mode {:?} not supported",
                    mode
                )))
            }
        }

        *self.mode.write().await = Some(mode);

        Ok(())
    }
    async fn get_modes(&self) -> Result<Vec<MultimeterMode>> {
        Ok(vec![
            MultimeterMode::Continuity,
            MultimeterMode::Diode,
            MultimeterMode::Frequency,
            MultimeterMode::Period,
            MultimeterMode::Temperature,
            MultimeterMode::Resistance,
            MultimeterMode::Capacitance,
            MultimeterMode::Resistance4W,
            MultimeterMode::DcVoltage,
            MultimeterMode::AcVoltage,
            MultimeterMode::DcCurrent,
            MultimeterMode::AcCurrent,
        ])
    }

    async fn get_mode(&self) -> Result<MultimeterMode> {
        let resp = self.query_str("CONF?").await?;
        let resp_vec: Vec<_> = resp.split(' ').collect();

        let Some(mode) = resp_vec.first() else {
            return Err(Error::BadResponse(format!("Malformed response: {}", resp)));
        };

        let mode = match *mode {
            "VOLT" => MultimeterMode::DcVoltage,
            "VOLT:AC" => MultimeterMode::AcVoltage,
            "CURR" => MultimeterMode::DcCurrent,
            "CURR:AC" => MultimeterMode::AcCurrent,
            "CONT" => MultimeterMode::Continuity,
            "DIOD" => MultimeterMode::Diode,
            "FREQ" => MultimeterMode::Frequency,
            "PER" => MultimeterMode::Period,
            "TEMP" => MultimeterMode::Temperature,
            "RES" => MultimeterMode::Resistance,
            "FRES" => MultimeterMode::Resistance4W,
            "CAP" => MultimeterMode::Capacitance,
            _ => {
                return Err(Error::BadResponse(format!("Unknown mode: {}", mode)));
            }
        };
        //self.mode = Some(mode);

        Ok(mode)
    }

    async fn get_reading(&self) -> Result<Reading> {
        let mode = self.get_mode_or_cache().await?;

        let resp = self.query_str("READ?").await?;
        let resp_vec: Vec<_> = resp.split(',').collect();

        let Some(reading) = resp_vec.first() else {
            return Err(Error::BadResponse(format!("Malformed response: {}", resp)));
        };

        let mut reading: f64 = reading.parse().map_err(|e| {
            Error::BadResponse(format!("Could not parse {} as f64: {}", reading, e))
        })?;
        if reading > 1e37 {
            /* Overload */
            reading = f64::NAN;
        }

        Ok(Reading {
            unit: mode.into(),
            value: reading,
        })
    }
}
