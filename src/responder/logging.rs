use crate::watcher::{Event, Receiver};
use anyhow::Result;
use log;

// A receiver that logs events
pub struct Logging {
    receiver: Receiver,
}

impl Logging {
    pub fn new(receiver: Receiver) -> Result<Self> {
        Ok(Logging { receiver })
    }

    pub async fn run(&mut self) -> Result<()> {
        while self.receiver.changed().await.is_ok() {
            let data = &*self.receiver.borrow();

            match data {
                Event::None => println!("None"),
                Event::SlotAvailable { location, slot } => log::info!(
                    "{} has an open slot at {}",
                    location.name,
                    slot.start_timestamp
                ),
                Event::Error(err) => log::error!("Error {}", err),
            }
        }

        Ok(())
    }
}
