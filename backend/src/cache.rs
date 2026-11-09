use std::{
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

pub struct Cache<T> {
    entry: Mutex<Option<(Instant, Arc<T>)>>,
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
        if let Some((time, value)) = entry.as_ref()
            && now().duration_since(*time) < self.ttl
        {
            return Ok(Arc::clone(value));
        }

        let value = Arc::new(load.await?);
        *entry = Some((now(), Arc::clone(&value)));
        Ok(value)
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
