use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    equipment::{
        BaseEquipment,
        ac_source::{
            AcSourceChannel, AcSourceCurrentReadings, AcSourceEquipment, AcSourcePowerReadings,
            AcSourceVoltageReadings,
        },
    },
    error::{Error, Result},
    protocol::ScpiProtocol,
};

pub struct KeysightAcSource {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    channels: Vec<Arc<Mutex<KeysightAcSourceChannel>>>,
}
impl KeysightAcSource {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        let proto_arc = Arc::new(Mutex::new(proto));

        Ok(Self {
            channels: vec![Arc::new(Mutex::new(KeysightAcSourceChannel::new(
                proto_arc.clone(),
            )))],
            proto: proto_arc,
        })
    }
}
#[async_trait]
impl BaseEquipment for KeysightAcSource {
    async fn connect(&mut self) -> Result<()> {
        /* TODO */
        Ok(())
    }
}
#[async_trait]
impl AcSourceEquipment for KeysightAcSource {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn AcSourceChannel>>> {
        match self.channels.get(idx as usize) {
            None => Err(Error::Unspecified("Index out of range".into())),
            Some(chan) => Ok(chan.clone()),
        }
    }

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn AcSourceChannel>>>> {
        Ok(self
            .channels
            .clone()
            .into_iter()
            .map(|ch| ch as _)
            .collect())
    }

    async fn trigger_now(&mut self) -> Result<()> {
        let mut proto = self.proto.lock().await;

        proto.send("INIT:IMM:SEQuence3").await?;
        proto.send("TRIG:SEQuence3:SOUR BUS").await?;
        proto.send("*TRG").await?;

        Ok(())
    }
}

struct KeysightAcSourceChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
}
impl KeysightAcSourceChannel {
    fn new(proto: Arc<Mutex<Box<dyn ScpiProtocol>>>) -> Self {
        Self { proto }
    }
}
#[async_trait]
impl AcSourceChannel for KeysightAcSourceChannel {
    fn name(&self) -> Result<String> {
        Ok("main".to_string())
    }

    async fn read_voltage(&self) -> Result<AcSourceVoltageReadings> {
        let mut proto = self.proto.lock().await;

        Ok(AcSourceVoltageReadings {
            dc: proto.query_f32(":FETC:VOLT?").await?,
            ac_rms: proto.query_f32(":FETC:VOLT:AC?").await?,
        })
    }

    async fn read_current(&self) -> Result<AcSourceCurrentReadings> {
        let mut proto = self.proto.lock().await;

        Ok(AcSourceCurrentReadings {
            dc: proto.query_f32(":FETC:CURR?").await?,
            ac_rms: proto.query_f32(":FETC:CURR:AC?").await?,
            max: proto.query_f32(":FETC:CURR:AMPL:MAX?").await?,
        })
    }

    async fn read_power(&self) -> Result<AcSourcePowerReadings> {
        let mut proto = self.proto.lock().await;

        Ok(AcSourcePowerReadings {
            dc: proto.query_f32(":FETC:POW?").await?,
            real: proto.query_f32(":FETC:POW:AC?").await?,
            apparent: proto.query_f32(":FETC:POW:AC:APP?").await?,
            reactive: proto.query_f32(":FETC:POW:AC:REAC?").await?,
            factor: proto.query_f32(":FETC:POW:AC:PFAC?").await?,
        })
    }

    async fn read_frequency(&self) -> Result<f32> {
        self.proto.lock().await.query_f32(":FETC:FREQ?").await
    }
}
