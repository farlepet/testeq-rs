use testeq_rs::{
    equipment::{
        drivers::{multimeter_siglent::SiglentMultimeter, psu_rigol::RigolPsu},
        multimeter::{MultimeterEquipment, MultimeterMode},
        psu::PowerSupplyEquipment,
    },
    protocol::{Protocol, ScpiTcpProtocol},
};

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
    let dmm: &mut dyn MultimeterEquipment = &mut dmm;

    let mut chan = dmm.get_channel(0).await?;

    println!("Mode: {:?}", chan.get_mode().await?);
    chan.set_mode(MultimeterMode::Temperature, None).await?;
    println!("Mode: {:?}", chan.get_mode().await?);

    for _ in 0..4 {
        println!("Reading: {}", chan.get_reading().await?);
    }

    Ok(())
}
