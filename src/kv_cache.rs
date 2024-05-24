use redis::{Client, Commands, Connection, FromRedisValue, RedisError};

struct KvCache {
    con: Connection,
}

#[derive(Debug)]
enum KvError {
    CommandFailed(RedisError),
    ConnectionNotEstablished,
}

impl From<RedisError> for KvError {
    fn from(err: RedisError) -> Self {
        KvError::CommandFailed(err)
    }
}

struct ResolvePayload<'a> {
    key: &'a str,
    value: &'a str,
    ttl: u64,
}

impl KvCache {
    fn new(url: &str) -> Result<KvCache, KvError> {
        let client = Client::open(url).map_err(|_| KvError::ConnectionNotEstablished)?;
        let con = client
            .get_connection()
            .map_err(|_| KvError::ConnectionNotEstablished)?;
        Ok(KvCache { con })
    }

    fn resolve(&mut self, payload: ResolvePayload) -> Result<String, KvError> {
        let res: String = self.con.get(payload.key).unwrap_or_else(|_| "".to_string());
        if res.is_empty() {
            self.con
                .set_ex(payload.key, payload.value, payload.ttl)
                .map_err(KvError::CommandFailed)?;
            return Ok(payload.value.to_string());
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl KvCache {
        fn set(&mut self, key: &str, value: &str) -> Result<(), KvError> {
            self.con.set(key, value).map_err(KvError::CommandFailed)?;
            Ok(())
        }

        fn get(&mut self, key: &str) -> Result<String, KvError> {
            let res: String = self.con.get(key).unwrap_or_else(|_| "".to_string());
            Ok(res)
        }

        fn unset(&mut self, key: &str) -> Result<(), KvError> {
            self.con.del(key).map_err(KvError::CommandFailed)?;
            Ok(())
        }
    }

    fn teardown(key: &str) {
        let mut cache = KvCache::new("redis://127.0.0.1:6379")
            .expect("Should establish connection with no problem");
        cache.unset(key).expect("Should not fail");
    }

    #[test]
    fn it_should_return_empty_value() {
        let key = "foo1";
        let mut cache = KvCache::new("redis://127.0.0.1:6379")
            .expect("Should establish connection with no problem");
        let res = cache
            .resolve(ResolvePayload {
                key,
                value: "",
                ttl: 1,
            })
            .expect("Should not fail");
        teardown(key);
        assert_eq!(res, "");
    }

    #[test]
    fn it_should_return_value() {
        let key = "foo2";
        let mut cache = KvCache::new("redis://127.0.0.1:6379")
            .expect("Should establish connection with no problem");
        cache.set(key, "42").expect("Should not fail");
        let res = cache
            .resolve(ResolvePayload {
                key,
                value: "42",
                ttl: 1,
            })
            .expect("Should not fail");
        teardown(key);
        assert_eq!(res, "42");
    }

    #[test]
    fn it_should_return_error_for_wrong_connection() {
        let cache = KvCache::new("");
        assert!(matches!(cache, Err(KvError::ConnectionNotEstablished)));
    }

    #[test]
    fn it_should_cache_value_for_ttl() {
        let key = "foo3";
        let mut cache = KvCache::new("redis://127.0.0.1:6379")
            .expect("Should establish connection with no problem");

        cache
            .resolve(ResolvePayload {
                key,
                value: "42",
                ttl: 1,
            })
            .expect("Should not fail");
        let res = cache.get(key).expect("Should not fail");
        assert_eq!(res, "42");
        std::thread::sleep(std::time::Duration::from_secs(2));
        let res = cache.get(key).expect("Should not fail");
        teardown(key);
        assert_eq!(res, "");
    }
}
