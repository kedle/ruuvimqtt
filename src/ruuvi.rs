use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::stream::StreamExt;
use log::{info, error, debug};
use ruuvi_sensor_protocol::{SensorValues, MacAddress};
use tokio::sync::{Mutex, mpsc::{Sender, Receiver}};
use tokio::time;
use std::error::Error;
use std::time::SystemTime;
use std::collections::HashMap;
use std::sync::{Arc};

const MANUFACTURER_DATA_ID: u16 = 0x0499;

// Measurement from RuuviTag sensor
#[derive(Debug)]
pub struct Measurement {
    pub timestamp: u64,
    pub values: SensorValues,
}

fn unix_time() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => {
            error!("SystemTime before UNIX EPOCH! Using 0");
            0
        },
    }
}

async fn get_central(manager: &Manager) -> Adapter {
    let adapters = manager.adapters().await.unwrap();
    adapters.into_iter().nth(0).unwrap()
}


// Updates the latest measurement for each mac address, held in shared
// state guarded by mutex
async fn update_latest_measurement(
    measurement_storage: Arc<Mutex<HashMap<[u8; 6], Measurement>>>,
    measurement: Measurement
) ->  Result<(), Box<dyn Error>> {
        let mac = measurement.values.mac_address().unwrap();
        let mut lock = measurement_storage.lock().await;
        lock.insert(mac, measurement);
        info!("Got updated measurement for tag {:02x?}", mac);
        //debug!("State is now: {:?}", lock);
        Ok(())
}

// Receives all measurements from the task ruuvi::poll
// Stores the latest measurements and sends them to mqtt sender
// if throttle interval has passed
pub async fn send_latest_to_broker(
    mut from_ruuvi_rx: Receiver<Measurement>) {

    // Latest measurements for each mac address
    let mut latest: HashMap<[u8; 6], Measurement> = HashMap::new();

    while let Some(measurement) = from_ruuvi_rx.recv().await {
        debug!("Got {:?}", measurement);
        let mac = measurement.values.mac_address().unwrap();

        let latest_check = latest.get(&mac);
        match latest_check {
            Some(latest_measurement) => {
                debug!("Found latest, checking timestamp");
                if latest_measurement.timestamp + 60 < measurement.timestamp {
                    debug!("Updating latest measurement");
                    latest.insert(mac, measurement);
                }
                else {
                    debug!("Update was sent less than 60 seconds ago, not updating latest");
                }
            },
            None => {
                debug!("No measurements found for {:?}, saving this one",mac);
                latest.insert(mac, measurement);
            },
        }

    }

}

// pub async fn poll(measurement_storage: Arc<Mutex<HashMap<[u8; 6], Measurement>>>) -> Result<(), Box<dyn Error>> {
pub async fn poll(from_ruuvi_tx: Sender<Measurement>) -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;

    // get the first bluetooth adapter
    // connect to the adapter
    let central = get_central(&manager).await;

    // Each adapter has an event stream, we fetch via events(),
    // simplifying the type, this will return what is essentially a
    // Future<Result<Stream<Item=CentralEvent>>>. 
    let mut events = central.events().await?;

    // start scanning for devices
    info!("Starting scan for tags");
    central.start_scan(ScanFilter::default()).await?;

    // Loop and wait for data
    while let Some(event) = events.next().await {
        // Only react on data advertisements, ignore others
        match event {
            CentralEvent::ManufacturerDataAdvertisement {
                id: _,
                manufacturer_data,
            } => {
                // Only match to data coming from RuuviTags with a known vendor id
                // second
                match manufacturer_data.get(&MANUFACTURER_DATA_ID) {
                    Some(values) => {
                        //println!("Ruuvidata! {:?}",values);
                        let result = SensorValues::from_manufacturer_specific_data(MANUFACTURER_DATA_ID, values).unwrap();
                        let measurement = Measurement{timestamp: unix_time(), values: result};
                        // debug!("Gor measurement in ruuvi.rs: {:?}",measurement);
                        from_ruuvi_tx.send(measurement).await;
                        // let storage = measurement_storage.clone();
                        // _ = update_latest_measurement(storage, measurement).await;

                    }
                    None => {}
                };
            }
            _ => {}
        }
    }
    Ok(())
}
