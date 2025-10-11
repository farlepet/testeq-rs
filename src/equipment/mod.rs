pub mod drivers;
pub mod multimeter;
pub mod oscilloscope;
pub mod psu;

use async_trait::async_trait;
use multimeter::MultimeterEquipment;
use oscilloscope::OscilloscopeEquipment;
use psu::PowerSupplyEquipment;

use crate::{
    error::{Error, Result},
    model::{Manufacturer, RigolFamily, SiglentFamily},
    protocol::ScpiProtocol,
};

use self::drivers::{
    multimeter_siglent::SiglentMultimeter, oscilloscope_siglent::SiglentOscilloscope,
    psu_rigol::RigolPsu,
};

pub enum Equipment {
    PowerSupply(Box<dyn PowerSupplyEquipment>),
    Multimeter(Box<dyn MultimeterEquipment>),
    Oscilloscope(Box<dyn OscilloscopeEquipment>),
}

pub async fn equipment_from_scpi(mut proto: Box<dyn ScpiProtocol>) -> Result<Equipment> {
    let model = proto.idn_model().await?;

    match &model.man_family {
        Manufacturer::Rigol(family) => match family {
            RigolFamily::DP800 | RigolFamily::DP2000 => {
                return Ok(Equipment::PowerSupply(Box::new(RigolPsu::new(proto)?)))
            }
            _ => {}
        },
        Manufacturer::Siglent(family) => match family {
            SiglentFamily::SDS3000X => {
                return Ok(Equipment::Oscilloscope(Box::new(SiglentOscilloscope::new(
                    proto,
                )?)))
            }
            SiglentFamily::SDM4000A => {
                return Ok(Equipment::Multimeter(Box::new(SiglentMultimeter::new(
                    proto,
                )?)))
            }
            _ => {}
        },
        _ => {}
    }

    Err(Error::NotSupported(format!(
        "No driver matching {:?}",
        model
    )))
}

#[async_trait]
pub trait BaseEquipment: Sync + Send {
    async fn connect(&mut self) -> Result<()>;
}
