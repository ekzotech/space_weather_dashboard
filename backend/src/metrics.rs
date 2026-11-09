use std::{
    fmt::Write,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    time::Duration,
};

#[derive(Default)]
pub struct FeedMetrics {
    requests: AtomicU64,
    failures: AtomicU64,
    elapsed_micros: AtomicU64,
}

impl FeedMetrics {
    pub fn record(&self, elapsed: Duration, failed: bool) {
        self.requests.fetch_add(1, Relaxed);
        self.failures.fetch_add(u64::from(failed), Relaxed);
        self.elapsed_micros
            .fetch_add(elapsed.as_micros().try_into().unwrap_or(u64::MAX), Relaxed);
    }

    pub fn write(&self, output: &mut String, provider: &str) {
        let count = self.requests.load(Relaxed);
        let sum = self.elapsed_micros.load(Relaxed) as f64 / 1_000_000.0;
        let failures = self.failures.load(Relaxed);
        let _ = writeln!(
            output,
            "space_weather_upstream_requests_total{{provider=\"{provider}\"}} {count}"
        );
        let _ = writeln!(
            output,
            "space_weather_upstream_errors_total{{provider=\"{provider}\"}} {failures}"
        );
        let _ = writeln!(
            output,
            "space_weather_upstream_duration_seconds_sum{{provider=\"{provider}\"}} {sum}"
        );
        let _ = writeln!(
            output,
            "space_weather_upstream_duration_seconds_count{{provider=\"{provider}\"}} {count}"
        );
    }
}

pub const HELP: &str = concat!(
    "# HELP space_weather_upstream_requests_total Feed refresh attempts, including cooldown rejections.\n",
    "# TYPE space_weather_upstream_requests_total counter\n",
    "# HELP space_weather_upstream_errors_total Failed feed refresh attempts.\n",
    "# TYPE space_weather_upstream_errors_total counter\n",
    "# HELP space_weather_upstream_duration_seconds Feed refresh duration.\n",
    "# TYPE space_weather_upstream_duration_seconds summary\n",
);
