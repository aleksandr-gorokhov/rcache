use redis::{Client, Commands, Connection, FromRedisValue, Value};

struct KvCache {
    con: Connection,
}

#[derive(Debug)]
enum KvError {
    CommandFailed,
}

impl KvCache {
    fn new(url: &str) -> KvCache {
        let client = Client::open(url).unwrap();
        let con = client.get_connection().unwrap();
        KvCache { con }
    }

    fn resolve(&mut self, key: &str) -> Result<Value, KvError> {
        self.con.get(key).map_err(|_| KvError::CommandFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl KvCache {
        fn set(&mut self, key: &str, value: &str) -> Result<(), KvError> {
            self.con
                .set(key, value)
                .map_err(|_| KvError::CommandFailed)?;
            Ok(())
        }
    }

    #[test]
    fn it_should_return_empty_value() {
        let key = "bar";
        let mut cache = KvCache::new("redis://127.0.0.1:6379");
        let res = cache.resolve(key).expect("Should not fail");
        assert_eq!(res, Value::Nil);
    }

    #[test]
    fn it_should_return_value() {
        let key = "foo";
        let mut cache = KvCache::new("redis://127.0.0.1:6379");
        cache.set(key, "42").expect("Should not fail");
        let res = cache.resolve(key).expect("Should not fail");
        assert_eq!(res, Value::Data("42".as_bytes().to_vec()));
    }

    #[test]
    fn mock_redis() {
        todo!("Find out how to mock redis");
    }
}
