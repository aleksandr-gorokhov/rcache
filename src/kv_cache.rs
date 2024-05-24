use redis::{Client, Commands, Connection, RedisError};

use crate::SetPayload;

pub struct KvCache {
    con: Connection,
}

#[derive(Debug)]
pub enum KvError {
    CommandFailed(RedisError),
    ConnectionNotEstablished,
}

impl From<RedisError> for KvError {
    fn from(err: RedisError) -> Self {
        KvError::CommandFailed(err)
    }
}

impl KvCache {
    pub fn new(url: &str) -> Result<KvCache, KvError> {
        let client = Client::open(url).map_err(|_| KvError::ConnectionNotEstablished)?;
        let con = client
            .get_connection()
            .map_err(|_| KvError::ConnectionNotEstablished)?;
        Ok(KvCache { con })
    }

    pub fn set(&mut self, payload: SetPayload) -> Result<String, KvError> {
        let res: String = self.con.get(payload.key).unwrap_or_else(|_| "".to_string());
        if res.is_empty() {
            self.con
                .set_ex(payload.key, payload.value, payload.ttl)
                .map_err(KvError::CommandFailed)?;
            return Ok(payload.value.to_string());
        }
        Ok(res)
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        let res: String = self.con.get(key).unwrap_or_else(|_| "".to_string());
        if res.is_empty() {
            return None;
        }
        Some(res)
    }

    pub fn unset(&mut self, key: &str) -> Result<(), KvError> {
        self.con.del(key).map_err(KvError::CommandFailed)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl KvCache {
        fn set_raw(&mut self, key: &str, value: &str) -> Result<(), KvError> {
            self.con.set(key, value).map_err(KvError::CommandFailed)?;
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
            .set(SetPayload {
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
        cache.set_raw(key, "42").expect("Should not fail");
        let res = cache
            .set(SetPayload {
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
            .set(SetPayload {
                key,
                value: "42",
                ttl: 1,
            })
            .expect("Should not fail");
        let res = cache.get(key).unwrap();
        assert_eq!(res, "42");
        std::thread::sleep(std::time::Duration::from_secs(2));
        let res = cache.get(key);
        teardown(key);
        assert!(res.is_none());
    }
}
