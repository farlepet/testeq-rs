use std::fmt::Display;

use async_trait::async_trait;

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct MultimeterDetails {}

#[async_trait]
pub trait MultimeterEquipment {
    async fn get_details(&mut self) -> Result<MultimeterDetails>;

    async fn get_channel(&mut self, idx: u8) -> Result<Box<dyn MultimeterChannel>>;

    async fn get_channels(&mut self) -> Result<Vec<Box<dyn MultimeterChannel>>>;
}

/* TODO: Move elsewhere */
fn get_prefix_and_scale(val: f64) -> (&'static str, f64) {
    let aval = val.abs();
    if aval < 1e-12 {
        ("f", val / 1e-15)
    } else if aval < 1e-9 {
        ("p", val / 1e-12)
    } else if aval < 1e-6 {
        ("n", val / 1e-9)
    } else if aval < 1e-3 {
        ("u", val / 1e-6)
    } else if aval < 1e0 {
        ("m", val / 1e-3)
    } else if aval < 1e3 {
        ("", val)
    } else if aval < 1e6 {
        ("k", val / 1e3)
    } else if aval < 1e9 {
        ("M", val / 1e6)
    } else if aval < 1e12 {
        ("G", val / 1e9)
    } else {
        ("T", val / 1e12)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MultimeterReading {
    Voltage(f64),
    Current(f64),
    Resistance(f64),
    Temperature(f64),
    Frequency(f64),
    Period(f64),
    Capacitance(f64),
    Inductance(f64),
}
impl MultimeterReading {
    pub fn from_val_and_mode(value: f64, mode: MultimeterMode) -> Self {
        match mode {
            MultimeterMode::DcVoltage | MultimeterMode::AcVoltage | MultimeterMode::Diode => {
                Self::Voltage(value)
            }
            MultimeterMode::DcCurrent | MultimeterMode::AcCurrent => Self::Current(value),
            MultimeterMode::Resistance
            | MultimeterMode::Resistance4W
            | MultimeterMode::Continuity => Self::Resistance(value),
            MultimeterMode::Temperature => Self::Temperature(value),
            MultimeterMode::Frequency => Self::Frequency(value),
            MultimeterMode::Period => Self::Period(value),
            MultimeterMode::Capacitance => Self::Capacitance(value),
            MultimeterMode::Inductance => Self::Inductance(value),
        }
    }

    pub fn unit(&self) -> &str {
        match self {
            Self::Voltage(_) => "V",
            Self::Current(_) => "A",
            Self::Resistance(_) => "Ohms",
            Self::Temperature(_) => "degC",
            Self::Frequency(_) => "Hz",
            Self::Period(_) => "s",
            Self::Capacitance(_) => "F",
            Self::Inductance(_) => "H",
        }
    }

    pub fn value(&self) -> f64 {
        match self {
            Self::Voltage(value)
            | Self::Current(value)
            | Self::Resistance(value)
            | Self::Temperature(value)
            | Self::Frequency(value)
            | Self::Period(value)
            | Self::Capacitance(value)
            | Self::Inductance(value) => *value,
        }
    }
}
impl Display for MultimeterReading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unit = self.unit();
        let (prefix, value) = get_prefix_and_scale(self.value());

        write!(f, "{} {}{}", value, prefix, unit)
    }
}

#[derive(Clone, Copy, Debug)]
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
    async fn get_reading(&self) -> Result<MultimeterReading> {
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
