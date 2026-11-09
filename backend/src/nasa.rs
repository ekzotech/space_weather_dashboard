use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;

use crate::domain::{Storm, storm_level};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DonkiStorm {
    #[serde(rename = "gstID")]
    gst_id: String,
    #[serde(deserialize_with = "donki_time")]
    start_time: DateTime<Utc>,
    all_kp_index: Option<Vec<KpReading>>,
    linked_events: Option<Vec<LinkedEvent>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KpReading {
    kp_index: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkedEvent {
    #[serde(rename = "activityID")]
    activity_id: String,
}

fn donki_time<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let time = String::deserialize(deserializer)?;
    if let Ok(time) = DateTime::parse_from_rfc3339(&time) {
        return Ok(time.to_utc());
    }
    NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%MZ")
        .map(|time| time.and_utc())
        .map_err(serde::de::Error::custom)
}

impl DonkiStorm {
    pub fn as_storm(&self) -> Option<Storm> {
        let peak_kp = self
            .all_kp_index
            .as_ref()?
            .iter()
            .map(|reading| reading.kp_index)
            .filter(|kp| (0.0..=9.0).contains(kp))
            .max_by(f64::total_cmp)?;

        Some(Storm {
            id: self.gst_id.clone(),
            start: self.start_time,
            peak_kp,
            level: storm_level(peak_kp)?,
            linked_events: self
                .linked_events
                .iter()
                .flatten()
                .map(|event| event.activity_id.clone())
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_donki_payload() {
        let rows: Vec<DonkiStorm> =
            serde_json::from_str(include_str!("../fixtures/donki-gst.json")).unwrap();
        let storms: Vec<_> = rows.iter().filter_map(DonkiStorm::as_storm).collect();
        assert_eq!(storms.len(), 1);
        assert_eq!(storms[0].peak_kp, 8.67);
        assert_eq!(storms[0].level, 4);
        assert_eq!(storms[0].linked_events, ["2024-05-08T05:36:00-CME-001"]);
    }
}
