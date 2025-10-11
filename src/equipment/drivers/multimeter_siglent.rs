use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    data::Reading,
    equipment::multimeter::{
        MultimeterChannel, MultimeterDetails, MultimeterEquipment, MultimeterMode,
    },
    error::{Error, Result},
    model::ModelInfo,
    protocol::ScpiProtocol,
};

pub struct SiglentMultimeter {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    model: Option<ModelInfo>,
    n_channels: u8,
}
impl SiglentMultimeter {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        Ok(Self {
            proto: Arc::new(Mutex::new(proto)),
            model: None,
            n_channels: 1,
        })
    }
}
#[async_trait::async_trait]
impl MultimeterEquipment for SiglentMultimeter {
    async fn get_details(&mut self) -> Result<MultimeterDetails> {
        Ok(MultimeterDetails {})
    }

    async fn get_channel(&mut self, idx: u8) -> Result<Box<dyn MultimeterChannel>> {
        if idx >= self.n_channels {
            return Err(Error::Unspecified("Index out of range".into()));
        }

        Ok(Box::new(SiglentMultimeterChannel::new(
            self.proto.clone(),
            idx,
        )))
    }

    async fn get_channels(&mut self) -> Result<Vec<Box<dyn MultimeterChannel>>> {
        let mut channels = vec![];
        for i in 0..self.n_channels {
            channels.push(self.get_channel(i).await?);
        }
        Ok(channels)
    }
}

struct SiglentMultimeterChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
    /// Cached multimeter mode
    mode: Option<MultimeterMode>,
}
impl SiglentMultimeterChannel {
    fn new(proto: Arc<Mutex<Box<dyn ScpiProtocol>>>, idx: u8) -> Self {
        Self {
            proto,
            idx,
            mode: None,
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
        match self.mode {
            Some(mode) => Ok(mode),
            None => self.get_mode().await,
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

    async fn set_mode(&mut self, mode: MultimeterMode, range: Option<u8>) -> Result<()> {
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

        self.mode = Some(mode);

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
