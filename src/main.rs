use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_serial::{SerialStream, SerialPortBuilderExt};
use serde::Deserialize;
use std::fs;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    unit_name: String,
    central_ip_address: String,
    central_ip_port: String,
    serial_port: String,
    serial_baud_rate: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = load_config(&args.config)?;

    println!("config loaded!");

    let stream = TcpStream::connect(format!("{}:{}", config.central_ip_address, config.central_ip_port)).await?;
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    let mut serial = tokio_serial::new(config.serial_port, config.serial_baud_rate).open_native_async()?;

    println!("Connection established!");

    while let Some(Ok(line)) = lines.next_line().await {
        println!("Received: {}", line);

        if let Some((unit_name, content)) = parse_message(&line) {
            if unit_name == config.unit_name {
                println!("Receiver: {}", content);
                tokio::io::AsyncWriteExt::write_all(&mut serial, content.as_bytes()).await?;
                tokio::io::AsyncWriteExt::write_all(&mut serial, b"\n").await?;
            } else {
                println!("Different ship({}), Skipping", unit_name);
            }
        }
    }

    Ok(())
}

fn load_config(path: &str) -> anyhow::Result<Config> {
    let text = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}

fn parse_message(msg: &str) -> Option<(String, String)> {
    if msg.starts_with(':') {
        let parts: Vec<&str> = msg.split(';').collect();
        if parts.len() == 2 {
            let ship_name = parts[0].trim_start_matches(':').to_string();
            let control = parts[1];
            if control.starts_with('!') {
                let content = &control[1..];
                return Some((ship_name, content.to_string()));
            }
        }
    }
    None
}