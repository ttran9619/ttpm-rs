use anyhow::Result;
use reqwest;
use serde::{Deserialize, Deserializer};
use time::PrimitiveDateTime;

// A TTP Interview Location
#[derive(Clone, Deserialize)]
pub struct Location {
    pub id: i16,
    pub name: String,
}

// A TTP Interview Slot
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub location_id: i16,
    #[serde(deserialize_with = "deserialize_iso8601_minutes_precision")]
    pub start_timestamp: PrimitiveDateTime,
    #[serde(deserialize_with = "deserialize_iso8601_minutes_precision")]
    pub end_timestamp: PrimitiveDateTime,
}

// A TTP Interview Slot Availability
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotAvailability {
    pub available_slots: Vec<Slot>,
    #[serde(deserialize_with = "deserialize_iso8601_seconds_precision")]
    pub last_published_date: PrimitiveDateTime,
}

// The TTP REST API Client
pub struct Client {
    http_client: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder().build()?;

        Ok(Client { http_client })
    }

    // Get all open locations for global entry
    pub async fn get_all_open_locations(&self) -> Result<Vec<Location>> {
        let response = self
            .http_client
            .get("https://ttp.cbp.dhs.gov/schedulerapi/locations/")
            .query(&[
                ("temporary", "false"),
                ("inviteOnly", "false"),
                ("operational", "true"),
                ("serviceName", "Global Entry"),
            ])
            .send()
            .await?;

        Ok(response.json::<Vec<Location>>().await?)
    }

    // Get slot availability for a location
    pub async fn get_slot_availability(&self, location: &Location) -> Result<SlotAvailability> {
        let response = self
            .http_client
            .get("https://ttp.cbp.dhs.gov/schedulerapi/slot-availability")
            .query(&[("locationId", location.id)])
            .send()
            .await?;

        Ok(response.json::<SlotAvailability>().await?)
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "(name: {}, id: {})", self.name, self.id)
    }
}

// Serde deserialize function for ISO8601 timestamps without milliseconds
fn deserialize_iso8601_seconds_precision<'de, D>(
    deserializer: D,
) -> Result<PrimitiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;

    const FORMAT: &[time::format_description::FormatItem<'_>] =
        time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");

    PrimitiveDateTime::parse(buf.as_str(), &FORMAT).map_err(serde::de::Error::custom)
}

// Serde deserialize function for ISO8601 timestamps without seconds or milliseconds
fn deserialize_iso8601_minutes_precision<'de, D>(
    deserializer: D,
) -> Result<PrimitiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;

    const FORMAT: &[time::format_description::FormatItem<'_>] =
        time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]");

    PrimitiveDateTime::parse(buf.as_str(), &FORMAT).map_err(serde::de::Error::custom)
}

// Create unit tests for serde functions
