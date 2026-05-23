use crate::routes::{create_router, AppState};
use gsf_hardware::EnclaveAbstraction;
use gsf_mesh::PeerAllowlist;
use gsf_policy::dsl::{Policy, PolicyScope, Rule, RuleEffect};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

fn default_policy() -> gsf_policy::Policy {
    Policy {
        version: "1.0".to_string(),
        scope: PolicyScope {
            domains: vec![],
            actions: vec![],
            deny_on_match: false,
        },
        rules: vec![Rule {
            id: "default".to_string(),
            action: "*".to_string(),
            effect: RuleEffect::Allow,
            conditions: vec![],
        }],
        invariants: vec![],
    }
}

pub async fn run_server(addr: SocketAddr) {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
    let ledger = gsf_core::SignedLedger::new(signing_key);

    let state = AppState {
        ledger: Arc::new(RwLock::new(ledger)),
        policy: Arc::new(default_policy()),
        peer_allowlist: Arc::new(PeerAllowlist::from_env("GSF_PEER_ALLOWLIST")),
        enclave: Arc::new(EnclaveAbstraction::new()),
        oversight: Arc::new(gsf_euaiact::OversightState::new()),
        registry: Arc::new(gsf_registry::ModelRegistry::new()),
    };

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
