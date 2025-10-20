use std::time::Duration;

use async_trait::async_trait;
use log::debug;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    time::Instant,
};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::{
    error::{Error, Result},
    model::ModelInfo,
    protocol::{Protocol, ScpiProtocol},
};

pub struct ScpiSerialProtocol {
    port: String,
    baud: u32,
    serial: Option<SerialStream>,
}
impl ScpiSerialProtocol {
    pub fn new(port: &str, baud: u32) -> Self {
        Self {
            port: port.to_string(),
            baud,
            serial: None,
        }
    }
}
#[async_trait]
impl Protocol for ScpiSerialProtocol {
    async fn connect(&mut self) -> Result<()> {
        if self.serial.is_some() {
            return Err(Error::Unspecified("Already connected".into()));
        }

        let serial = tokio_serial::new(&self.port, self.baud)
            .open_native_async()
            .map_err(|e| Error::Unhandled(e.into()))?;
        self.serial = Some(serial);

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.serial.take();
        Ok(())
    }

    async fn model(&mut self) -> Result<ModelInfo> {
        (self as &mut dyn ScpiProtocol).idn_model().await
    }
}
#[async_trait]
impl ScpiProtocol for ScpiSerialProtocol {
    async fn int_send(&mut self, data: &[u8]) -> Result<()> {
        let Some(serial) = &mut self.serial else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        debug!(
            "int_send(): {}",
            String::from_utf8_lossy(data)
                .replace('\n', "␤")
                .replace('\r', "␊")
        );

        serial
            .write_all(data)
            .await
            .map_err(|e| Error::Unhandled(e.into()))?;

        Ok(())
    }

    async fn int_recv(&mut self) -> Result<Vec<u8>> {
        let Some(serial) = &mut self.serial else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        let mut resp = vec![];
        let mut stream = BufReader::new(serial);
        /* TODO: Timeout */
        stream
            .read_until(b'\n', &mut resp)
            .await
            .map_err(|e| Error::Unhandled(e.into()))?;

        debug!(
            "int_recv: {}",
            String::from_utf8_lossy(&resp)
                .replace('\n', "␤")
                .replace('\r', "␊")
        );

        Ok(resp)
    }

    async fn int_query(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.int_send(data).await?;
        self.int_recv().await
    }

    async fn recv_raw(
        &mut self,
        length: Option<usize>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>> {
        let Some(serial) = &mut self.serial else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        debug!("recv_raw({length:?}, {timeout:?})");

        if let Some(length) = length {
            let mut resp = vec![0; length];

            if let Some(timeout) = timeout {
                if tokio::time::timeout(timeout, serial.read_exact(&mut resp))
                    .await
                    .is_err()
                {
                    return Err(Error::Timeout(format!(
                        "Timed out reading {} bytes for {} ms",
                        length,
                        timeout.as_millis()
                    )));
                }
            } else {
                serial.read_exact(&mut resp).await?;
            }

            debug!(
                "recv_raw: {}",
                String::from_utf8_lossy(&resp)
                    .replace('\n', "␤")
                    .replace('\r', "␊")
            );

            Ok(resp)
        } else {
            Err(Error::Unimplemented("TODO".into()))
        }
    }

    async fn recv_until(&mut self, byte: u8, timeout: Duration) -> Result<Vec<u8>> {
        let Some(serial) = &mut self.serial else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        debug!("recv_until({byte}, {timeout:?})");

        let mut data = vec![];
        let end = Instant::now() + timeout;

        loop {
            let now = Instant::now();
            if now >= end {
                return Err(Error::Timeout(format!(
                    "Timed out waiting for {} for {} ms",
                    byte,
                    timeout.as_millis()
                )));
            }
            let remaining = end - now;

            match tokio::time::timeout(remaining, serial.read_u8()).await {
                Err(_) => {
                    return Err(Error::Timeout(format!(
                        "Timed out waiting for {} for {} ms",
                        byte,
                        timeout.as_millis()
                    )));
                }
                Ok(res) => {
                    let res = res?;
                    data.push(res);
                    if res == byte {
                        debug!(
                            "recv_until: {}",
                            String::from_utf8_lossy(&data)
                                .replace('\n', "␤")
                                .replace('\r', "␊")
                        );
                        return Ok(data);
                    }
                }
            }
        }
    }

    async fn flush_rx(&mut self, timeout: Duration) -> Result<()> {
        let Some(serial) = &mut self.serial else {
            return Err(Error::Unspecified("Not connected".into()));
        };

        debug!("flush_rx({timeout:?})");

        let end = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= end {
                break;
            }
            let remaining = end - now;

            /* TODO: Use larger buffer for more efficiency */
            match tokio::time::timeout(remaining, serial.read_u8()).await {
                Err(_) => {
                    break;
                }
                Ok(res) => {
                    res?;
                }
            }
        }

        Ok(())
    }
}
