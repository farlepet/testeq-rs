pub mod ac_source;
pub mod drivers;
pub mod multimeter;
pub mod oscilloscope;
pub mod psu;
pub mod spectrum_analyzer;

use std::net::ToSocketAddrs;

use async_trait::async_trait;

use ac_source::AcSourceEquipment;
use multimeter::MultimeterEquipment;
use oscilloscope::OscilloscopeEquipment;
use psu::PowerSupplyEquipment;
use spectrum_analyzer::SpectrumAnalyzerEquipment;

use crate::{
    error::{Error, Result},
    model::{KeysightFamily, Manufacturer, RigolFamily, SiglentFamily},
    protocol::{self, Protocol, ScpiProtocol},
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

pub async fn equipment_from_uri(uri: impl AsRef<str>) -> Result<Equipment> {
    /* TODO: Centralize URI parsing */

    let uri = uri.as_ref();
    if let Some(socket) = uri.strip_prefix("vxi11://") {
        let socket = if socket.contains(':') {
            socket.to_string()
        } else {
            format!("{}:{}", socket, protocol::PORTMAP_PORT)
        };
        let Some(socket) = socket.to_socket_addrs()?.next() else {
            return Err(Error::Unspecified(format!("Could not resolve '{socket}'")));
        };

        let mut client = protocol::ScpiVxiProtocol::new(socket);
        client.connect().await?;
        equipment_from_scpi(Box::new(client)).await
    } else if let Some(socket) = uri.strip_prefix("tcp://") {
        let Some(socket) = socket.to_socket_addrs()?.next() else {
            return Err(Error::Unspecified(format!("Could not resolve '{socket}'")));
        };

        let mut scpi = protocol::ScpiTcpProtocol::new(socket)?;
        scpi.connect().await?;
        equipment_from_scpi(Box::new(scpi)).await
    } else if let Some(path) = uri.strip_prefix("serial:") {
        let (path, args) = match path.split_once('?') {
            Some((path, args)) => (path, args.split('&').collect()),
            None => (path, vec![]),
        };
        let mut baud = 9600;

        for arg in args {
            let Some((key, value)) = arg.split_once('=') else {
                return Err(Error::InvalidArgument(format!(
                    "Improperly formatted URI argument '{arg}'"
                )));
            };

            match key {
                "baud" => {
                    baud = value.parse().map_err(|_| {
                        Error::InvalidArgument(format!("Invalid value for baud rate: {value}"))
                    })?
                }
                _ => {
                    return Err(Error::InvalidArgument(format!(
                        "Unsupported argument '{key}' in URI"
                    )));
                }
            }
        }

        let mut client = protocol::ScpiSerialProtocol::new(path, baud);
        client.connect().await?;
        equipment_from_scpi(Box::new(client)).await
    } else {
        Err(Error::InvalidArgument(format!("Unknown scheme in '{uri}'")))
    }
}

#[async_trait]
pub trait BaseEquipment: Sync + Send {
    async fn connect(&mut self) -> Result<()>;
}
