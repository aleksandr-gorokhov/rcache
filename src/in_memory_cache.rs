use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cache_service::ResolvePayload;

struct CacheValue {
    value: String,
    timestamp: u64,
    ttl: u64,
}

#[derive(Debug, PartialEq)]
pub(crate) enum CacheError {
    EmptyKey,
}

pub(crate) trait TimeSource {
    fn now(&self) -> u64;
}

pub(crate) struct SystemTimeSource;

impl TimeSource for SystemTimeSource {
    fn now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }
}

impl Default for SystemTimeSource {
    fn default() -> Self {
        SystemTimeSource
    }
}

pub struct InMemoryCache<T: TimeSource = SystemTimeSource> {
    values: Arc<Mutex<HashMap<String, CacheValue>>>,
    #[cfg(test)]
    time_source: T,
    #[cfg(not(test))]
    time_source: SystemTimeSource,
    _marker: PhantomData<T>,
    hits: Arc<Mutex<u64>>,
}

impl<T: TimeSource> InMemoryCache<T> {
    pub fn resolve<'b>(&mut self, payload: ResolvePayload) -> Result<String, CacheError> {
        if payload.key.is_empty() {
            return Err(CacheError::EmptyKey);
        }

        let mut hits = self.hits.lock().unwrap();
        let mut values = self.values.lock().unwrap();
        *hits += 1;
        let now = self.time_source.now();

        if *hits % 50000 == 0 {
            values.retain(|_, value| now < value.timestamp + value.ttl);
        } else if let Some(cached_value) = values.get(payload.key) {
            if now >= cached_value.timestamp + cached_value.ttl {
                values.remove(payload.key);
            }
        }

        Ok(values
            .entry(payload.key.to_owned())
            .or_insert_with(|| CacheValue {
                value: payload.value.to_owned(),
                timestamp: now,
                ttl: payload.ttl,
            })
            .value
            .to_owned())
    }
}

impl InMemoryCache<SystemTimeSource> {
    pub fn new() -> InMemoryCache<SystemTimeSource> {
        InMemoryCache {
            values: Arc::new(Mutex::new(HashMap::new())),
            time_source: SystemTimeSource,
            _marker: PhantomData,
            hits: Arc::new(Mutex::new(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<T: TimeSource> InMemoryCache<T> {
        fn new_with_time_source(time_source: T) -> InMemoryCache<T> {
            InMemoryCache {
                time_source,
                values: Arc::new(Mutex::new(HashMap::new())),
                _marker: PhantomData,
                hits: Arc::new(Mutex::new(0)),
            }
        }

        fn set_hits(&mut self, hits: u64) {
            let mut hits_val = self.hits.lock().unwrap();
            *hits_val = hits;
        }

        fn get_values_length(&self) -> usize {
            self.values.lock().unwrap().len()
        }

        fn get_value(&self, key: &str) -> String {
            self.values
                .lock()
                .unwrap()
                .get(key)
                .unwrap()
                .value
                .to_owned()
        }
    }

    struct MockTimeSource {
        now: u64,
    }

    impl MockTimeSource {
        fn new(now: u64) -> Self {
            MockTimeSource { now }
        }

        fn advance(&mut self, secs: u64) {
            self.now += secs;
        }
    }

    impl TimeSource for MockTimeSource {
        fn now(&self) -> u64 {
            self.now
        }
    }

    #[test]
    fn it_should_create_empty_cache() {
        let cache = InMemoryCache::new();
        assert_eq!(cache.get_values_length(), 0);
    }

    #[test]
    fn it_should_return_value() {
        let mut cache = InMemoryCache::new();
        let result = cache
            .resolve(ResolvePayload {
                key: "key",
                value: "value",
                ttl: 1,
            })
            .expect("Should not fail");
        assert_eq!(result, "value");
    }

    #[test]
    fn it_should_store_value_in_cache() {
        let mut cache = InMemoryCache::new();
        assert_eq!(cache.get_values_length(), 0);
        cache
            .resolve(ResolvePayload {
                key: "key",
                value: "value",
                ttl: 1,
            })
            .expect("Should not fail");
        assert_eq!(cache.get_values_length(), 1);
        let result = cache.get_value("key");
        assert_eq!(result, "value");
    }

    #[test]
    fn it_should_cache_value_for_ttl() {
        let mut cache = InMemoryCache::new();
        cache
            .resolve(ResolvePayload {
                key: "key",
                value: "value",
                ttl: 1,
            })
            .expect("Should not fail");
        let cached = cache.resolve(ResolvePayload {
            key: "key",
            value: "value123",
            ttl: 1,
        });
        assert_eq!(cached.unwrap(), "value");
    }

    #[test]
    fn it_should_change_value_on_expiry() {
        let mut cache = InMemoryCache::new_with_time_source(MockTimeSource::new(0));
        cache
            .resolve(ResolvePayload {
                key: "key",
                value: "value",
                ttl: 1,
            })
            .expect("Should not fail");
        cache.time_source.advance(2);
        let cached = cache.resolve(ResolvePayload {
            key: "key",
            value: "value123",
            ttl: 1,
        });
        assert_eq!(cached.unwrap(), "value123");
    }

    #[test]
    fn it_should_resolve_fast_on_big_cache() {
        let now = SystemTime::now();

        let mut cache = InMemoryCache::new();
        for i in 0..100000 {
            cache
                .resolve(ResolvePayload {
                    key: &format!("key{}", i),
                    value: &format!("value{}", i),
                    ttl: 100,
                })
                .expect("Should not fail");
        }
        let result = cache.resolve(ResolvePayload {
            key: "key30",
            value: "value",
            ttl: 100,
        });
        let elapsed = now.elapsed().unwrap().as_millis();
        assert_eq!(&result.unwrap(), "value30");
        assert!(
            elapsed < 500,
            "Elapsed time: {}, should be less than 500",
            elapsed
        );
    }

    #[test]
    fn it_should_get_rid_off_all_expired_keys_periodically() {
        let mut cache = InMemoryCache::new_with_time_source(MockTimeSource::new(0));
        cache
            .resolve(ResolvePayload {
                key: "key49999",
                value: "value49999",
                ttl: 1,
            })
            .expect("Should not fail");
        cache.set_hits(49999);
        cache.time_source.advance(2);
        cache
            .resolve(ResolvePayload {
                key: "key50000",
                value: "value50000",
                ttl: 1,
            })
            .expect("Should not fail");

        assert_eq!(cache.get_values_length(), 1);
    }

    #[test]
    fn it_should_return_error_when_key_is_empty() {
        let mut cache = InMemoryCache::new();
        let result = cache.resolve(ResolvePayload {
            key: "",
            value: "value",
            ttl: 1,
        });
        assert!(matches!(result, Err(CacheError::EmptyKey)));
    }
}
