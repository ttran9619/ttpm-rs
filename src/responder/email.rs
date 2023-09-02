use crate::watcher::{Event, Receiver};
use anyhow::Result;
use log;
use sendgrid::v3 as sendgrid;
use serde::Deserialize;

// API Key and From Email for SendGrid
#[derive(Clone, Deserialize)]
pub struct EmailConfig {
    pub api_key: String,
    pub from_email: String,
}

// Read EmailConfig from a file
pub fn load_email_data(path: std::path::PathBuf) -> Result<EmailConfig> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    let config = serde_json::from_reader(reader)?;

    Ok(config)
}

// Sends availability information via email using SendGrid
pub struct Email {
    receiver: Receiver,
    sender: sendgrid::Sender,
    from: sendgrid::Email,
}

impl Email {
    pub fn new(receiver: Receiver, config: EmailConfig) -> Result<Self> {
        let sender = sendgrid::Sender::new(config.api_key);
        let from = sendgrid::Email::new(config.from_email).set_name("TTP Interview Alert");
        Ok(Email {
            receiver,
            sender,
            from,
        })
    }

    // Send an email when a slot becomes available to the given email address
    pub async fn alert_on_availability(&mut self, email: &str) -> Result<()> {
        let target_email = sendgrid::Email::new(email);
        while self.receiver.changed().await.is_ok() {
            // Clone the event data since sending an email can take awhile
            // and we don't want to hold the data lock for too long.
            let data = self.receiver.borrow().clone();

            if let Event::SlotAvailable { location, slot } = data {
                let personalization = sendgrid::Personalization::new(target_email.clone());

                let scheduler_link = "https://ttp.cbp.dhs.gov/schedulerui/schedule-interview/location?lang=en&vo=true&returnUrl=ttp-external&service=UP";
                let scheduler_display_text =
                    "https://ttp.cbp.dhs.gov/schedulerui/schedule-interview/location";
                let content = sendgrid::Content::new()
                    .set_content_type("text/html")
                    .set_value(format!(
                        r#"
                        <html>
                            <body>
                                A Trusted Traveler Program slot has opened for the {} at {}. <br>
                                Schedule at <a href="{}">{}</a>.
                            </body>
                        </html>"#,
                        location.name, slot.start_timestamp, scheduler_link, scheduler_display_text
                    ));

                let m = sendgrid::Message::new(self.from.clone())
                    .set_subject("TTP Slot Available!")
                    .add_content(content)
                    .add_personalization(personalization);

                match self.sender.send(&m).await {
                    Ok(_) => log::info!("Sent email"),
                    Result::Err(err) => log::error!("{}", err),
                }
            }
        }

        Ok(())
    }
}
