use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Ovation {
    #[serde(rename = "Forecast Time")]
    pub forecast_time: DateTime<Utc>,
    #[serde(rename = "Observation Time")]
    pub observation_time: DateTime<Utc>,
    pub coordinates: Vec<(i16, i16, i16)>,
}

pub struct AuroraGrid {
    pub forecast_time: DateTime<Utc>,
    pub observation_time: DateTime<Utc>,
    cells: HashMap<(i16, i16), i16>,
}

pub fn valid_coordinates(lat: f64, lon: f64) -> bool {
    (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)
}

impl From<Ovation> for AuroraGrid {
    fn from(feed: Ovation) -> Self {
        Self {
            forecast_time: feed.forecast_time,
            observation_time: feed.observation_time,
            cells: feed
                .coordinates
                .into_iter()
                .filter(|(_, lat, value)| (-90..=90).contains(lat) && (0..=100).contains(value))
                .map(|(lon, lat, value)| ((lon.rem_euclid(360), lat), value))
                .collect(),
        }
    }
}

impl AuroraGrid {
    pub fn distance_to_cell_km(lat: f64, lon: f64) -> f64 {
        let center_lat = lat.round();
        let lat_delta = (center_lat - lat).to_radians();
        let lon_delta = (lon.round() - lon).to_radians();
        let haversine = (lat_delta / 2.0).sin().powi(2)
            + lat.to_radians().cos()
                * center_lat.to_radians().cos()
                * (lon_delta / 2.0).sin().powi(2);
        6371.0 * 2.0 * haversine.clamp(0.0, 1.0).sqrt().asin()
    }

    pub fn estimate(&self, lat: f64, lon: f64) -> Option<i16> {
        let grid_lon = (lon.round() as i16).rem_euclid(360);
        self.cells.get(&(grid_lon, lat.round() as i16)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_longitude_and_distinguishes_zero_from_missing() {
        let payload = r#"{
            "Forecast Time": "2024-05-10T20:30:00Z",
            "Observation Time": "2024-05-10T20:00:00Z",
            "coordinates": [[359, 65, 40], [0, 65, 0], [180, 65, 80]]
        }"#;
        let grid = AuroraGrid::from(serde_json::from_str::<Ovation>(payload).unwrap());
        assert_eq!(grid.estimate(65.1, -1.1), Some(40));
        assert_eq!(grid.estimate(65.1, -0.1), Some(0));
        assert_eq!(grid.estimate(65.1, -180.0), Some(80));
        assert_eq!(grid.estimate(64.0, 0.0), None);
        assert_eq!(AuroraGrid::distance_to_cell_km(65.0, -180.0), 0.0);
        assert!(AuroraGrid::distance_to_cell_km(0.0, 179.75) < 28.0);
        assert!(!valid_coordinates(f64::NAN, 0.0));
    }
}
