use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::stream::StreamExt;
use log::{info, error, debug};
use ruuvi_sensor_protocol::{SensorValues, MacAddress};
use tokio::sync::Mutex;
use tokio::time;
use std::error::Error;
use std::time::SystemTime;
use std::collections::HashMap;
use std::sync::Arc;
use std::ops::Deref;

const MANUFACTURER_DATA_ID: u16 = 0x0499;

// Measurement from RuuviTag sensor
#[derive(Debug)]
pub struct Measurement {
    pub timestamp: u64,
    pub published: bool,
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


// Receives all measurements from the task ruuvi::poll
// Stores the latest measurements in an array
async fn update_latest_measurement_array(
    measurement: Measurement,
    latest: Arc<Mutex<HashMap<[u8; 6], Measurement>>> ) {

    debug!("Got {:?}", measurement);
    let mac = measurement.values.mac_address().unwrap();

    let mut latest_changer = latest.lock().await;

    let latest_check = latest_changer.get(&mac);
    match latest_check {
        Some(latest_measurement) => {
            debug!("Updating latest measurement for {:?}", mac);
            latest_changer.insert(mac, measurement);
        },
        None => {
            debug!("No measurements found for {:?}, saving this one", mac);
            latest_changer.insert(mac, measurement);
        },
    }

    drop(latest_changer);


}

pub async fn poll(latest: Arc<Mutex<HashMap<[u8; 6], Measurement>>>) -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;

    // get the first bluetooth adapter and connect
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
                match manufacturer_data.get(&MANUFACTURER_DATA_ID) {
                    Some(values) => {
                        let result = SensorValues::from_manufacturer_specific_data(MANUFACTURER_DATA_ID, values).unwrap();
                        let measurement = Measurement{timestamp: unix_time(), published: false, values: result};
                        let latest_clone = Arc::clone(&latest);
                        update_latest_measurement_array(measurement, latest_clone).await;

                    }
                    None => {}
                };
            }
            _ => {}
        }
    }
    Ok(())
}

