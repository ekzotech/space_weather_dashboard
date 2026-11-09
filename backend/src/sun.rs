use chrono::{DateTime, Datelike, Timelike, Utc};
use std::f64::consts::TAU;

// NOAA fractional-year approximation for solar declination and equation of time.
pub fn elevation(lat: f64, lon: f64, time: DateTime<Utc>) -> f64 {
    let utc_hours = f64::from(time.num_seconds_from_midnight()) / 3600.0;
    let year_length = if time.date_naive().leap_year() {
        366.0
    } else {
        365.0
    };
    let gamma = TAU * (f64::from(time.ordinal0()) + (utc_hours - 12.0) / 24.0) / year_length;
    let (sin_1, cos_1) = gamma.sin_cos();
    let (sin_2, cos_2) = (2.0 * gamma).sin_cos();
    let (sin_3, cos_3) = (3.0 * gamma).sin_cos();
    let equation_of_time = 229.18
        * (0.000075 + 0.001868 * cos_1 - 0.032077 * sin_1 - 0.014615 * cos_2 - 0.040849 * sin_2);
    let declination = 0.006918 - 0.399912 * cos_1 + 0.070257 * sin_1 - 0.006758 * cos_2
        + 0.000907 * sin_2
        - 0.002697 * cos_3
        + 0.00148 * sin_3;
    let hour_angle = ((utc_hours * 60.0 + equation_of_time + 4.0 * lon) / 4.0 - 180.0).to_radians();
    let latitude = lat.to_radians();
    let sine =
        latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * hour_angle.cos();
    sine.clamp(-1.0, 1.0).asin().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_day_and_night_at_the_equinox() {
        let noon: DateTime<Utc> = "2024-03-20T12:00:00Z".parse().unwrap();
        assert!(elevation(0.0, 0.0, noon) > 85.0);
        assert!(elevation(0.0, 180.0, noon) < -85.0);
    }
}
