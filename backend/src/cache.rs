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
