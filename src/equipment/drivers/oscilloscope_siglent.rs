use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{
    data::{Readings, Unit},
    equipment::{
        oscilloscope::{
            AnalogWaveform, OscilloscopeCapture, OscilloscopeChannel, OscilloscopeDigitalChannel,
            OscilloscopeEquipment,
        },
        BaseEquipment,
    },
    error::{Error, Result},
    model::ModelInfo,
    protocol::ScpiProtocol,
};

pub struct SiglentOscilloscope {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    model: Option<ModelInfo>,
    analog_channels: Vec<Arc<Mutex<SiglentOscilloscopeChannel>>>,
    digital_channels: Vec<Arc<Mutex<SiglentOscilloscopeDigitalChannel>>>,
}
impl SiglentOscilloscope {
    pub fn new(proto: Box<dyn ScpiProtocol>) -> Result<Self> {
        let proto_arc = Arc::new(Mutex::new(proto));

        Ok(Self {
            analog_channels: vec![],
            digital_channels: vec![],
            proto: proto_arc,
            model: None,
        })
    }
}
#[async_trait::async_trait]
impl BaseEquipment for SiglentOscilloscope {
    async fn connect(&mut self) -> Result<()> {
        if !self.analog_channels.is_empty() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        let model = self.proto.lock().await.model().await?;
        self.model = Some(model);
        /* TODO: Check model */
        self.analog_channels
            .push(Arc::new(Mutex::new(SiglentOscilloscopeChannel::new(
                self.proto.clone(),
                0,
            ))));
        self.analog_channels
            .push(Arc::new(Mutex::new(SiglentOscilloscopeChannel::new(
                self.proto.clone(),
                1,
            ))));
        self.analog_channels
            .push(Arc::new(Mutex::new(SiglentOscilloscopeChannel::new(
                self.proto.clone(),
                2,
            ))));
        self.analog_channels
            .push(Arc::new(Mutex::new(SiglentOscilloscopeChannel::new(
                self.proto.clone(),
                3,
            ))));
        Ok(())
    }
}
#[async_trait::async_trait]
impl OscilloscopeEquipment for SiglentOscilloscope {
    async fn get_channel(&mut self, idx: u8) -> Result<Arc<Mutex<dyn OscilloscopeChannel>>> {
        match self.analog_channels.get(idx as usize) {
            None => Err(Error::Unspecified("Index out of range".into())),
            Some(chan) => Ok(chan.clone()),
        }
    }

    async fn get_channels(&mut self) -> Result<Vec<Arc<Mutex<dyn OscilloscopeChannel>>>> {
        Ok(self
            .analog_channels
            .clone()
            .into_iter()
            .map(|ch| ch as _)
            .collect())
    }

    async fn get_digital_channel(
        &mut self,
        idx: u8,
    ) -> Result<Arc<Mutex<dyn OscilloscopeDigitalChannel>>> {
        match self.digital_channels.get(idx as usize) {
            None => Err(Error::Unspecified("Index out of range".into())),
            Some(chan) => Ok(chan.clone()),
        }
    }

    async fn get_digital_channels(
        &mut self,
    ) -> Result<Vec<Arc<Mutex<dyn OscilloscopeDigitalChannel>>>> {
        Ok(self
            .digital_channels
            .clone()
            .into_iter()
            .map(|ch| ch as _)
            .collect())
    }

    async fn read_capture(&mut self) -> Result<OscilloscopeCapture> {
        Ok(OscilloscopeCapture {
            analog: vec![],
            digital: vec![],
        })
    }
}

struct SiglentOscilloscopeChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
}
impl SiglentOscilloscopeChannel {
    fn new(proto: Arc<Mutex<Box<dyn ScpiProtocol>>>, idx: u8) -> Self {
        Self { proto, idx }
    }

    async fn send(&self, cmd: impl AsRef<[u8]>) -> Result<()> {
        self.proto.lock().await.send(cmd).await
    }

    async fn query_str(&self, cmd: impl AsRef<[u8]>) -> Result<String> {
        let resp = self.proto.lock().await.query(cmd).await?;
        let resp = String::from_utf8_lossy(&resp);
        Ok(resp
            .trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string())
    }

    async fn read_header(&self, proto: &mut dyn ScpiProtocol) -> Result<String> {
        proto.recv_until(b'#', Duration::from_secs(3)).await?;
        let length = proto
            .recv_raw(Some(1), Some(Duration::from_secs(1)))
            .await?;
        let value = proto
            .recv_raw(
                Some((length[0] - b'0').into()),
                Some(Duration::from_secs(1)),
            )
            .await?;
        Ok(String::from_utf8_lossy(&value).to_string())
    }

    async fn read_preamble(&self) -> Result<WaveformPreable> {
        let mut proto = self.proto.lock().await;

        proto.send(":WAV:PRE?").await?;
        let pre = self.read_header(proto.as_mut()).await?;
        let desc = proto
            .recv_raw(Some(346), Some(Duration::from_secs(1)))
            .await?;

        Ok(WaveformPreable {
            header: pre,
            wavedesc: WaveDescData::from_bytes(&desc)?,
        })
    }

