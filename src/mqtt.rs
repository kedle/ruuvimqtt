// use tokio::io::AsyncWriteExt;
// use tokio::net::TcpStream;
use log::{info, debug};
use uuid::Uuid;
// use ruuvi_sensor_protocol::{SensorValues, MacAddress};
use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use tokio::sync::{Mutex, mpsc::{Sender, Receiver}};
use tokio::time::{sleep, Duration};
use rumqttc::QoS;

use super::config::Config;
use super::ruuvi::Measurement;


pub fn generate_client_id() -> String {
    format!("ruuvimqtt-{}", Uuid::new_v4())
}

pub async fn mqtt_sender(
    mut to_mqtt_rx: Receiver<Measurement>,
    client: rumqttc::AsyncClient,
    config: Arc<Config>
    ) -> () {

    // mac suodatus ja topikin vaihto ennen lahetysta tahan
    // pitaisko suodattaa jo aiemmin ettei jono t'yty turhista??

    while let Some(message) = to_mqtt_rx.recv().await {
        let ack = client.publish(
            "hello/rumqtt",
            QoS::AtLeastOnce,
            false,
            message.to_string()
        ).await.unwrap();
    }

    ()
}

pub async fn mqtt_eventloop(
    mut eventloop: rumqttc::EventLoop
    ) -> () {

    while let Ok(notification) = eventloop.poll().await {
        debug!("{:?}", notification);
    }
}


pub async fn add_latest_measurements_to_send_queue(
    mut from_ruuvi_tx: Sender<Measurement>,

    latest: Arc<Mutex<HashMap<[u8; 6], Measurement>>> ) -> Result<(), Box<dyn Error>> {

    loop {
        let mut latest_changer = latest.lock().await;

        let mut measurements: Vec<Measurement> = latest_changer.values().cloned().collect();

        for (_, val) in latest_changer.iter_mut() {
            val.published = true;
        }

        drop(latest_changer);

        for measurement in measurements {
            match measurement.published {
                true => { debug!("Found measurement that is already published, ignoring")},
                false => {
                    debug!("Found new measurement: {}", measurement.to_owned());
                    from_ruuvi_tx.send(measurement.clone()).await;
                }
            };
        }

        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}
