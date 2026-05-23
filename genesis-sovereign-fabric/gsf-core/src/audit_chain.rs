use crate::signed_ledger::{SignedEntry, SignedLedger};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AuditChain {
    ledger: SignedLedger,
    index: HashMap<String, Vec<usize>>,
}

impl AuditChain {
    pub fn new(ledger: SignedLedger) -> Self {
        let mut chain = Self {
            ledger,
            index: HashMap::new(),
        };
        chain.rebuild_index();
        chain
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, e) in self.ledger.entries().iter().enumerate() {
            self.index
                .entry(e.action.clone())
                .or_default()
                .push(i);
        }
    }

    pub fn ledger(&self) -> &SignedLedger {
        &self.ledger
    }

    pub fn ledger_mut(&mut self) -> &mut SignedLedger {
        &mut self.ledger
    }

    pub fn entries(&self) -> &[SignedEntry] {
        self.ledger.entries()
    }

    pub fn last_hash(&self) -> &str {
        self.ledger.last_hash()
    }

    pub fn export(&self) -> Vec<serde_json::Value> {
        self.entries()
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id.to_string(),
                    "timestamp": e.timestamp.to_rfc3339(),
                    "action": e.action,
                    "payload": e.payload,
                    "prev_hash": e.prev_hash,
                    "hash": e.hash,
                    "signature": e.signature,
                    "signer": e.signer,
                })
            })
            .collect()
    }

    pub fn verify_chain_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(SignedLedger::GENESIS_HASH.as_bytes());
        for e in self.entries() {
            hasher.update(e.hash.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    pub fn find_by_action(&self, action: &str) -> Vec<&SignedEntry> {
        self.index
            .get(action)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&i| self.entries().get(i))
                    .collect()
            })
            .unwrap_or_default()
    }
}
