use std::fmt::Display;

use strum_macros::EnumIter;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
pub enum Unit {
    /// Unitless
    None,
    /// Voltage - volts
    Voltage,
    /// Current - amps
    Current,
    /// Resistance - ohms
    Resistance,
    /// Temperature - degrees celsius
    Temperature,
    /// Frequenct - hertz
    Frequency,
    /// Period - seconds
    Period,
    /// Capacitance - farads
    Capacitance,
    /// Inductance - henries
    Inductance,
}
impl Unit {
    fn unit_abbrev(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Voltage => "V",
            Self::Current => "A",
            Self::Resistance => "Ω",
            Self::Temperature => "°C",
            Self::Frequency => "Hz",
            Self::Period => "s",
            Self::Capacitance => "F",
            Self::Inductance => "H",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Reading {
    pub unit: Unit,
    pub value: f64,
}
impl Display for Reading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.value.is_nan() {
            write!(f, "OVERLOAD {}", self.unit.unit_abbrev())
        } else {
            let (prefix, value) = get_prefix_and_scale(self.value);

            write!(f, "{} {}{}", value, prefix, self.unit.unit_abbrev())
        }
    }
}

#[derive(Clone, Debug)]
pub struct Readings {
    pub unit: Unit,
    pub values: Vec<f64>,
}
