#![allow(unused_imports)]

use std::time::Duration;

use strum::IntoEnumIterator;
use testeq_rs::{
    equipment::{
        drivers::{multimeter_siglent::SiglentMultimeter, psu_rigol::RigolPsu},
        multimeter::{MultimeterEquipment, MultimeterMode},
        psu::PowerSupplyEquipment,
    },
    error::Result,
    protocol::{Protocol, ScpiTcpProtocol},
};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scpi = ScpiTcpProtocol::new("10.0.60.56:5025".parse()?).unwrap();
    scpi.connect().await?;
    /*let scpi: &mut dyn ScpiProtocol = &mut scpi;

    scpi.connect().await?;

    let idn = scpi.identify().await?;
    println!("Identity: {}", idn);

    let model = scpi.model().await?;
    println!("Model: {:?}", model);*/

    /*let mut psu = RigolPsu::new(Box::new(scpi))?;
    let psu: &mut dyn PowerSupplyEquipment = &mut psu;

    let info = psu.get_details().await?;
    println!("PSU details: {:?}", info);

    let mut chan0 = psu.get_channel(0).await?;

    println!("CH0 state: {}", chan0.get_enabled().await?);
    chan0.set_enabled(false).await?;
    println!("CH0 state: {}", chan0.get_enabled().await?);

    println!("CH0 voltage: {}", chan0.get_voltage().await?);
    println!("CH0 current: {}", chan0.get_current().await?);

    chan0.set_voltage(2.34567).await?;
    chan0.set_current(1.23456).await?;

    println!("CH0 voltage: {}", chan0.get_voltage().await?);
    println!("CH0 current: {}", chan0.get_current().await?);*/

    let mut dmm = SiglentMultimeter::new(Box::new(scpi))?;
    test_dmm(&mut dmm).await?;

    Ok(())
}

async fn test_dmm(dmm: &mut dyn MultimeterEquipment) -> Result<()> {
    let mut chans = dmm.get_channels().await?;
    for chan in &mut chans {
        println!("Testing channel {}", chan.name()?);
        for mode in MultimeterMode::iter() {
            /* TODO: Iterate ranges as well */
            if let Err(e) = chan.set_mode(mode, None).await {
                println!("Could not set mode {:?}: {}", mode, e);
                continue;
            }

            sleep(Duration::from_millis(50)).await;

            match chan.get_mode().await {
                Err(e) => {
                    println!("Could not get mode: {}", e);
                    continue;
                }
                Ok(rmode) => {
                    if rmode != mode {
                        println!(
                            "Reported mode does not match set: {:?} != {:?}",
                            rmode, mode
                        );
                        continue;
                    }
                }
            }

            match chan.get_reading().await {
                Err(e) => {
                    println!("Could not get reading in mode {:?}: {}", mode, e);
                    continue;
                }
                Ok(val) => println!("{:?} reading: {}", mode, val),
            }
        }
    }

    Ok(())
}
