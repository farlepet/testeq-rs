use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{
    data::{Readings, Unit},
    equipment::{
        BaseEquipment,
        oscilloscope::{
            AnalogWaveform, OscilloscopeCapture, OscilloscopeChannel, OscilloscopeDigitalChannel,
            OscilloscopeEquipment,
            scope_trig::{self, TriggerMode},
        },
    },
    error::{Error, Result},
    model::{Manufacturer, ModelInfo, SiglentFamily},
    protocol::ScpiProtocol,
};

#[allow(unused)]
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

    async fn get_enabled_channels(&self) -> Result<u8> {
        let mut count = 0;
        for chan in &self.analog_channels {
            if chan.lock().await.get_enabled().await? {
                count += 1;
            }
        }
        Ok(count)
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
        for i in 0..3 {
            self.analog_channels
                .push(Arc::new(Mutex::new(SiglentOscilloscopeChannel::new(
                    self.proto.clone(),
                    i,
                ))));
        }
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

    async fn get_memory_depths(&self) -> Result<Vec<u64>> {
        let Some(model) = &self.model else {
            return Ok(vec![]);
        };
        let Manufacturer::Siglent(family) = &model.man_family else {
            return Err(Error::Unspecified(
                "Siglent oscilloscope with non-siglent family!".into(),
            ));
        };

        let n_chan = self.get_enabled_channels().await?;

        Ok(match family {
            SiglentFamily::SDS3000X => {
                /* NOTE: This does not match what is listed in the documentation,
                 * but is what is shown on an SDS3104X. */
                match n_chan {
                    0 | 1 => vec![
                        2_000,
                        10_000,
                        20_000,
                        100_000,
                        200_000,
                        1_000_000,
                        2_000_000,
                        10_000_000,
                        20_000_000,
                        100_000_000,
                        200_000_000,
                        400_000_000,
                    ],
                    _ => vec![
                        1_000,
                        5_000,
                        10_000,
                        50_000,
                        100_000,
                        500_000,
                        1_000_000,
                        5_000_000,
                        10_000_000,
                        50_000_000,
                        100_000_000,
                    ],
                }
            }
            _ => vec![],
        })
    }

    async fn get_memory_depth(&self) -> Result<u64> {
        let resp = self.query_str(":ACQ:MDEP?").await?;
        let mult = match resp.chars().last() {
            Some('k') => 1_000,
            Some('M') => 1_000_000,
            _ => 1,
        };

        let num = if mult != 1 {
            &resp[0..resp.len() - 1]
        } else {
            &resp[..]
        };

        let num: u64 = num
            .parse()
            .map_err(|e| Error::BadResponse(format!("Could not parse response `{num}`: {e}")))?;
        Ok(num * mult)
    }

    async fn set_memory_depth(&self, depth: u64) -> Result<()> {
        let depth_str = if depth < 1_000 {
            format!("{depth}")
        } else if depth < 1_000_000 {
            format!("{}k", depth / 1_000)
        } else if depth < 1_000_000_000 {
            format!("{}M", depth / 1_000_000)
        } else {
            format!("{}G", depth / 1_000_000_000)
        };
        self.send(format!(":ACQ:MDEP {depth_str}")).await
    }

    async fn get_trigger_mode(&self) -> Result<TriggerMode> {
        let mode = self.query_str(":TRIG:MODE?").await?;

        Ok(match mode.as_ref() {
            "AUTO" => TriggerMode::Auto,
            "NORMal" => TriggerMode::Normal,
            "SINGle" => TriggerMode::Single,
            _ => {
                return Err(Error::BadResponse(format!(
                    "Unknown trigger mode response '{mode}'"
                )));
            }
        })
    }

    async fn set_trigger_mode(&mut self, mode: scope_trig::TriggerMode) -> Result<()> {
        let mode_str = match mode {
            TriggerMode::Auto => "AUTO",
            TriggerMode::Normal => "NORM",
            TriggerMode::Single => "SING",
        };

        self.send(format!(":TRIG:MODE {mode_str}")).await
    }

    async fn trigger_now(&mut self) -> Result<()> {
        self.send(":TRIG:MODE FTRIG").await
    }

    async fn read_capture(&mut self) -> Result<OscilloscopeCapture> {
        let mut capture = OscilloscopeCapture::default();

        for chan_lock in &self.analog_channels {
            let chan = chan_lock.lock().await;

            if !chan.get_enabled().await? {
                continue;
            }

            let name = chan.name()?;
            let waveform = chan.read_waveform().await?;

            capture.analog.insert(name, waveform);

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(capture)
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
        let res = proto
            .recv_raw(Some(2), Some(Duration::from_secs(3)))
            .await?;
        let length = (res[1] - b'0').into();
        let value = proto
            .recv_raw(Some(length), Some(Duration::from_secs(1)))
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

        /* Consume final newline */
        self.proto.lock().await.recv().await?;

        Ok(waveform)
    }

    async fn get_enabled(&self) -> Result<bool> {
        let enabled = self
            .query_str(format!(":CHAN{}:SWIT?", self.idx + 1))
            .await?;

        if enabled == "ON" {
            Ok(true)
        } else if enabled == "OFF" {
            Ok(false)
        } else {
            Err(Error::BadResponse(format!(
                "Bad channel switch response '{enabled}'"
            )))
        }
    }

    async fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        let enable_str = if enabled { "ON" } else { "OFF" };
        self.send(format!(":CHAN{}:SWIT {}", self.idx + 1, enable_str))
            .await
    }
}

#[allow(unused)]
struct SiglentOscilloscopeDigitalChannel {
    proto: Arc<Mutex<Box<dyn ScpiProtocol>>>,
    idx: u8,
}
#[allow(unused)]
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
            Ok(unsafe { std::mem::transmute::<[u8; 346], Self>(*ptr) })
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
struct WaveformPreable {
    header: String,
    wavedesc: WaveDescData,
}
