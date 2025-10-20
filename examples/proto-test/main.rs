use std::{env, process::exit, time::Duration};

use testeq_rs::protocol::scpi_from_uri;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

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

    let mut scpi = scpi_from_uri(uri).await?;

    let start = Instant::now();
    scpi.send("*IDN?").await?;
    let stop = Instant::now();

    println!("Send: {} ms", (stop - start).as_secs_f64() * 1000.);

    let start = Instant::now();
    let data = scpi.recv().await?;
    let stop = Instant::now();

    println!("Recv: {} ms", (stop - start).as_secs_f64() * 1000.);
    println!("  Data: {}", String::from_utf8_lossy(&data));

    let start = Instant::now();
    scpi.query("*IDN?").await?;
    let stop = Instant::now();

    println!("Query {} ms", (stop - start).as_secs_f64() * 1000.);

    scpi.send("*IDN?").await?;
    let start = Instant::now();
    let resp = scpi.recv_raw(None, None).await;
    let stop = Instant::now();

    println!(
        "Recv raw (None, None): {} ms",
        (stop - start).as_secs_f64() * 1000.
    );
    if let Err(e) = resp {
        println!("  Error: {e}");
    }

    scpi.send("*IDN?").await?;
    let start = Instant::now();
    let resp = scpi.recv_raw(Some(10), None).await;
    let stop = Instant::now();

    println!(
        "Recv raw (10 bytes, None): {} ms",
        (stop - start).as_secs_f64() * 1000.
    );
    if let Err(e) = resp {
        println!("  Error: {e}");
    }

    scpi.recv().await?;

    let start = Instant::now();
    let resp = scpi.recv_raw(None, Some(Duration::from_secs(1))).await;
    let stop = Instant::now();

    println!(
        "Recv raw (None, 1 sec (no data)): {} ms",
        (stop - start).as_secs_f64() * 1000.
    );
    if let Err(e) = resp {
        println!("  Error: {e}");
    }

    Ok(())
}
