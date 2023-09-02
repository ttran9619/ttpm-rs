use crate::rest;
use anyhow::{Context, Result};
use serde_json;
use time::{Date, Duration, PrimitiveDateTime};
use tokio::sync::watch;
use tokio::time::sleep;

#[derive(Clone)]
pub enum Event {
    None,
    Error(String),
    SlotAvailable {
        location: rest::Location,
        slot: rest::Slot,
    },
}

pub type Receiver = watch::Receiver<Event>;

// Polls the REST API, posting Events to receivers
pub struct Watcher {
    rest_client: rest::Client,
    sender: watch::Sender<Event>,
    receiver_base: Receiver,
    poll_period: Duration,
    locations: Vec<rest::Location>,
}

// Read Location data from a file
pub fn load_locations_from_file(path: std::path::PathBuf) -> Result<Vec<rest::Location>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    let locations = serde_json::from_reader(reader)?;

    Ok(locations)
}

impl Watcher {
    pub async fn new_no_cache(poll_period: Duration) -> Result<Self> {
        let rest_client = rest::Client::new()?;

        let locations = rest_client.get_all_open_locations().await?;

        let (notify, receiver_base) = watch::channel(Event::None);

        Ok(Watcher {
            rest_client,
            sender: notify,
            receiver_base,
            poll_period,
            locations,
        })
    }

    #[allow(dead_code)]
    pub fn new(poll_period: Duration, locations: Vec<rest::Location>) -> Result<Self> {
        let rest_client = rest::Client::new()?;

        let (notify, receiver_base) = watch::channel(Event::None);

        Ok(Watcher {
            rest_client,
            sender: notify,
            receiver_base,
            poll_period,
            locations,
        })
    }

    // Get a receiver for the Watcher
    pub fn get_receiver(&self) -> Receiver {
        self.receiver_base.clone()
    }

    // Poll the REST API for a location
    pub async fn watch(&self, location: &str) -> Result<()> {
        let search_pattern = location;

        let target_location = self
            .locations
            .iter()
            .find(|loc| loc.name.contains(search_pattern))
            .context(format!(
                "Could not find location matching {}",
                search_pattern
            ))?;

        let mut last_ts_string = PrimitiveDateTime::MIN;
        let mut last_date = Date::MIN;

        loop {
            match self
                .rest_client
                .get_slot_availability(target_location)
                .await
            {
                Ok(availability) => {
                    // Check if the last published date has changed
                    if availability.last_published_date != last_ts_string {
                        if let Some(slot) = availability.available_slots.first() {
                            // Check if the slot date has changed
                            if slot.start_timestamp.date() != last_date {
                                self.sender.send(Event::SlotAvailable {
                                    location: target_location.clone(),
                                    slot: slot.clone(),
                                })?;

                                last_date = slot.start_timestamp.date();
                                last_ts_string = availability.last_published_date;
                            }
                        }
                    }
                }
                Result::Err(err) => self.sender.send(Event::Error(err.to_string()))?,
            }

            let poll_period =
                std::time::Duration::from_secs(self.poll_period.whole_seconds() as u64);
            sleep(poll_period).await;
        }
    }
}
