pub mod drivers;
pub mod multimeter;
pub mod psu;
pub mod oscilloscope;

use multimeter::MultimeterEquipment;
use psu::PowerSupplyEquipment;
use oscilloscope::OscilloscopeEquipment;

pub enum Equipment {
    PowerSupply(Box<dyn PowerSupplyEquipment>),
    Multimeter(Box<dyn MultimeterEquipment>),
    Oscilloscope(Box<dyn OscilloscopeEquipment>),
}
