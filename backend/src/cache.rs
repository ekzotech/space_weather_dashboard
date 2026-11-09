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
        let mut entry = self.entry.lock().await;
        if let Some((time, value)) = entry.as_ref()
            && time.elapsed() < self.ttl
        {
            return Ok(Arc::clone(value));
        }

        let value = Arc::new(load.await?);
        *entry = Some((Instant::now(), Arc::clone(&value)));
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
