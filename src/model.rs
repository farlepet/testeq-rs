use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct ModelInfo {
    /// Manufacturer and family
    pub man_family: Manufacturer,
    /// Manufacturer string
    pub manufacturer: String,
    /// Model string
    pub model: String,
    /// Serial number
    pub serial: Option<String>,
    /// Version number (as reported by *IDN?)
    pub version: Option<String>,
}
impl ModelInfo {
    pub fn from_idn(idn: &str) -> Result<Self> {
        let idn_sep: Vec<_> = idn.split(',').collect();
        if idn_sep.len() < 2 {
            return Err(Error::Unspecified(format!("Invalid *IDN? response: {idn}")));
        }

        Ok(Self {
            man_family: Manufacturer::from_idn(&idn_sep)?,
            manufacturer: idn_sep[0].to_string(),
            model: idn_sep[1].to_string(),
            serial: idn_sep.get(2).map(|s| s.to_string()),
            version: idn_sep.get(3).map(|s| s.to_string()),
        })
    }
}

#[derive(Clone, Debug)]
pub enum Manufacturer {
    /// Unknown manufacturer
    Unknown,
    /// LeCroy, also includes Teledyne Lecroy
    LeCroy(LecroyFamily),
    /// Rigol
    Rigol(RigolFamily),
    /// Siglent
    Siglent(SiglentFamily),
    /// Spirent
    Spirent(SpirentFamily),
    /// Keysight, also includes Agilent and HP
    Keysight(KeysightFamily),
}
impl Manufacturer {
    fn from_idn(idn: &[&str]) -> Result<Self> {
        let man = idn[0].to_lowercase();

        if man.contains("rigol") {
            Ok(Self::Rigol(RigolFamily::from_idn(idn)?))
        } else if man.contains("siglent") {
            Ok(Self::Siglent(SiglentFamily::from_idn(idn)?))
        } else {
            Ok(Self::Unknown)
        }
    }
}

#[derive(Clone, Debug)]
pub enum LecroyFamily {
    /// Lecroy WavePro 7000 series oscilloscope
    WavePro7000,
}

#[derive(Clone, Debug)]
pub enum RigolFamily {
    Unknown,
    /// Rigol DS1200 series oscilloscope
    DS1200,
    /// Rigol DP800 series power supply
    DP800,
    /// Rigol DP2000 series power supply
    DP2000,
}
impl RigolFamily {
    fn from_idn(idn: &[&str]) -> Result<Self> {
        let model = idn[1].to_lowercase();

        if model.contains("ds12") {
            Ok(Self::DS1200)
        } else if model.contains("dp8") {
            Ok(Self::DP800)
        } else if model.contains("dp2") {
            Ok(Self::DP2000)
        } else {
            Ok(Self::Unknown)
        }
    }
}

#[derive(Clone, Debug)]
pub enum SiglentFamily {
    Unknown,
    /// Siglent SDS3000X series oscilloscope
    SDS3000X,
    /// Siglent SSA3000X Plus series spectrum analyzer
    SSA3000XPlus,
    /// Siglent SDM4000A series multimeter
    SDM4000A,
    /// Siglent SDG3000X series function generator
    SDG3000X,
}
impl SiglentFamily {
    fn from_idn(idn: &[&str]) -> Result<Self> {
        let model = idn[1].to_lowercase();

        if model.contains("sds3") {
            Ok(Self::SDS3000X)
        } else if model.contains("ssa3") {
            /* TODO: Differentiate non-plus, if necessary */
            Ok(Self::SSA3000XPlus)
        } else if model.contains("sdm4") {
            Ok(Self::SDM4000A)
        } else if model.contains("sdg3") {
            Ok(Self::SDG3000X)
        } else {
            Ok(Self::Unknown)
        }
    }
}

#[derive(Clone, Debug)]
pub enum SpirentFamily {
    /// Spirent GSS6300 Multi-GNSS generator
    GSS6300,
}

#[derive(Clone, Debug)]
pub enum KeysightFamily {
    /* TODO: How to deal with leading numbers? */
    /// Agilent 86130A bit error rate tester
    _86130A,
    /// HP/Agilent/Keysight 6800-series AC source/analyzer
    _6800,
}
