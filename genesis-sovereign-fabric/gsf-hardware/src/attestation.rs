use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationRequest {
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationResponse {
    pub nonce: String,
    pub statement: String,
    pub signature: String,
    pub public_key: String,
}

#[derive(Debug, Error)]
pub enum AttestationError {
    #[error("signature verification failed")]
    VerificationFailed,
}

#[derive(Clone)]
pub struct AttestationService {
    signing_key: SigningKey,
}

impl AttestationService {
    pub fn new(signing_key: SigningKey) -> Self {
        Self { signing_key }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self::new(SigningKey::from_bytes(bytes))
    }

    pub fn attest(&self, nonce: &str) -> AttestationResponse {
        let statement = Self::build_statement(nonce);
        let sig = self.signing_key.sign(statement.as_bytes());
        let pk = self.signing_key.verifying_key();

        AttestationResponse {
            nonce: nonce.to_string(),
            statement: statement.clone(),
            signature: BASE64.encode(sig.to_bytes()),
            public_key: BASE64.encode(pk.as_bytes()),
        }
    }

    fn build_statement(nonce: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"genesis_attest");
        hasher.update(nonce.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        format!("attest:nonce={}:hash={}", nonce, hash)
    }

    pub fn verify(response: &AttestationResponse) -> Result<(), AttestationError> {
        let pk_bytes = BASE64
            .decode(&response.public_key)
            .map_err(|_| AttestationError::VerificationFailed)?;
        let pk = VerifyingKey::from_bytes(
            pk_bytes
                .as_slice()
                .try_into()
                .map_err(|_| AttestationError::VerificationFailed)?,
        )
        .map_err(|_| AttestationError::VerificationFailed)?;

        let sig_bytes = BASE64
            .decode(&response.signature)
            .map_err(|_| AttestationError::VerificationFailed)?;
        let sig = Signature::from_bytes(
            sig_bytes
                .as_slice()
                .try_into()
                .map_err(|_| AttestationError::VerificationFailed)?,
        );

        pk.verify(response.statement.as_bytes(), &sig)
            .map_err(|_| AttestationError::VerificationFailed)
    }
}
