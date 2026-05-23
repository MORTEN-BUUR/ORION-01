use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub hash: String,
}

#[derive(Debug, Default)]
pub struct SymbolMap {
    inner: RwLock<HashMap<String, SymbolEntry>>,
}

impl SymbolMap {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<SymbolEntry> {
        self.inner.read().get(key).cloned()
    }

    pub fn insert(&self, key: String, value: serde_json::Value) -> SymbolEntry {
        let hash = Self::compute_hash(&key, &value);
        let entry = SymbolEntry {
            key: key.clone(),
            value: value.clone(),
            hash: hash.clone(),
        };
        self.inner.write().insert(key, entry.clone());
        entry
    }

    pub fn remove(&self, key: &str) -> Option<SymbolEntry> {
        self.inner.write().remove(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.inner.read().contains_key(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.inner.read().keys().cloned().collect()
    }

    pub fn export(&self) -> HashMap<String, SymbolEntry> {
        self.inner.read().clone()
    }

    fn compute_hash(key: &str, value: &serde_json::Value) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.update(value.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
