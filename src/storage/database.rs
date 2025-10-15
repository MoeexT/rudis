use std::time::{Duration, SystemTime};

use dashmap::DashMap;

use crate::object::redis_object::RedisObject;

pub struct Database {
    data: DashMap<String, RedisObject>,
    expires: DashMap<String, SystemTime>,
}

impl Database {
    pub fn new(_: usize) -> Self {
        Database {
            data: DashMap::new(),
            expires: DashMap::new(),
        }
    }

    /// Returns the value's clone by key.
    pub fn get(&self, key: &str) -> Option<RedisObject> {
        if self.is_expired(key) {
            self.data.remove(key);
            None
        } else {
            self.data.get(key).map(|val| val.clone())
        }
    }

    /// Access the value by the closure `f`
    pub fn get_with<F, R>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(&RedisObject) -> R,
    {
        if self.is_expired(key) {
            self.data.remove(key);
            None
        } else {
            self.data.get(key).map(|ref_val| f(&*ref_val))
        }
    }

    pub fn set(&self, key: String, value: RedisObject, ttl: Option<Duration>) {
        self.data.insert(key.clone(), value);
        if let Some(ttl) = ttl {
            self.expires.insert(key, SystemTime::now() + ttl);
        } else {
            self.expires.remove(&key);
        }
    }

    fn is_expired(&self, key: &str) -> bool {
        self.expires
            .remove_if(key, |_, v| SystemTime::now() > *v)
            .is_some()
    }
}

#[cfg(test)]
mod test {
    #[cfg(test)]
    use std::collections::HashMap;
    #[cfg(test)]
    use std::mem;

    #[test]
    fn test() {
        let map: HashMap<String, String> = HashMap::new();
        dbg!("{}", map.get("k"));

        dbg!("", mem::size_of::<std::time::Instant>());
        dbg!("", mem::size_of::<tokio::time::Instant>());

        let instant = std::time::Instant::now();
        let _ = instant.elapsed();
    }
}
