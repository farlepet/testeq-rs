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
    //let mut scpi = ScpiTcpProtocol::new("10.0.60.56:5025".parse()?).unwrap();
    let mut scpi = ScpiTcpProtocol::new("10.0.0.105:5555".parse()?).unwrap();
    scpi.connect().await?;

    let mut psu = RigolPsu::new(Box::new(scpi))?;
    psu.connect().await?;
    test_psu(&mut psu).await?;

    /*let mut dmm = SiglentMultimeter::new(Box::new(scpi))?;
    test_dmm(&mut dmm).await?;*/

    Ok(())
}

async fn test_psu(psu: &mut dyn PowerSupplyEquipment) -> Result<()> {
    let mut chans = psu.get_channels().await?;
    for chan_mutex in &mut chans {
        let mut chan = chan_mutex.lock().await;

        println!("Testing channel {}", chan.name()?);

        println!("  details: {:?}", chan.details()?);

        println!("  state: {}", chan.get_enabled().await?);
        println!("  set voltage:  {} V", chan.get_voltage().await?);
        println!("  set current:  {} A", chan.get_current().await?);
        println!("  read voltage: {} V", chan.read_voltage().await?);
        println!("  read current: {} A", chan.read_current().await?);
        println!("  read power:   {} W", chan.read_power().await?);
    }

    Ok(())
}

async fn test_dmm(dmm: &mut dyn MultimeterEquipment) -> Result<()> {
    let mut chans = dmm.get_channels().await?;
    for chan_mutex in &mut chans {
        let mut chan = chan_mutex.lock().await;

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
