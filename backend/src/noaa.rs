use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;

use crate::domain::Sample;

#[derive(Deserialize)]
pub struct KpRow {
    time_tag: NaiveDateTime,
    #[serde(rename = "Kp", alias = "kp")]
    kp: f64,
    observed: Option<String>,
}

pub fn kp_series(rows: Vec<KpRow>, forecast: bool) -> Vec<Sample> {
    let mut points: Vec<_> = rows
        .into_iter()
        .filter(|row| !forecast || row.observed.as_deref() == Some("predicted"))
        .filter(|row| (0.0..=9.0).contains(&row.kp))
        .map(|row| Sample {
            time: row.time_tag.and_utc(),
            value: row.kp,
        })
        .collect();
    points.sort_by_key(|point| point.time);
    points.dedup_by_key(|point| point.time);
    points
}

#[derive(Deserialize)]
pub struct XrayRow {
    time_tag: DateTime<Utc>,
    energy: String,
    flux: Option<f64>,
}

pub fn xray_series(rows: Vec<XrayRow>) -> Vec<Sample> {
    let mut points: Vec<_> = rows
        .into_iter()
        .filter(|row| row.energy == "0.1-0.8nm")
        .filter_map(|row| {
            Some(Sample {
                time: row.time_tag,
                value: row.flux.filter(|flux| *flux > 0.0)?,
            })
        })
        .collect();
    points.sort_by_key(|point| point.time);
    points.dedup_by_key(|point| point.time);
    points
}
