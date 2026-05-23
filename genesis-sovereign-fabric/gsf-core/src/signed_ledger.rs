use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("prev_hash mismatch: expected {expected}, got {actual}")]
    PrevHashMismatch { expected: String, actual: String },
    #[error("invalid base64: {0}")]
    InvalidBase64(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub payload: serde_json::Value,
    pub prev_hash: String,
    pub hash: String,
    pub signature: String,
    pub signer: String,
}

impl SignedEntry {
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(self.payload.to_string().as_bytes());
        hasher.update(self.prev_hash.as_bytes());
        hasher.update(self.signer.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn verify_signature(&self, public_key: &VerifyingKey) -> Result<(), LedgerError> {
        let sig_bytes = BASE64
            .decode(&self.signature)
            .map_err(|e| LedgerError::InvalidBase64(e.to_string()))?;
        let sig = Signature::from_bytes(
            sig_bytes
                .as_slice()
                .try_into()
                .map_err(|_| LedgerError::SignatureVerificationFailed)?,
        );
        let msg = self.hash.as_bytes();
        public_key
            .verify(msg, &sig)
            .map_err(|_| LedgerError::SignatureVerificationFailed)
    }

    pub fn verify_chain(&self, prev_hash: &str) -> Result<(), LedgerError> {
        if self.prev_hash != prev_hash {
            return Err(LedgerError::PrevHashMismatch {
                expected: prev_hash.to_string(),
                actual: self.prev_hash.clone(),
            });
        }
        let computed = self.compute_hash();
        if self.hash != computed {
            return Err(LedgerError::SignatureVerificationFailed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SignedLedger {
    entries: Vec<SignedEntry>,
    genesis_hash: String,
    signing_key: SigningKey,
}

impl SignedLedger {
    pub const GENESIS_HASH: &'static str =
        "genesis_sha256_acb92fd8346a65ff17dbf9a41e3003f2d566a17f839af4c3a90a4b4b1789dd28a";

    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            entries: Vec::new(),
            genesis_hash: Self::GENESIS_HASH.to_string(),
            signing_key,
        }
    }

    pub fn from_signing_key_bytes(bytes: &[u8; 32]) -> Self {
        Self::new(SigningKey::from_bytes(bytes))
    }

    pub fn verify_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn append(
        &mut self,
        action: &str,
        payload: serde_json::Value,
    ) -> Result<SignedEntry, LedgerError> {
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| self.genesis_hash.clone());

        let timestamp = Utc::now();
        let id = {
            let mut hasher = Sha256::new();
            hasher.update(timestamp.to_rfc3339().as_bytes());
            hasher.update(prev_hash.as_bytes());
            hasher.update(action.as_bytes());
            format!("{:x}", hasher.finalize())
        };
        let signer = BASE64.encode(self.signing_key.verifying_key().as_bytes());

        let mut entry = SignedEntry {
            id,
            timestamp,
            action: action.to_string(),
            payload,
            prev_hash: prev_hash.clone(),
            hash: String::new(),
            signature: String::new(),
            signer,
        };
        entry.hash = entry.compute_hash();
        let sig = self.signing_key.sign(entry.hash.as_bytes());
        entry.signature = BASE64.encode(sig.to_bytes());

        entry.verify_chain(&prev_hash)?;
        self.entries.push(entry.clone());
        Ok(entry)
    }

    pub fn append_verified(&mut self, entry: SignedEntry, verifier: &VerifyingKey) -> Result<(), LedgerError> {
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| self.genesis_hash.clone());

        entry.verify_chain(&prev_hash)?;
        entry.verify_signature(verifier)?;
        self.entries.push(entry);
        Ok(())
    }

    pub fn entries(&self) -> &[SignedEntry] {
        &self.entries
    }

    pub fn last_hash(&self) -> &str {
        self.entries
            .last()
            .map(|e| e.hash.as_str())
            .unwrap_or(&self.genesis_hash)
    }

    pub fn verify_chain(&self, verifier: &VerifyingKey) -> Result<(), LedgerError> {
        let mut prev = &self.genesis_hash[..];
        for e in &self.entries {
            e.verify_chain(prev)?;
            e.verify_signature(verifier)?;
            prev = &e.hash;
        }
        Ok(())
    }

    pub fn replace_with_verified(
        &mut self,
        entries: Vec<SignedEntry>,
        verifier: &VerifyingKey,
    ) -> Result<(), LedgerError> {
        let mut prev = self.genesis_hash.clone();
        for e in &entries {
            e.verify_chain(&prev)?;
            e.verify_signature(verifier)?;
            prev = e.hash.clone();
        }
        self.entries = entries;
        Ok(())
    }
}
