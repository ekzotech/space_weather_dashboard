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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_kp_preserves_fractions_and_removes_duplicate_times() {
        let rows = serde_json::from_str(
            r#"[
            {"time_tag":"2026-11-08T03:00:00","Kp":4.33,"station_count":8},
            {"time_tag":"2026-11-08T00:00:00","Kp":2.67,"station_count":8},
            {"time_tag":"2026-11-08T00:00:00","Kp":2.67,"station_count":8},
            {"time_tag":"2026-11-08T06:00:00","Kp":-1}
        ]"#,
        )
        .unwrap();

        let points = kp_series(rows, false);
        assert_eq!(
            points.iter().map(|point| point.value).collect::<Vec<_>>(),
            [2.67, 4.33]
        );
        assert!(points[0].time < points[1].time);
    }

    #[test]
    fn kp_forecast_excludes_observed_and_estimated_rows() {
        let rows = serde_json::from_str(
            r#"[
            {"time_tag":"2026-11-08T00:00:00","kp":2.33,"observed":"observed"},
            {"time_tag":"2026-11-08T03:00:00","kp":5.33,"observed":"estimated"},
            {"time_tag":"2026-11-08T06:00:00","kp":6.67,"observed":"predicted"}
        ]"#,
        )
        .unwrap();

        let points = kp_series(rows, true);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].value, 6.67);
    }

    #[test]
    fn goes_keeps_positive_flux_from_the_long_wavelength_band() {
        let rows = serde_json::from_str(
            r#"[
            {"time_tag":"2026-11-08T00:00:00Z","energy":"0.05-0.4nm","flux":0.00001},
            {"time_tag":"2026-11-08T00:00:00Z","energy":"0.1-0.8nm","flux":0.00002},
            {"time_tag":"2026-11-08T00:01:00Z","energy":"0.1-0.8nm","flux":null},
            {"time_tag":"2026-11-08T00:02:00Z","energy":"0.1-0.8nm","flux":0}
        ]"#,
        )
        .unwrap();

        let points = xray_series(rows);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].value, 0.00002);
    }
}
