use std::{
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const RETRY_DELAY: Duration = Duration::from_secs(30);
const STALE_WINDOW: Duration = Duration::from_secs(30 * 60);

struct Entry<T> {
    received: Instant,
    value: Arc<T>,
    failed: Option<Instant>,
}

pub struct Cache<T> {
    entry: Mutex<Option<Entry<T>>>,
    ttl: Duration,
}

impl<T> Cache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entry: Mutex::new(None),
            ttl,
        }
    }

    pub async fn get<E>(&self, load: impl Future<Output = Result<T, E>>) -> Result<Arc<T>, E> {
        self.get_with_clock(load, Instant::now).await
    }

    async fn get_with_clock<E>(
        &self,
        load: impl Future<Output = Result<T, E>>,
        now: impl Fn() -> Instant,
    ) -> Result<Arc<T>, E> {
        let mut entry = self.entry.lock().await;
        if let Some(entry) = entry.as_ref() {
            let age = now().duration_since(entry.received);
            let backing_off = entry
                .failed
                .is_some_and(|failed| now().duration_since(failed) < RETRY_DELAY);
            if age < self.ttl || (age < self.ttl + STALE_WINDOW && backing_off) {
                return Ok(Arc::clone(&entry.value));
            }
        }

        match load.await {
            Ok(value) => {
                let value = Arc::new(value);
                *entry = Some(Entry {
                    received: now(),
                    value: Arc::clone(&value),
                    failed: None,
                });
                Ok(value)
            }
            Err(error) => {
                if let Some(entry) = entry.as_mut()
                    && now().duration_since(entry.received) < self.ttl + STALE_WINDOW
                {
                    entry.failed = Some(now());
                    return Ok(Arc::clone(&entry.value));
                }
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn expires_exactly_at_ttl() {
        let ttl = Duration::from_secs(30);
        let start = Instant::now();
        let cache = Cache::new(ttl);

        for (elapsed, incoming, expected) in [(0, 1, 1), (29, 2, 1), (30, 3, 3)] {
            let result = cache
                .get_with_clock(async { Ok::<_, ()>(incoming) }, || {
                    start + Duration::from_secs(elapsed)
                })
                .await
                .unwrap();
            assert_eq!(*result, expected);
        }
    }

    #[tokio::test]
    async fn serves_last_success_only_within_stale_window() {
        let ttl = Duration::from_secs(10);
        let start = Instant::now();
        let cache = Cache::new(ttl);
        cache
            .get_with_clock(async { Ok::<_, ()>(42) }, || start)
            .await
            .unwrap();
        let stale = cache
            .get_with_clock(async { Err(()) }, || start + ttl)
            .await
            .unwrap();
        assert_eq!(*stale, 42);
        assert!(
            cache
                .get_with_clock(async { Err(()) }, || start + ttl + STALE_WINDOW)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn concurrent_readers_share_a_successful_load() {
        let cache = Cache::new(Duration::from_secs(60));
        let (started, loader_started) = tokio::sync::oneshot::channel();
        let (release, loader_release) = tokio::sync::oneshot::channel();

        let first = cache.get(async {
            started.send(()).unwrap();
            loader_release.await.unwrap();
            Ok::<_, ()>(7)
        });
        let second = async {
            loader_started.await.unwrap();
            let pending = cache.get(async {
                panic!("a concurrent request started a duplicate load");
            });
            tokio::pin!(pending);
            tokio::select! {
                biased;
                _ = &mut pending => panic!("cache unexpectedly ready before first load"),
                _ = std::future::ready(()) => {}
            }
            release.send(()).unwrap();
            pending.await
        };

        let (a, b) = tokio::join!(first, second);
        let b: Result<Arc<i32>, ()> = b;
        assert!(Arc::ptr_eq(&a.unwrap(), &b.unwrap()));
    }
}
