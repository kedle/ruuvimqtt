use std::collections::HashMap;
use std::sync::Arc;
use log::{debug, info};
use clap::Parser;
use tokio::sync::{Mutex, mpsc};

pub mod mqtt;
pub mod ruuvi;
pub mod utils;

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
    alias: Vec<utils::Alias>,

}


#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Parsing configuration");
    let options = Options::parse();

    // Map from mac address to readable name
    let aliasmap = utils::alias_map(&options.alias);
    debug!("Alias map: {:?}",aliasmap);

    // Channel for passing measurement for MQTT sender task
    // Holds the measurements in memory if broker is down until channel
    // limits are reached
    let (to_mqtt_tx, mut to_mqtt_rx) = mpsc::channel(1024);

    // Latest measurements from RuuviTags
    let latest: Arc<Mutex<HashMap<[u8; 6], ruuvi::Measurement>>> = Arc::new(Mutex::new(HashMap::new()));
    let latest2 = Arc::clone(&latest);

    let mut handles = Vec::new();

    // Task for polling BLE messages and storing them in an array
    handles.push(tokio::task::spawn(async move {
        _ = ruuvi::poll(latest).await;
    }));

    // Task executed periodically to add measurements in a queue (channel)
    // to be sent to the MQTT broker
    handles.push(tokio::task::spawn(async move {
        _ = mqtt::add_latest_measurements_to_send_queue(to_mqtt_tx, latest2).await;
    }));

    // Picks measurements from channel and sends them to MQTT broker
    handles.push(tokio::task::spawn(async move {
        _ = mqtt::mqtt_sender(to_mqtt_rx).await;
    }));

    for handle in handles {
        let output = handle.await.expect("Panic in task");
        debug!("Pushed a handle");
    }

    debug!("Out of loop!");

}
