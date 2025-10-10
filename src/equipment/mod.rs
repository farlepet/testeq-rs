pub mod drivers;
pub mod multimeter;
pub mod psu;

use multimeter::MultimeterEquipment;
use psu::PowerSupplyEquipment;

pub enum Equipment {
    PowerSupply(Box<dyn PowerSupplyEquipment>),
    Multimeter(Box<dyn MultimeterEquipment>),
}
