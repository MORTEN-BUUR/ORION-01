use crate::fork_resolution::ForkResolver;
use crate::peer::PeerAllowlist;
use gsf_core::signed_ledger::SignedEntry;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("peer not in allowlist")]
    PeerNotAllowed,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("broken prev_hash chain")]
    BrokenChain,
    #[error("fork resolution: no valid chain")]
    NoValidChain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshSyncPayload {
    pub entries: Vec<SignedEntry>,
    pub peer_fingerprint: String,
}

pub struct MeshSync;

impl MeshSync {
    pub fn verify_peer(allowlist: &PeerAllowlist, fingerprint: &str) -> Result<(), SyncError> {
        if allowlist.is_empty() || allowlist.contains(fingerprint) {
            Ok(())
        } else {
            Err(SyncError::PeerNotAllowed)
        }
    }

    pub fn merge_chains(
        local_entries: Vec<SignedEntry>,
        remote_entries: Vec<SignedEntry>,
        verifier: &ed25519_dalek::VerifyingKey,
    ) -> Result<Vec<SignedEntry>, SyncError> {
        let local_ledger = Self::entries_to_ledger(&local_entries, verifier);
        let remote_ledger = Self::entries_to_ledger(&remote_entries, verifier);

        let idx = ForkResolver::select_longest_valid(
            vec![&local_ledger, &remote_ledger],
            verifier,
        )
        .ok_or(SyncError::NoValidChain)?;

        Ok(if idx == 0 {
            local_entries
        } else {
            remote_entries
        })
    }

    fn entries_to_ledger(
        entries: &[SignedEntry],
        verifier: &ed25519_dalek::VerifyingKey,
    ) -> gsf_core::SignedLedger {
        let key = [0u8; 32];
        let mut ledger = gsf_core::SignedLedger::from_signing_key_bytes(&key);
        for e in entries {
            let _ = ledger.append_verified(e.clone(), verifier);
        }
        ledger
    }
}
