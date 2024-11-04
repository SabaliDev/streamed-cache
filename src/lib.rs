use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt};
use std::{
    collections::HashMap,
    result::Result,
    sync::{Arc, Mutex},
};

type City = String;
type Temperature = u64;

#[async_trait]
pub trait Api: Send + Sync + 'static + Clone {
    async fn fetch(&self) -> Result<HashMap<City, Temperature>, String>;
    async fn subscribe(&self) -> BoxStream<Result<(City, Temperature), String>>;
}

pub struct StreamCache {
    results: Arc<Mutex<HashMap<String, u64>>>,
}

impl StreamCache {
    pub fn new(api: impl Api) -> Self {
        let instance = Self {
            results: Arc::new(Mutex::new(HashMap::new())),
        };
        instance.update_in_background(api);
        instance
    }

    pub fn get(&self, key: &str) -> Option<u64> {
        let results = self.results.lock().expect("poisoned");
        results.get(key).copied()
    }

    pub fn update_in_background(&self, api: impl Api + 'static) {
        let results = Arc::clone(&self.results);

        tokio::spawn(async move {
            // Start subscribing to updates
            let mut subscription = api.subscribe().await;

            // Spawn a task to handle the fetch operation
            let fetch_results = Arc::clone(&results);
            let fetch_api = api.clone();
            let fetch_handle = tokio::spawn(async move {
                if let Ok(fetched_data) = fetch_api.fetch().await {
                    let mut results_lock = fetch_results.lock().unwrap();
                    for (city, temp) in fetched_data {
                        results_lock.entry(city).or_insert(temp);
                    }
                }
            });

            // Process subscription updates
            while let Some(update) = subscription.next().await {
                if let Ok((city, temperature)) = update {
                    let mut results_lock = results.lock().unwrap();
                    results_lock.insert(city, temperature);
                }
            }

            // Ensure the fetch operation completes
            let _ = fetch_handle.await;
        });
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio::sync::Notify;
    use tokio::time;

    use futures::{future, stream::select, FutureExt, StreamExt};
    use maplit::hashmap;

    use super::*;

    #[derive(Default, Clone)]
    struct TestApi {
        signal: Arc<Notify>,
        fetch_error: Option<String>,
    }

    #[async_trait]
    impl Api for TestApi {
        async fn fetch(&self) -> Result<HashMap<City, Temperature>, String> {
            // Simulate error if fetch_error is set
            if let Some(err) = self.fetch_error.clone() {
                return Err(err);
            }
            // fetch is slow and may get delayed until after we receive the first updates
            self.signal.notified().await;
            Ok(hashmap! {
                "Berlin".to_string() => 29,
                "Paris".to_string() => 31,
            })
        }

        async fn subscribe(&self) -> BoxStream<Result<(City, Temperature), String>> {
            let results = vec![
                Ok(("London".to_string(), 27)),
                Ok(("Paris".to_string(), 32)),
                // Simulate an error for demonstration purposes
                Err("Subscription error".to_string()),
            ];
            select(
                futures::stream::iter(results),
                async {
                    self.signal.notify_one();
                    future::pending().await
                }
                .into_stream(),
            )
            .boxed()
        }
    }

    #[tokio::test]
    async fn works() {
        let cache = StreamCache::new(TestApi::default());

        // Allow cache to update
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(cache.get("Berlin"), Some(29));
        assert_eq!(cache.get("London"), Some(27));
        assert_eq!(cache.get("Paris"), Some(32));
    }

    #[tokio::test]
    async fn test_subscription_error_handling() {
        let api = TestApi::default();
        let cache = StreamCache::new(api.clone());

        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(cache.get("London"), Some(27));
        assert_eq!(cache.get("Paris"), Some(32));
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(cache.get("London"), Some(27));
    }

    #[tokio::test]
    async fn test_missing_keys() {
        let cache = StreamCache::new(TestApi::default());
        assert_eq!(cache.get("NotExist"), None);
    }

    #[tokio::test]
    async fn test_initial_state() {
        let cache = StreamCache::new(TestApi::default());
        assert_eq!(cache.get("Berlin"), None);
        assert_eq!(cache.get("London"), None);
        assert_eq!(cache.get("Paris"), None);
    }
}
