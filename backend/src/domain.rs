use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Sample {
    pub time: DateTime<Utc>,
    pub value: f64,
}

pub fn storm_level(kp: f64) -> Option<u8> {
    (5.0..=9.0).contains(&kp).then(|| kp.floor() as u8 - 4)
}

#[derive(Serialize)]
pub struct Storm {
    pub id: String,
    pub start: DateTime<Utc>,
    pub peak_kp: f64,
    pub peak_source: Option<String>,
    pub level: u8,
    pub linked_events: Vec<String>,
}

pub fn rank_storms(storms: &mut Vec<Storm>, min_kp: f64) {
    storms.retain(|storm| storm.peak_kp >= min_kp);
    storms.sort_by(|a, b| {
        b.peak_kp
            .total_cmp(&a.peak_kp)
            .then_with(|| b.start.cmp(&a.start))
            .then_with(|| a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storm_scale_boundaries() {
        for (kp, expected) in [
            (4.99, None),
            (5.0, Some(1)),
            (6.67, Some(2)),
            (9.0, Some(5)),
            (9.1, None),
        ] {
            assert_eq!(storm_level(kp), expected);
        }
    }

    #[test]
    fn ranking_filters_and_breaks_ties() {
        let storm = |id: &str, day, peak_kp| Storm {
            id: id.into(),
            start: DateTime::parse_from_rfc3339(&format!("2024-05-{day}T00:00:00Z"))
                .unwrap()
                .to_utc(),
            peak_kp,
            peak_source: None,
            level: storm_level(peak_kp).unwrap(),
            linked_events: vec![],
        };
        let mut storms = vec![
            storm("c", 12, 7.0),
            storm("a", 10, 7.0),
            storm("b", 10, 8.0),
            storm("d", 13, 5.0),
        ];
        rank_storms(&mut storms, 7.0);
        assert_eq!(
            storms.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["b", "c", "a"]
        );
    }
}
