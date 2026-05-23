use base64::Engine;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use gsf_core::{AuditChain, SignedLedger};
use gsf_euaiact::{OversightState, OversightType, PendingDecision};
use gsf_hardware::{AttestationRequest, EnclaveAbstraction};
use gsf_mesh::{MeshSync, PeerAllowlist};
use gsf_policy::{ActionValidator, Policy};
use gsf_registry::ModelRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub ledger: Arc<RwLock<SignedLedger>>,
    pub policy: Arc<Policy>,
    pub peer_allowlist: Arc<PeerAllowlist>,
    pub enclave: Arc<EnclaveAbstraction>,
    pub oversight: Arc<OversightState>,
    pub registry: Arc<ModelRegistry>,
}

#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub action: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub success: bool,
    pub entry_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MeshSyncRequest {
    pub entries: Vec<gsf_core::SignedEntry>,
    pub peer_fingerprint: String,
}

#[derive(Debug, Serialize)]
pub struct MeshSyncResponse {
    pub accepted: bool,
    pub merged_count: Option<usize>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditExportResponse {
    pub entries: Vec<serde_json::Value>,
    pub chain_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct AuditVerifyRequest {
    pub entries: Vec<gsf_core::SignedEntry>,
}

#[derive(Debug, Serialize)]
pub struct AuditVerifyResponse {
    pub valid: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AttestRequest {
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub ledger_length: usize,
    pub genesis_anchor: String,
}

pub async fn run_handler(
    State(state): State<AppState>,
    Json(req): Json<RunRequest>,
) -> impl IntoResponse {
    let context = HashMap::new();
    match ActionValidator::validate(&state.policy, &req.action, &req.payload, &context) {
        Ok(()) => {}
        Err(e) => {
            return (
                StatusCode::FORBIDDEN,
                Json(RunResponse {
                    success: false,
                    entry_id: None,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }

    let mut ledger = state.ledger.write().await;
    match ledger.append(&req.action, req.payload) {
        Ok(entry) => (
            StatusCode::OK,
            Json(RunResponse {
                success: true,
                entry_id: Some(entry.id.clone()),
                error: None,
            }),
        )
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RunResponse {
                success: false,
                entry_id: None,
                error: Some(e.to_string()),
            }),
        )
        .into_response(),
    }
}

pub async fn mesh_sync_handler(
    State(state): State<AppState>,
    Json(req): Json<MeshSyncRequest>,
) -> impl IntoResponse {
    if let Err(e) = MeshSync::verify_peer(&state.peer_allowlist, &req.peer_fingerprint) {
        return (
            StatusCode::FORBIDDEN,
            Json(MeshSyncResponse {
                accepted: false,
                merged_count: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response();
    }

    let verifier = state.ledger.read().await.verify_key();
    let local_entries: Vec<_> = state.ledger.read().await.entries().to_vec();

    match MeshSync::merge_chains(local_entries, req.entries, &verifier) {
        Ok(merged) => {
            let mut ledger = state.ledger.write().await;
            if ledger.replace_with_verified(merged.clone(), &verifier).is_err() {
                return (
                    StatusCode::CONFLICT,
                    Json(MeshSyncResponse {
                        accepted: false,
                        merged_count: None,
                        error: Some("replace failed".to_string()),
                    }),
                )
                    .into_response();
            }
            (
                StatusCode::OK,
                Json(MeshSyncResponse {
                    accepted: true,
                    merged_count: Some(merged.len()),
                    error: None,
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::CONFLICT,
            Json(MeshSyncResponse {
                accepted: false,
                merged_count: None,
                error: Some(e.to_string()),
            }),
        )
        .into_response(),
    }
}

pub async fn audit_export_handler(State(state): State<AppState>) -> impl IntoResponse {
    let ledger = state.ledger.read().await;
    let chain = AuditChain::new(ledger.clone());
    let entries = chain.export();
    let chain_hash = chain.verify_chain_hash();

    Json(AuditExportResponse {
        entries,
        chain_hash,
    })
}

pub async fn audit_verify_handler(
    State(state): State<AppState>,
    Json(req): Json<AuditVerifyRequest>,
) -> impl IntoResponse {
    let verifier = state.ledger.read().await.verify_key();
    let key = [0u8; 32];
    let mut ledger = SignedLedger::from_signing_key_bytes(&key);

    let verifier = req.entries.first().and_then(|e| {
        let decoded = base64::engine::general_purpose::STANDARD.decode(&e.signer).ok()?;
        let arr: [u8; 32] = decoded.as_slice().try_into().ok()?;
        ed25519_dalek::VerifyingKey::from_bytes(&arr).ok()
    }).unwrap_or(verifier);

    for e in &req.entries {
        if ledger.append_verified(e.clone(), &verifier).is_err() {
            return Json(serde_json::json!({
                "valid": false,
                "error": "signature or chain verification failed"
            }));
        }
    }

    Json(serde_json::json!({
        "valid": true,
        "error": null
    }))
}

pub async fn hardware_attest_handler(
    State(state): State<AppState>,
    Json(req): Json<AttestRequest>,
) -> impl IntoResponse {
    let response = state.enclave.attest(&AttestationRequest {
        nonce: req.nonce,
    });
    Json(response)
}

pub async fn system_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    let ledger = state.ledger.read().await;
    Json(StatusResponse {
        status: "operational".to_string(),
        ledger_length: ledger.entries().len(),
        genesis_anchor: gsf_core::SignedLedger::GENESIS_HASH.to_string(),
    })
}

#[derive(Debug, Deserialize)]
pub struct OversightSubmitRequest {
    pub action: String,
    pub payload: serde_json::Value,
    pub oversight_type: String,
}

#[derive(Debug, Serialize)]
pub struct OversightSubmitResponse {
    pub pending_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct OversightApproveRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct OversightHaltRequest {
    pub id: String,
    pub reason: String,
}

pub async fn oversight_submit_handler(
    State(state): State<AppState>,
    Json(req): Json<OversightSubmitRequest>,
) -> impl IntoResponse {
    let oversight_type = match req.oversight_type.as_str() {
        "human_on_the_loop" => OversightType::HumanOnTheLoop,
        "human_in_command" => OversightType::HumanInCommand,
        _ => OversightType::HumanInTheLoop,
    };
    let decision = PendingDecision::new(req.action, req.payload, oversight_type);
    let id = decision.id.clone();
    state.oversight.enqueue(decision);
    (
        StatusCode::OK,
        Json(OversightSubmitResponse {
            pending_id: id,
            status: "pending".to_string(),
        }),
    )
        .into_response()
}

pub async fn oversight_approve_handler(
    State(state): State<AppState>,
    Json(req): Json<OversightApproveRequest>,
) -> impl IntoResponse {
    match state.oversight.approve(&req.id) {
        Ok(decision) => {
            let context = HashMap::new();
            if let Err(e) = ActionValidator::validate(&state.policy, &decision.action, &decision.payload, &context) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(RunResponse {
                        success: false,
                        entry_id: None,
                        error: Some(e.to_string()),
                    }),
                )
                    .into_response();
            }
            let mut ledger = state.ledger.write().await;
            match ledger.append(&decision.action, decision.payload) {
                Ok(entry) => (
                    StatusCode::OK,
                    Json(RunResponse {
                        success: true,
                        entry_id: Some(entry.id.clone()),
                        error: None,
                    }),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(RunResponse {
                        success: false,
                        entry_id: None,
                        error: Some(e.to_string()),
                    }),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(RunResponse {
                success: false,
                entry_id: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}

pub async fn oversight_halt_handler(
    State(state): State<AppState>,
    Json(req): Json<OversightHaltRequest>,
) -> impl IntoResponse {
    match state.oversight.halt(&req.id) {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "halted", "reason": req.reason})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn oversight_pending_handler(State(state): State<AppState>) -> impl IntoResponse {
    let pending = state.oversight.list_pending();
    Json(serde_json::json!({"pending": pending}))
}

#[derive(Debug, Deserialize)]
pub struct RegistryRegisterRequest {
    pub id: String,
    pub version: String,
    pub sbom_hash: String,
    pub governance_flags: Vec<String>,
    pub annex_iii_category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegistryRegisterResponse {
    pub success: bool,
    pub risk_level: String,
    pub error: Option<String>,
}

pub async fn registry_register_handler(
    State(state): State<AppState>,
    Json(req): Json<RegistryRegisterRequest>,
) -> impl IntoResponse {
    let category = req.annex_iii_category.as_deref().and_then(|s| {
        use gsf_euaiact::AnnexIIICategory;
        match s {
            "biometric_identification" => Some(AnnexIIICategory::BiometricIdentification),
            "education" => Some(AnnexIIICategory::Education),
            "employment" => Some(AnnexIIICategory::Employment),
            "critical_infrastructure" => Some(AnnexIIICategory::CriticalInfrastructure),
            _ => None,
        }
    });
    match state.registry.register(
        req.id,
        req.version,
        req.sbom_hash,
        req.governance_flags,
        category,
    ) {
        Ok(entry) => (
            StatusCode::OK,
            Json(RegistryRegisterResponse {
                success: true,
                risk_level: format!("{:?}", entry.risk_level),
                error: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(RegistryRegisterResponse {
                success: false,
                risk_level: String::new(),
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct DisclosureRequest {
    pub risk_level: Option<String>,
}

pub async fn euaiact_disclosure_handler(
    req: Option<Json<DisclosureRequest>>,
) -> impl IntoResponse {
    let risk = req.as_ref().and_then(|r| r.risk_level.as_deref());
    let disclosure = match risk {
        Some("high") => gsf_euaiact::EndUserDisclosure::for_high_risk("See technical documentation."),
        _ => gsf_euaiact::EndUserDisclosure::for_limited_risk(),
    };
    Json(disclosure)
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/run", post(run_handler))
        .route("/mesh/sync", post(mesh_sync_handler))
        .route("/audit/export", get(audit_export_handler))
        .route("/audit/verify", post(audit_verify_handler))
        .route("/hardware/attest", post(hardware_attest_handler))
        .route("/system/status", get(system_status_handler))
        .route("/oversight/submit", post(oversight_submit_handler))
        .route("/oversight/approve", post(oversight_approve_handler))
        .route("/oversight/halt", post(oversight_halt_handler))
        .route("/oversight/pending", get(oversight_pending_handler))
        .route("/registry/register", post(registry_register_handler))
        .route("/euaiact/disclosure", post(euaiact_disclosure_handler))
        .with_state(state)
}
