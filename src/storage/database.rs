use std::time::Duration;

use dashmap::DashMap;
use tokio::time::Instant;

use crate::object::redis_object::RedisObject;

pub struct Database {
    id: usize,
    data: DashMap<String, RedisObject>,
    expires: DashMap<String, Instant>,
}

impl Database {
    pub fn new(id: usize) -> Self {
        Database {
            id,
            data: DashMap::new(),
            expires: DashMap::new(),
        }
    }

    /// Returns the value's clone by key.
    pub fn get(&self, key: &str) -> Option<RedisObject> {
        if let Some(expire) = self.expires.get(key) {
            if Instant::now() > *expire {
                self.data.remove(key);
                self.expires.remove(key);
                return None;
            }
        }
        self.data.get(key).map(|val| val.clone())
    }

    /// Access the value by the closure `f`
    pub fn get_with<F, R>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(&RedisObject) -> R,
    {
        if let Some(expire) = self.expires.get(key) {
            if Instant::now() > *expire {
                self.data.remove(key);
                self.expires.remove(key);
                return None;
            }
        }
        self.data.get(key).map(|ref_val| f(&*ref_val))
    }

    pub fn set(&self, key: String, value: RedisObject, ttl: Option<Duration>) {
        self.data.insert(key.clone(), value);
        if let Some(ttl) = ttl {
            self.expires.insert(key, Instant::now() + ttl);
        } else {
            self.expires.remove(&key);
        }
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
        instant.elapsed();
    }
}
