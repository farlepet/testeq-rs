#![allow(unused_imports)]

use std::{env, net::ToSocketAddrs, process::exit, time::Duration};

use strum::IntoEnumIterator;
use testeq_rs::{
    equipment::{
        drivers::{
            multimeter_siglent::SiglentMultimeter, oscilloscope_siglent::SiglentOscilloscope,
            psu_rigol::RigolPsu,
        },
        equipment_from_scpi,
        multimeter::{MultimeterEquipment, MultimeterMode},
        oscilloscope::OscilloscopeEquipment,
        psu::PowerSupplyEquipment,
        Equipment,
    },
    error::Result,
    protocol::{Protocol, ScpiTcpProtocol},
};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: ... <protocol> <path>");
        println!("  <protocol>:");
        println!("    scpi_tcp: SCPI over raw TCP");
        println!("  <path>");
        println!("    Path to device, or network address (IP:PORT or HOSTNAME:PORT)");
        exit(1);
    }

    let proto = &args[1];
    let path = &args[2];

    let equip = match proto.as_ref() {
        "scpi_tcp" => {
            let Some(socket) = path.to_socket_addrs()?.next() else {
                println!("Could not resolve '{}'", path);
                exit(1);
            };
            let mut scpi = ScpiTcpProtocol::new(socket)?;
            scpi.connect().await?;
            equipment_from_scpi(Box::new(scpi)).await?
        }
        _ => {
            println!("Protocol '{}' not supported", proto);
            exit(1);
        }
    };

    match equip {
        Equipment::PowerSupply(mut psu) => test_psu(psu.as_mut()).await?,
        Equipment::Multimeter(mut dmm) => test_dmm(dmm.as_mut()).await?,
        Equipment::Oscilloscope(mut scope) => test_scope(scope.as_mut()).await?,
    }

    Ok(())
}

async fn test_psu(psu: &mut dyn PowerSupplyEquipment) -> Result<()> {
    psu.connect().await?;

    let mut chans = psu.get_channels().await?;
    for chan_mutex in &mut chans {
        let chan = chan_mutex.lock().await;

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
    dmm.connect().await?;

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

async fn test_scope(scope: &mut dyn OscilloscopeEquipment) -> Result<()> {
    scope.connect().await?;

    let mut chans = scope.get_channels().await?;
    for chan_mutex in &mut chans {
        let chan = chan_mutex.lock().await;

        println!("Testing channel {}", chan.name()?);
        println!("  enabled: {}", chan.get_enabled().await?);
    }

    println!("Global:");
    println!(
        "  trigger mode: {}",
        scope.get_trigger_mode().await?.as_ref()
    );
    println!(
        "  supported memory depths: {:?}",
        scope.get_memory_depths().await?
    );
    println!(
        "  current memory depth: {}",
        scope.get_memory_depth().await?
    );

    let capture = scope.read_capture().await?;
    println!("  capture:");
    for (name, chan) in capture.analog {
        println!("    {}: {} points", name, chan.readings.values.len());
    }
    Ok(())
}
