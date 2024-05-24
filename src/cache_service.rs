use crate::{in_memory_cache, kv_cache};
use crate::in_memory_cache::InMemoryCache;
use crate::kv_cache::KvCache;

pub struct ResolvePayload<'a> {
    pub key: &'a str,
    pub value: &'a str,
    pub ttl: u64,
}

struct CacheService {
    in_memory_cache: InMemoryCache,
    kv_cache: KvCache,
    ttl: u64,
}

#[derive(Debug)]
enum CacheServiceError {
    InMemoryCacheError(in_memory_cache::InMemoryCacheError),
    KvCacheError(kv_cache::KvError),
}

impl CacheService {
    pub fn new(ttl: u64) -> CacheService {
        CacheService {
            in_memory_cache: InMemoryCache::new(),
            kv_cache: KvCache::new("redis://127.0.0.1:6379").expect("KvCache creation failed"),
            ttl,
        }
    }

    pub fn resolve<T>(&mut self, key: &str, resolver: T) -> Result<String, CacheServiceError>
    where
        T: FnOnce() -> String,
    {
        let memory_value = self.in_memory_cache.get(key);

        if let Some(value) = memory_value {
            return Ok(value);
        }

        let kv_value = self.kv_cache.get(key);

        if let Some(value) = kv_value {
            return Ok(value);
        }
        let value = resolver();

        self.kv_cache
            .resolve(ResolvePayload {
                key,
                value: &value,
                ttl: self.ttl,
            })
            .map_err(CacheServiceError::KvCacheError)?;

        let val = self
            .in_memory_cache
            .resolve(ResolvePayload {
                key,
                value: &value,
                ttl: self.ttl,
            })
            .map_err(CacheServiceError::InMemoryCacheError)?;
        println!("Value: {}", val);
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_resolve_value() {
        let mut cache = CacheService::new(10);
        let value = cache.resolve("key", || "value".to_string()).unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn it_should_resolve_value_from_memory() {
        let mut cache = CacheService::new(10);
        cache
            .in_memory_cache
            .resolve(ResolvePayload {
                key: "key",
                value: "value",
                ttl: 10,
            })
            .expect("All should be ok");
        let value = cache.resolve("key", || "never_see".to_string()).unwrap();

        assert_eq!(value, "value");
    }

    #[test]
    fn it_should_resolve_value_from_kv() {
        let mut cache = CacheService::new(10);
        cache
            .kv_cache
            .resolve(ResolvePayload {
                key: "key",
                value: "value",
                ttl: 10,
            })
            .expect("All should be ok");
        let value = cache.resolve("key", || "never_see".to_string()).unwrap();

        assert_eq!(value, "value");
    }

    #[test]
    fn should_set_value_to_memory_cache() {
        let mut cache = CacheService::new(10);
        cache.resolve("key", || "value".to_string()).unwrap();

        let in_memory_value = cache.in_memory_cache.get("key").unwrap();

        assert_eq!(in_memory_value, "value");
    }

    #[test]
    fn should_set_value_to_kv_cache() {
        let mut cache = CacheService::new(10);
        cache.resolve("key", || "value".to_string()).unwrap();

        let kv_cache = cache.kv_cache.get("key").unwrap();

        assert_eq!(kv_cache, "value");
    }
}
