use clap::Parser;
// use ruuvi_sensor_protocol::{SensorValues, Humidity, Temperature, ParseError};
use tokio::sync::{Mutex, mpsc};
use tokio::time;
use std::mem;
use std::collections::HashMap;
use std::sync::Arc;
//use tracing::{info, Level};
//use tracing_subscriber;
use log::{debug, info};

pub mod ruuvi;
pub mod utils;
use utils::Alias;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, about, rename_all = "kebab-case")]
struct Options {
    /// MQTT host, dns ipv4 or ipv6
    #[clap(long, default_value_t = String::from("127.0.0.1"))]
    host: String,

    /// MQTT port
    #[clap(long, default_value_t = 1883)]
    port: u16,

    /// MQTT topic
    #[clap(long, default_value_t = String::from("kitchen/ruuvitag/SENSOR"))]
    topic: String,

    /// mac address
    #[clap(long, default_value_t = String::from("DE:AD:BE:EF:00:00"))]
    mac: String,

    /// verbose logging
    #[clap(short = 'v', long = "verbose")]
    verbose: bool,

    #[clap(long, parse(try_from_str = utils::parse_alias))]
    /// example: DE:AD:BE:EF:00:00=Sauna.
    alias: Vec<Alias>,

}


#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Parsing configuration");
    let options = Options::parse();

    // Map from mac address to readable name
    let aliasmap = utils::alias_map(&options.alias);
    debug!("Alias map: {:?}",aliasmap);

    // To store the latest measurement from each tag
    // let latest_measurements = Arc::new(Mutex::new(HashMap::new()));

    // Clone the handle for the latest measuements to pass it fowrards for the writer
    // let measurement_input = latest_measurements.clone();

    let (from_ruuvi_tx, mut from_ruuvi_rx) = mpsc::channel(32);
    let from_ruuvi_tx2 = from_ruuvi_tx.clone();

    let mut handles = Vec::new();

    info!("Starting bluetooth");
    handles.push(tokio::task::spawn(async move {
        _ = ruuvi::poll(from_ruuvi_tx2).await;
    }));


    handles.push(tokio::task::spawn(async move {
        _ = ruuvi::send_latest_to_broker(from_ruuvi_rx).await;
    }));
    //ruuvi::poll().await;
    //println!("Sizeof measurement: {:?}", mem::size_of::<ruuvi::Measurement>());

    for handle in handles {
        let output = handle.await.expect("Panic in task");
        debug!("Pushed a handle");
    }

    debug!("Out of loop!");

}