    async fn read_samples(
        &self,
        count: usize,
        wavedesc: &WaveDescData,
        dest: &mut Vec<f64>,
    ) -> Result<()> {
        let mut proto = self.proto.lock().await;

        let bytes = if wavedesc.comm_type == 0 {
            count
        } else {
            count * 2
        };

        self.read_header(proto.as_mut()).await?;
        let raw = proto
            .recv_raw(Some(bytes), Some(Duration::from_secs(3)))
            .await?;

        let scale = (wavedesc.attenuation as f64 * wavedesc.vert_gain as f64)
            / wavedesc.code_per_div as f64;
        let offset = wavedesc.vert_offset as f64;

        if wavedesc.comm_type == 0 {
            let mut samples = raw
                .into_iter()
                .map(|s| ((s as i8) as f64) * scale - offset)
                .collect();
            dest.append(&mut samples);
        } else {
            let mut samples = raw
                .chunks(2)
                .map(|s| (i16::from_le_bytes(s.try_into().unwrap()) as f64) * scale - offset)
                .collect();
            dest.append(&mut samples);
        }

        Ok(())
    }
}
#[async_trait::async_trait]
impl OscilloscopeChannel for SiglentOscilloscopeChannel {
    fn name(&self) -> Result<String> {
        Ok(format!("CH{}", self.idx + 1))
    }

    async fn read_waveform(&self) -> Result<AnalogWaveform> {
        self.send(format!(":WAV:SOUR C{}", self.idx + 1)).await?;
        /* Start at point 0 */
        self.send(":WAV:STAR 0").await?;
        /* 20000 points per read */
        /* TODO: Use :WAV:MAXP? to get max size */
        self.send(":WAV:POIN 20000").await?;
        /* Retrieve every data point */
        self.send(":WAV:INT 1").await?;
        /* Set data width */
        /* TODO: Support byte width */
        self.send(":WAV:WIDT WORD").await?;

        let pre = self.read_preamble().await?;

        let mut waveform = AnalogWaveform {
            time_per_pt: pre.wavedesc.horiz_interval.into(),
            readings: Readings {
                unit: Unit::Voltage,
                values: vec![],
            },
        };

        let samples = pre.wavedesc.n_points as usize;
        let mut sample = 0;

        while sample < samples {
            let mut samples_to_read = samples - sample;
            if samples_to_read > 20000 {
                samples_to_read = 20000;
            }

            self.send(":WAV:DATA?").await?;
            self.read_samples(
                samples_to_read,
                &pre.wavedesc,
                &mut waveform.readings.values,
            )
            .await?;

            sample += samples_to_read;
        }

        Ok(waveform)
    }
}

struct SiglentOscilloscopeDigitalChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
}
impl SiglentOscilloscopeDigitalChannel {
    fn new(proto: Arc<Mutex<Box<dyn ScpiProtocol>>>, idx: u8) -> Self {
        Self { proto, idx }
    }

    async fn send(&self, cmd: &str) -> Result<()> {
        self.proto.lock().await.send(cmd).await
    }

    async fn query_str(&self, cmd: &str) -> Result<String> {
        let resp = self.proto.lock().await.query(cmd).await?;
        let resp = String::from_utf8_lossy(&resp);
        Ok(resp
            .trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string())
    }
}
#[async_trait::async_trait]
impl OscilloscopeDigitalChannel for SiglentOscilloscopeDigitalChannel {
    fn name(&self) -> Result<String> {
        Ok(format!("D{}", self.idx))
    }
}

#[derive(Debug)]
#[repr(C, packed(2))]
struct WaveDescData {
    descriptor: [u8; 16],
    template: [u8; 16],
    comm_type: u16,
    comm_order: u16,
    length: u32,
    _res0: [u32; 5],
    wave_len: u32,
    _res1: [u32; 3],
    name: [u8; 16],
    _res2: [u32; 6],
    n_points: u32,
    _res3: [u32; 3],
    start_point: u32,
    point_interval: u32,
    _res4: u32,
    read_frames: u32,
    sum_frames: u32,
    _res5: u32,
    vert_gain: f32,
    vert_offset: f32,
    code_per_div: f32,
    _res6: u32,
    adc_bit: u16,
    frame_idx: u16,
    horiz_interval: f32,
    horiz_offset: f64,
    _res7: [u32; 34],
    time_base: u16,
    coupling: u16,
    attenuation: f32,
    fixed_vert_gain: u16,
    bwidth_limit: u16,
    _res8: [u32; 2],
    source: u16,
}
impl WaveDescData {
    fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() != std::mem::size_of::<Self>() {
            Err(Error::Unspecified(format!(
                "Attempt to populate WaveDescData from data of size {}!",
                data.len()
            )))
        } else {
            let ptr: *const [u8; std::mem::size_of::<Self>()] = data.as_ptr() as _;
            Ok(unsafe { std::mem::transmute(*ptr) })
        }
    }
}

#[derive(Debug)]
struct WaveformPreable {
    header: String,
    wavedesc: WaveDescData,
}
