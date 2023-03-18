// use tokio::io::AsyncWriteExt;
// use tokio::net::TcpStream;
use log::{trace, info, debug};
use uuid::Uuid;
// use ruuvi_sensor_protocol::{SensorValues, MacAddress};
use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use tokio::sync::{Mutex, mpsc::{Sender, Receiver}};
use tokio::time::{sleep, Duration};
use rumqttc::{MqttOptions, AsyncClient, QoS};

use super::ruuvi;


pub fn generate_client_id() -> String {
    format!("ruuvimqtt/{}", Uuid::new_v4())
}

pub async fn mqtt_sender(
    mut to_mqtt_rx: Receiver<ruuvi::Measurement>
    ) -> () {

    let mut mqttoptions = MqttOptions::new(generate_client_id(), "10.0.10.10", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    client.subscribe("hello/rumqtt", QoS::AtMostOnce).await.unwrap();

    while let Some(message) = to_mqtt_rx.recv().await {
        debug!("msg received: {:?}", message);
        let ack = client.publish("hello/rumqtt", QoS::AtLeastOnce, false, "neger").await.unwrap();
        debug!("ack: {:?}",ack);

    }

    ()
}


pub async fn add_latest_measurements_to_send_queue(
    mut from_ruuvi_tx: Sender<ruuvi::Measurement>,
    latest: Arc<Mutex<HashMap<[u8; 6], ruuvi::Measurement>>> ) -> Result<(), Box<dyn Error>> {

    // Latest measurements for each mac address
    // let mut latest: HashMap<[u8; 6], ruuvi::Measurement> = HashMap::new();


    debug!("Entering latest checker loop");
    loop {
        debug!("Pretending to send measurements!");
        let mut latest_changer = latest.lock().await;

        let mut measurements: Vec<ruuvi::Measurement> = latest_changer.values().cloned().collect();

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

        // debug!("measurements: {:?}",measurements);

        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}
