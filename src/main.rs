use std::{env, process::exit, time::Duration};

use strum::IntoEnumIterator;
use testeq_rs::{
    data::{Reading, Unit},
    equipment::{
        Equipment,
        ac_source::AcSourceEquipment,
        equipment_from_uri,
        multimeter::{MultimeterEquipment, MultimeterMode},
        oscilloscope::OscilloscopeEquipment,
        psu::PowerSupplyEquipment,
        spectrum_analyzer::SpectrumAnalyzerEquipment,
    },
    error::Result,
};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: ... <uri>");
        println!("  <uri>:");
        println!("    tcp://<host>:<port>: SCPI over raw TCP");
        println!("    vxi11://<host>[:<port>]: SCPI over raw VXI11");
        println!("    serial:<port>[?baud=<baud>]: SCPI over serial");
        exit(1);
    }

    let uri = &args[1];

    let equip = equipment_from_uri(uri).await?;

    match equip {
        Equipment::AcSource(mut ac) => test_ac_source(ac.as_mut()).await?,
        Equipment::PowerSupply(mut psu) => test_psu(psu.as_mut()).await?,
        Equipment::Multimeter(mut dmm) => test_dmm(dmm.as_mut()).await?,
        Equipment::Oscilloscope(mut scope) => test_scope(scope.as_mut()).await?,
        Equipment::SpectrumAnalyzer(mut sa) => test_sa(sa.as_mut()).await?,
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
                println!("Could not set mode {mode:?}: {e}");
                continue;
            }

            sleep(Duration::from_millis(50)).await;

            match chan.get_mode().await {
                Err(e) => {
                    println!("Could not get mode: {e}");
                    continue;
                }
                Ok(rmode) => {
                    if rmode != mode {
                        println!("Reported mode does not match set: {rmode:?} != {mode:?}");
                        continue;
                    }
                }
            }

            match chan.get_reading().await {
                Err(e) => {
                    println!("Could not get reading in mode {mode:?}: {e}");
                    continue;
                }
                Ok(val) => println!("{mode:?} reading: {val}"),
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

async fn test_sa(sa: &mut dyn SpectrumAnalyzerEquipment) -> Result<()> {
    sa.connect().await?;

    let mut chans = sa.get_channels().await?;
    for chan_mutex in &mut chans {
        let chan = chan_mutex.lock().await;

        println!("Testing channel {}", chan.name()?);
        let freq_cfg = chan.get_frequency_conf().await?;
        println!("  Span: {}", freq_cfg.span);
        println!(
            "  Resolution: {}",
            Reading::new(Unit::Frequency, freq_cfg.resolution as f64)
        );

        let trace = chan.read_trace(0).await?;
        println!("  Received {} points", trace.readings.values.len());
        println!("    Frequency: {}", trace.span);
        println!(
            "    Frequency step: {}",
            Reading::new(Unit::Frequency, trace.freq_step as f64)
        );

        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut max_freq = 0.0;
        let mut avg = 0.0;
        for (idx, point) in trace.readings.values.iter().enumerate() {
            if *point < min {
                min = *point;
            }
            if *point > max {
                max = *point;
                max_freq = trace.span.start() + trace.freq_step * idx as f32;
            }
            avg += point;
        }
        avg /= trace.readings.values.len() as f64;

        println!("    Min: {}", Reading::new(trace.readings.unit, min));
        println!(
            "    Max: {} @ {}",
            Reading::new(trace.readings.unit, max),
            Reading::new(Unit::Frequency, max_freq as f64)
        );
        println!("    Average: {}", Reading::new(trace.readings.unit, avg));
    }
    Ok(())
}

async fn test_ac_source(ac: &mut dyn AcSourceEquipment) -> Result<()> {
    ac.connect().await?;

    ac.trigger_now().await?;

    let mut chans = ac.get_channels().await?;
    for chan_mutex in &mut chans {
        let chan = chan_mutex.lock().await;

        println!("Testing channel {}", chan.name()?);
        let volt = chan.read_voltage().await?;
        println!("  Voltage:");
        println!("    DC: {}", Reading::new(Unit::Voltage, volt.dc as f64));
        println!(
            "    AC: {} RMS",
            Reading::new(Unit::Voltage, volt.ac_rms as f64)
        );
        let curr = chan.read_current().await?;
        println!("  Current:");
        println!("    DC: {}", Reading::new(Unit::Current, curr.dc as f64));
        println!(
            "    AC: {} RMS",
            Reading::new(Unit::Current, curr.ac_rms as f64)
        );
        println!("    MAX: {}", Reading::new(Unit::Current, curr.max as f64));
        let pow = chan.read_power().await?;
        println!("  Power:");
        println!("    DC: {}", Reading::new(Unit::Power, pow.dc as f64));
        println!(
            "    AC real: {}",
            Reading::new(Unit::Power, pow.real as f64)
        );
        println!(
            "    AC apparent: {}",
            Reading::new(Unit::Power, pow.apparent as f64)
        );
        println!(
            "    AC reactive: {}",
            Reading::new(Unit::Power, pow.reactive as f64)
        );
        println!("    Power factor: {}", pow.factor);
        let freq = chan.read_frequency().await?;
        println!(
            "  Frequency: {}",
            Reading::new(Unit::Frequency, freq as f64)
        );
    }

    Ok(())
}
