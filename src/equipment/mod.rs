pub mod ac_source;
pub mod drivers;
pub mod multimeter;
pub mod oscilloscope;
pub mod psu;
pub mod spectrum_analyzer;

use async_trait::async_trait;

use ac_source::AcSourceEquipment;
use multimeter::MultimeterEquipment;
use oscilloscope::OscilloscopeEquipment;
use psu::PowerSupplyEquipment;
use spectrum_analyzer::SpectrumAnalyzerEquipment;

use crate::{
    error::{Error, Result},
    model::{KeysightFamily, Manufacturer, RigolFamily, SiglentFamily},
    protocol::ScpiProtocol,
};

use self::drivers::{
    ac_source_keysight::KeysightAcSource, multimeter_siglent::SiglentMultimeter,
    oscilloscope_siglent::SiglentOscilloscope, psu_scpi::GenericScpiPsu,
    sa_siglent::SiglentSpectrumAnalyzer,
};

pub enum Equipment {
    AcSource(Box<dyn AcSourceEquipment>),
    PowerSupply(Box<dyn PowerSupplyEquipment>),
    Multimeter(Box<dyn MultimeterEquipment>),
    Oscilloscope(Box<dyn OscilloscopeEquipment>),
    SpectrumAnalyzer(Box<dyn SpectrumAnalyzerEquipment>),
}

pub async fn equipment_from_scpi(mut proto: Box<dyn ScpiProtocol>) -> Result<Equipment> {
    let model = proto.idn_model().await?;

    #[allow(clippy::collapsible_match)]
    match &model.man_family {
        Manufacturer::Keysight(family) => {
            if matches!(family, KeysightFamily::_6800) {
                return Ok(Equipment::AcSource(Box::new(KeysightAcSource::new(proto)?)));
            }
        }
        Manufacturer::Rigol(family) => match family {
            RigolFamily::DP700 | RigolFamily::DP800 | RigolFamily::DP900 | RigolFamily::DP2000 => {
                return Ok(Equipment::PowerSupply(Box::new(GenericScpiPsu::new(
                    proto,
                )?)));
            }
            _ => {}
        },
        Manufacturer::Siglent(family) => match family {
            SiglentFamily::SDM4000A => {
                return Ok(Equipment::Multimeter(Box::new(SiglentMultimeter::new(
                    proto,
                )?)));
            }
            SiglentFamily::SDS3000X => {
                return Ok(Equipment::Oscilloscope(Box::new(SiglentOscilloscope::new(
                    proto,
                )?)));
            }
            SiglentFamily::SSA3000X => {
                return Ok(Equipment::SpectrumAnalyzer(Box::new(
                    SiglentSpectrumAnalyzer::new(proto)?,
                )));
            }
            SiglentFamily::SPD1000X | SiglentFamily::SPD3000 | SiglentFamily::SPD4000X => {
                return Ok(Equipment::PowerSupply(Box::new(GenericScpiPsu::new(
                    proto,
                )?)));
            }
            _ => {}
        },
        _ => {}
    }

    Err(Error::NotSupported(format!("No driver matching {model:?}")))
}

#[async_trait]
pub trait BaseEquipment: Sync + Send {
    async fn connect(&mut self) -> Result<()>;
}
