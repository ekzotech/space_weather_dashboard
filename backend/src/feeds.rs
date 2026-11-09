use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use chrono::{Days, Utc};
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;

use crate::{
    aurora::{AuroraGrid, Ovation},
    cache::{Cache, Cached},
    domain::Sample,
    nasa::DonkiStorm,
    noaa::{kp_series, xray_series},
};

const MAX_FEED_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum FeedError {
    Http(reqwest::Error),
    TooLarge,
    RateLimited,
    InvalidJson(serde_json::Error),
}

impl From<reqwest::Error> for FeedError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error.without_url())
    }
}

async fn fetch_json<T: DeserializeOwned>(
    request: RequestBuilder,
    cooldown: &Mutex<Option<Instant>>,
) -> Result<T, FeedError> {
    if cooldown
        .lock()
        .await
        .is_some_and(|until| Instant::now() < until)
    {
        return Err(FeedError::RateLimited);
    }
    let mut response = request.send().await?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let delay = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| {
                value.parse::<u64>().ok().or_else(|| {
                    let deadline = chrono::DateTime::parse_from_rfc2822(value).ok()?;
                    Some((deadline.to_utc() - Utc::now()).num_seconds().max(0) as u64)
                })
            })
            .unwrap_or(60)
            .clamp(1, 86_400);
        let until = Instant::now() + Duration::from_secs(delay);
        let mut current = cooldown.lock().await;
        *current = Some(current.map_or(until, |existing| existing.max(until)));
        return Err(FeedError::RateLimited);
    }
    response = response.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|size| size > MAX_FEED_BYTES as u64)
    {
        return Err(FeedError::TooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > MAX_FEED_BYTES - bytes.len() {
            return Err(FeedError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(FeedError::InvalidJson)
}

const NOAA: &str = "https://services.swpc.noaa.gov";

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CloudCover {
    pub cloud_cover: Option<u8>,
    pub time: String,
}

#[derive(serde::Deserialize)]
struct CloudResponse {
    current: CloudCover,
}

pub struct Feeds {
    client: Client,
    noaa_base: String,
    nasa_url: String,
    noaa_cooldown: Mutex<Option<Instant>>,
    nasa_cooldown: Mutex<Option<Instant>>,
    weather_cooldown: Mutex<Option<Instant>>,
    kp: Cache<Vec<Sample>>,
    forecast: Cache<Vec<Sample>>,
    xray: Cache<Vec<Sample>>,
    aurora: Cache<AuroraGrid>,
    storms: Cache<Vec<DonkiStorm>>,
}

impl Feeds {
    pub fn new() -> Result<Self, reqwest::Error> {
        let five_minutes = Duration::from_secs(300);
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(12))
                .user_agent("SpaceWeatherDashboard/0.1")
                .build()?,
            noaa_base: NOAA.into(),
            nasa_url: "https://ccmc.gsfc.nasa.gov/DONKI-API/get/GST".into(),
            noaa_cooldown: Mutex::new(None),
            nasa_cooldown: Mutex::new(None),
            weather_cooldown: Mutex::new(None),
            kp: Cache::new(five_minutes),
            forecast: Cache::new(five_minutes),
            xray: Cache::new(Duration::from_secs(60)),
            aurora: Cache::new(five_minutes),
            storms: Cache::new(Duration::from_secs(1800)),
        })
    }

    #[cfg(test)]
    pub fn test_sources(mut self, base: &str) -> Self {
        self.noaa_base = base.into();
        self.nasa_url = format!("{base}/gst");
        self
    }

    async fn noaa<T: DeserializeOwned>(&self, path: &str) -> Result<T, FeedError> {
        fetch_json(
            self.client.get(format!("{}/{path}", self.noaa_base)),
            &self.noaa_cooldown,
        )
        .await
    }

    pub async fn kp(&self) -> Result<Cached<Vec<Sample>>, FeedError> {
        self.kp
            .get(async {
                Ok(kp_series(
                    self.noaa("products/noaa-planetary-k-index.json").await?,
                    false,
                ))
            })
            .await
    }

    pub async fn forecast(&self) -> Result<Cached<Vec<Sample>>, FeedError> {
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

    pub async fn xray(&self) -> Result<Cached<Vec<Sample>>, FeedError> {
        self.xray
            .get(async {
                Ok(xray_series(
                    self.noaa("json/goes/primary/xrays-1-day.json").await?,
                ))
            })
            .await
    }

    pub async fn aurora(&self) -> Result<Cached<AuroraGrid>, FeedError> {
        self.aurora
            .get(async {
                Ok(AuroraGrid::from(
                    self.noaa::<Ovation>("json/ovation_aurora_latest.json")
                        .await?,
                ))
            })
            .await
    }

    pub async fn cloud_cover(&self, lat: f64, lon: f64) -> Result<CloudCover, FeedError> {
        let data: CloudResponse = fetch_json(
            self.client
                .get("https://api.open-meteo.com/v1/forecast")
                .query(&[
                    ("latitude", lat.to_string()),
                    ("longitude", lon.to_string()),
                    ("current", "cloud_cover".into()),
                    ("timezone", "UTC".into()),
                ]),
            &self.weather_cooldown,
        )
        .await?;
        Ok(data.current)
    }

    pub async fn storms(&self) -> Result<Cached<Vec<DonkiStorm>>, FeedError> {
        self.storms
            .get(async {
                let end = Utc::now().date_naive();
                let start = end - Days::new(30);
                fetch_json(
                    self.client.get(&self.nasa_url).query(&[
                        ("startDate", start.to_string()),
                        ("endDate", end.to_string()),
                    ]),
                    &self.nasa_cooldown,
                )
                .await
            })
            .await
    }
}
