use std::{sync::Arc, time::Duration};

use chrono::{Days, Utc};
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::{
    aurora::{AuroraGrid, Ovation},
    cache::Cache,
    domain::Sample,
    nasa::DonkiStorm,
    noaa::{kp_series, xray_series},
};

const NOAA: &str = "https://services.swpc.noaa.gov";

pub struct Feeds {
    client: Client,
    nasa_key: String,
    kp: Cache<Vec<Sample>>,
    forecast: Cache<Vec<Sample>>,
    xray: Cache<Vec<Sample>>,
    aurora: Cache<AuroraGrid>,
    storms: Cache<Vec<DonkiStorm>>,
}

impl Feeds {
    pub fn new(nasa_key: String) -> Result<Self, reqwest::Error> {
        let five_minutes = Duration::from_secs(300);
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(12))
                .user_agent("SpaceWeatherDashboard/0.1")
                .build()?,
            nasa_key,
            kp: Cache::new(five_minutes),
            forecast: Cache::new(five_minutes),
            xray: Cache::new(Duration::from_secs(60)),
            aurora: Cache::new(five_minutes),
            storms: Cache::new(Duration::from_secs(1800)),
        })
    }

    async fn noaa<T: DeserializeOwned>(&self, path: &str) -> Result<T, reqwest::Error> {
        self.client
            .get(format!("{NOAA}/{path}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    pub async fn kp(&self) -> Result<Arc<Vec<Sample>>, reqwest::Error> {
        self.kp
            .get(async {
                Ok(kp_series(
                    self.noaa("products/noaa-planetary-k-index.json").await?,
                    false,
                ))
            })
            .await
    }

    pub async fn forecast(&self) -> Result<Arc<Vec<Sample>>, reqwest::Error> {
        self.forecast
            .get(async {
                Ok(kp_series(
                    self.noaa("products/noaa-planetary-k-index-forecast.json")
                        .await?,
                    true,
                ))
            })
            .await
    }

    pub async fn xray(&self) -> Result<Arc<Vec<Sample>>, reqwest::Error> {
        self.xray
            .get(async {
                Ok(xray_series(
                    self.noaa("json/goes/primary/xrays-1-day.json").await?,
                ))
            })
            .await
    }

    pub async fn aurora(&self) -> Result<Arc<AuroraGrid>, reqwest::Error> {
        self.aurora
            .get(async {
                Ok(AuroraGrid::from(
                    self.noaa::<Ovation>("json/ovation_aurora_latest.json")
                        .await?,
                ))
            })
            .await
    }

    pub async fn storms(&self) -> Result<Arc<Vec<DonkiStorm>>, reqwest::Error> {
        self.storms
            .get(async {
                let end = Utc::now().date_naive();
                let start = end - Days::new(30);
                self.client
                    .get("https://api.nasa.gov/DONKI/GST")
                    .query(&[
                        ("startDate", start.to_string()),
                        ("endDate", end.to_string()),
                        ("api_key", self.nasa_key.clone()),
                    ])
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await
            })
            .await
    }
}
