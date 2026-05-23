use crate::signed_ledger::{SignedEntry, SignedLedger};
use ed25519_dalek::VerifyingKey;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct ReplayEngine {
    entries: VecDeque<SignedEntry>,
    #[allow(dead_code)]
    genesis_hash: String,
}

impl ReplayEngine {
    pub fn new(entries: Vec<SignedEntry>, genesis_hash: &str) -> Self {
        Self {
            entries: entries.into_iter().collect(),
            genesis_hash: genesis_hash.to_string(),
        }
    }

    pub fn from_ledger(ledger: &SignedLedger) -> Self {
        Self::new(
            ledger.entries().to_vec(),
            SignedLedger::GENESIS_HASH,
        )
    }

    pub fn replay_into_ledger(
        &self,
        verifier: &VerifyingKey,
    ) -> Result<SignedLedger, crate::signed_ledger::LedgerError> {
        let signing_key_bytes = [0u8; 32];
        let mut ledger = SignedLedger::from_signing_key_bytes(&signing_key_bytes);

        for entry in &self.entries {
            ledger.append_verified(entry.clone(), verifier)?;
        }

        Ok(ledger)
    }

    pub fn verify_replay_equivalence(
        &self,
        original: &SignedLedger,
        verifier: &VerifyingKey,
    ) -> Result<bool, crate::signed_ledger::LedgerError> {
        let replayed = self.replay_into_ledger(verifier)?;

        if original.entries().len() != replayed.entries().len() {
            return Ok(false);
        }

        for (a, b) in original.entries().iter().zip(replayed.entries().iter()) {
            if a.hash != b.hash || a.id != b.id {
                return Ok(false);
            }
        }

        Ok(original.last_hash() == replayed.last_hash())
    }

    pub fn entries(&self) -> &VecDeque<SignedEntry> {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
