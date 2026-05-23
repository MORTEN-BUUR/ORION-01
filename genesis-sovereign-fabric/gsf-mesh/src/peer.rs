use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub fingerprint: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Default)]
pub struct PeerAllowlist {
    fingerprints: HashSet<String>,
}

impl PeerAllowlist {
    pub fn new() -> Self {
        Self {
            fingerprints: HashSet::new(),
        }
    }

    pub fn from_env(env_var: &str) -> Self {
        let mut allowlist = Self::new();
        if let Ok(val) = std::env::var(env_var) {
            for fp in val.split(',') {
                let fp = fp.trim();
                if !fp.is_empty() {
                    allowlist.add(fp);
                }
            }
        }
        allowlist
    }

    pub fn add(&mut self, fingerprint: &str) {
        self.fingerprints.insert(fingerprint.to_string());
    }

    pub fn contains(&self, fingerprint: &str) -> bool {
        self.fingerprints.contains(fingerprint)
    }

    pub fn remove(&mut self, fingerprint: &str) -> bool {
        self.fingerprints.remove(fingerprint)
    }

    pub fn is_empty(&self) -> bool {
        self.fingerprints.is_empty()
    }

    pub fn len(&self) -> usize {
        self.fingerprints.len()
    }
}
