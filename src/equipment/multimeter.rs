use std::{fmt::Display, str::FromStr, sync::Arc};

use async_trait::async_trait;
use strum_macros::EnumIter;
use tokio::sync::Mutex;

use crate::{
    data::{Reading, Unit},
    error::{Error, Result},
};

use super::BaseEquipment;

#[derive(Clone, Debug)]
pub struct MultimeterDetails {}

#[async_trait]
pub trait MultimeterEquipment: BaseEquipment {
    async fn get_details(&mut self) -> Result<MultimeterDetails>;

    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn MultimeterChannel>>>;

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn MultimeterChannel>>>>;

    /// Arm trigger
    async fn trigger_arm(&mut self) -> Result<()>;

    /// Trigger immediately, may only be valid when trigger source is set to Bus
    async fn trigger_now(&mut self) -> Result<()>;

    async fn get_trigger_source(&mut self) -> Result<MultimeterTrigSource>;

    async fn set_trigger_source(&mut self, source: MultimeterTrigSource) -> Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
pub enum MultimeterMode {
    DcVoltage,
    AcVoltage,
    DcCurrent,
    AcCurrent,
    Resistance,
    Resistance4W,
    Continuity,
    Diode,
    Temperature,
    Frequency,
    Period,
    Capacitance,
    Inductance,
}
impl Display for MultimeterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::DcVoltage => "dcv",
            Self::AcVoltage => "acv",
            Self::DcCurrent => "dci",
            Self::AcCurrent => "aci",
            Self::Resistance => "res",
            Self::Resistance4W => "res4w",
            Self::Continuity => "cont",
            Self::Diode => "diode",
            Self::Temperature => "temp",
            Self::Frequency => "freq",
            Self::Period => "per",
            Self::Capacitance => "cap",
            Self::Inductance => "ind",
        };
        write!(f, "{val}")
    }
}
impl FromStr for MultimeterMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "dcv" => Self::DcVoltage,
            "acv" => Self::AcVoltage,
            "dci" => Self::DcCurrent,
            "aci" => Self::AcCurrent,
            "res" => Self::Resistance,
            "res4w" => Self::Resistance4W,
            "cont" => Self::Continuity,
            "diode" => Self::Diode,
            "temp" => Self::Temperature,
            "freq" => Self::Frequency,
            "per" => Self::Period,
            "cap" => Self::Capacitance,
            "ind" => Self::Inductance,
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "{s} is not a valid multimeter mode string"
                )));
            }
        })
    }
}
impl From<MultimeterMode> for Unit {
    fn from(value: MultimeterMode) -> Self {
        match value {
            MultimeterMode::DcVoltage | MultimeterMode::AcVoltage | MultimeterMode::Diode => {
                Self::Voltage
            }
            MultimeterMode::DcCurrent | MultimeterMode::AcCurrent => Self::Current,
            MultimeterMode::Resistance
            | MultimeterMode::Resistance4W
            | MultimeterMode::Continuity => Self::Resistance,
            MultimeterMode::Temperature => Self::Temperature,
            MultimeterMode::Frequency => Self::Frequency,
            MultimeterMode::Period => Self::Period,
            MultimeterMode::Capacitance => Self::Capacitance,
            MultimeterMode::Inductance => Self::Inductance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MultimeterRange {
    /// ID of range, used when setting range
    pub id: u8,
    pub name: String,
    pub min_value: f64,
    pub max_value: f64,
}

#[async_trait]
/* Don't warn about unused arguments for default implementations */
#[allow(unused_variables)]
pub trait MultimeterChannel: Send + Sync {
    fn name(&self) -> Result<String>;

    async fn get_reading(&self) -> Result<Reading> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_modes(&self) -> Result<Vec<MultimeterMode>> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_mode(&self) -> Result<MultimeterMode> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn set_mode(&mut self, mode: MultimeterMode, range: Option<u8>) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_ranges(&self, mode: MultimeterMode) -> Result<Vec<MultimeterRange>> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn get_range(&self) -> Result<MultimeterRange> {
        Err(Error::Unimplemented("Not implemented".into()))
    }

    async fn set_range(&mut self, range: u8) -> Result<()> {
        Err(Error::Unimplemented("Not implemented".into()))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MultimeterTrigSource {
    /// Always trigger immediately
    Immediate,
    /// Trigger on bus command
    Bus,
    /// Trigger from external trigger input
    External(u8),
}
impl Display for MultimeterTrigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate => write!(f, "imm"),
            Self::Bus => write!(f, "bus"),
            Self::External(n) => write!(f, "ext{n}"),
        }
    }
}
impl FromStr for MultimeterTrigSource {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "imm" => Self::Immediate,
            "bus" => Self::Bus,
            "ext" => Self::External(0),
            _ if s.starts_with("ext") => {
                let num = &s[3..];
                match num.parse() {
                    Ok(n) => Self::External(n),
                    Err(_) => {
                        return Err(Error::InvalidArgument(format!(
                            "Suffix on '{s}' cannot be parsed as u8"
                        )));
                    }
                }
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "{s} is not a valid trigger source string"
                )));
            }
        })
    }
}
