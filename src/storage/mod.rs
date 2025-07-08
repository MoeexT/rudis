use std::time::Duration;

use dashmap::DashMap;
use tokio::time::Instant;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Object {
    String(Vec<u8>),
    List,
}

pub struct Database {
    id: usize,
    data: DashMap<String, Object>,
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
    pub fn get(&self, key: &str) -> Option<Object> {
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
        F: FnOnce(&Object) -> R,
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

    pub fn set(&self, key: String, value: Object, ttl: Option<Duration>) {
        self.data.insert(key.clone(), value);
        if let Some(ttl) = ttl {
            self.expires.insert(key, Instant::now() + ttl);
        } else {
            self.expires.remove(&key);
        }
    }
}
// let storage_cleaner = storage.clone();
// tokio::spawn(async move {
//     let mut interval = tokio::time::interval(Duration::from_secs(5));
//     loop {
//         interval.tick().await;
//         let mut storage = storage_cleaner.write().await;
//         storage.retain(|key, entry|{
//             entry.expiry.map(|expiry| {
//                 log::debug!("Retain key: {key}");
//                 return expiry > Instant::now();
//             }).unwrap_or(true)
//         });
//     }
// });
mod test {
    #[cfg(test)]
    use std::collections::HashMap;

    #[test]
    fn test() {
        let map: HashMap<String, String> = HashMap::new();
        dbg!("{}", map.get("k"));
    }
}
