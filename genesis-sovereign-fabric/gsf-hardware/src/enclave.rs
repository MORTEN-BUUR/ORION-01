use crate::attestation::{AttestationRequest, AttestationResponse, AttestationService};
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

#[derive(Clone)]
pub struct EnclaveAbstraction {
    attestation: AttestationService,
}

impl EnclaveAbstraction {
    pub fn new() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self {
            attestation: AttestationService::new(signing_key),
        }
    }

    pub fn with_key(signing_key: SigningKey) -> Self {
        Self {
            attestation: AttestationService::new(signing_key),
        }
    }

    pub fn attest(&self, request: &AttestationRequest) -> AttestationResponse {
        self.attestation.attest(&request.nonce)
    }
}

impl Default for EnclaveAbstraction {
    fn default() -> Self {
        Self::new()
    }
}
