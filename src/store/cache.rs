use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

pub type Cache = Arc<Mutex<HashMap<String, String>>>;

#[derive(Debug, Default, Clone)]
pub struct Db {
    pub cache: Cache,
}

impl Db {
    pub fn new() -> Self {
        Db {
            cache: Default::default(),
        }
    }

    pub async fn store(&self, key: String, val: String) {
        self.cache.lock().await.insert(key, val);
    }

    pub async fn fetch(&self, key: String) -> Option<String> {
        self.cache.lock().await.get(&key).cloned()
    }

    #[allow(dead_code)]
    pub async fn invalidate(&self, key: &String) {
        self.cache.lock().await.remove(key);
    }
}
