use std::{collections::HashMap, time::SystemTime};

use tokio::sync::Mutex;

pub type Cache = Mutex<HashMap<String, RespEntry>>;

#[derive(Debug, Default)]
pub struct Db {
    pub cache: Cache,
}

#[derive(Debug, Clone)]
pub struct RespEntry {
    pub value: String,
    pub expiry: Option<SystemTime>,
}

impl RespEntry {
    pub fn new(value: String, expiry: Option<SystemTime>) -> Self {
        RespEntry { value, expiry }
    }
}

impl Db {
    pub fn new() -> Self {
        Db {
            cache: Default::default(),
        }
    }

    pub async fn store(&self, key: String, val: String, expiry: Option<SystemTime>) {
        self.cache
            .lock()
            .await
            .insert(key, RespEntry::new(val, expiry));
    }

    pub async fn fetch(&self, key: String) -> Option<String> {
        let entry = {
            let cache = self.cache.lock().await;
            cache.get(&key).cloned()
        };

        match entry {
            Some(entry) => {
                if let Some(expiry) = entry.expiry {
                    if expiry < SystemTime::now() {
                        self.invalidate(&key).await;
                        return None;
                    }
                }
                Some(entry.value)
            }
            None => None,
        }
    }

    async fn invalidate(&self, key: &String) {
        self.cache.lock().await.remove(key);
    }
}
