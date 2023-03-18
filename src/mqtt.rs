// use tokio::io::AsyncWriteExt;
// use tokio::net::TcpStream;
use log::{trace, info, debug};
use uuid::Uuid;
// use ruuvi_sensor_protocol::{SensorValues, MacAddress};
use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use tokio::sync::{Mutex, mpsc::{Sender, Receiver}};
use ntex::time::{sleep, Seconds};
use ntex::util::{join_all, lazy, ByteString, Bytes, Ready};
use ntex_mqtt::v3::{
    client, codec, ControlMessage, Handshake, HandshakeAck, MqttServer, Publish, Session,
};
use super::ruuvi;


// use mqtt::control::variable_header::ConnectReturnCode;
// use mqtt::packet::*;
// use mqtt::{Decodable, Encodable, QualityOfService};
#[derive(Debug)]
struct AsdError;

pub fn generate_client_id() -> String {
    format!("ruuvimqtt/{}", Uuid::new_v4())
}

pub async fn mqtt_sender(
    mut to_mqtt_rx: Receiver<ruuvi::Measurement>
    ) -> () {


    let client = client::MqttConnector::new("10.0.10.10:1883")
        .client_id(generate_client_id())
        .max_packet_size(30)
        .keep_alive(Seconds(3))
        .connect()
        .await
        .unwrap();

    let sink = client.sink();

    // ntex::rt::spawn(client.start_default());
        // publish handler
    let router = client.resource("response", |pkt: Publish| async move {
        log::info!(
            "incoming publish: {:?} -> {:?} payload {:?}",
            pkt.id(),
            pkt.topic(),
            pkt.payload()
        );
        Ok::<_, AsdError>(())
    });
    ntex::rt::spawn(router.start_default());

    while let Some(message) = to_mqtt_rx.recv().await {
        let ack = sink
            .publish(ByteString::from_static("topic"), "neekeri".into())
            .send_at_least_once()
            .await
            .unwrap();
        // debug!("mesg: {:?}", message);
        debug!("Ack received: {:?}", ack);

    }

    sink.close();

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
        debug!("MQTT latest: {:?}",latest_changer);
        sleep(Seconds(10)).await;
        drop(latest_changer);
    }
    Ok(())
}
