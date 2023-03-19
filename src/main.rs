use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use std::time::Duration;
use log::{debug, info, error};
use tokio::sync::{Mutex, mpsc};
use rumqttc::{MqttOptions, AsyncClient, QoS};

// use std::{fs::File, io::BufReader};

pub mod config;
pub mod mqtt;
pub mod ruuvi;
pub mod utils;

async fn testi() -> Result<(), Box<dyn Error>> {
    info!("Testifunktiosta moro");
    Ok(())
}


#[tokio::main]
async fn main() {
    env_logger::init();

    // Reada the config and store it behind an immutable atomic referenc
    let cli = config::cli();
    let config = Arc::new(config::read(cli.config));

    info!("Config: {:?}", config);


    // Channel for passing measurement for MQTT sender task
    // Holds the measurements in memory if broker is down until channel
    // limits are reached
    let (to_mqtt_tx, to_mqtt_rx) = mpsc::channel(config.mqtt.queue_capacity);

    // Latest measurements from RuuviTags in a mutable, reference counted array
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

    let mut mqttoptions = MqttOptions::new(mqtt::generate_client_id(), &config.mqtt.host, config.mqtt.port);
    mqttoptions.set_keep_alive(Duration::from_secs(config.mqtt.keepalive_interval));

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);

    // Picks measurements from channel and sends them to MQTT broker
    let config_to_mqtt_sender = Arc::clone(&config);
    handles.push(tokio::task::spawn(async move {
        _ = mqtt::mqtt_sender(to_mqtt_rx, client, config_to_mqtt_sender).await;
    }));

    // Polls the MQTT event loop and handles sending the messages
    handles.push(tokio::task::spawn(async move {
        _ = mqtt::mqtt_eventloop(eventloop).await;
    }));

    handles.push(tokio::task::spawn(async move {
        _ = testi().await;
    }));


    debug!("References to config: {}", Arc::strong_count(&config));

    for handle in handles {
        let output = match handle.await {
            Ok(_) => {info!("Task terminated gracefully")},
            Err(e) => {error!("Task terminated with an error: {}",e)},
        };
    }

    debug!("Out of loop!");

}
