use prometheus::{
    histogram_opts, opts, register_counter_vec, register_histogram_vec, register_int_gauge_vec,
    CounterVec, HistogramVec, IntGaugeVec,
};
use std::sync::Once;

static INIT: Once = Once::new();

#[derive(Clone)]
pub struct Metrics {
    pub policy_decisions: CounterVec,
    pub policy_denies: CounterVec,
    pub mesh_rejects: CounterVec,
    pub signature_verifications: CounterVec,
    pub replay_operations: CounterVec,
    pub request_latency: HistogramVec,
    pub ledger_length: IntGaugeVec,
}

impl Metrics {
    pub fn init() -> Self {
        INIT.call_once(|| {});

        let policy_decisions = register_counter_vec!(
            opts!("gsf_policy_decisions_total", "Total policy decisions"),
            &["action", "result"]
        )
        .expect("metrics init");

        let policy_denies = register_counter_vec!(
            opts!("gsf_policy_denies_total", "Total policy denials"),
            &["reason"]
        )
        .expect("metrics init");

        let mesh_rejects = register_counter_vec!(
            opts!("gsf_mesh_rejects_total", "Total mesh rejections"),
            &["reason"]
        )
        .expect("metrics init");

        let signature_verifications = register_counter_vec!(
            opts!("gsf_signature_verifications_total", "Total signature verifications"),
            &["result"]
        )
        .expect("metrics init");

        let replay_operations = register_counter_vec!(
            opts!("gsf_replay_operations_total", "Total replay operations"),
            &["result"]
        )
        .expect("metrics init");

        let request_latency = register_histogram_vec!(
            histogram_opts!("gsf_request_latency_seconds", "Request latency in seconds"),
            &["endpoint"]
        )
        .expect("metrics init");

        let ledger_length = register_int_gauge_vec!(
            opts!("gsf_ledger_length", "Current ledger length"),
            &["node"]
        )
        .expect("metrics init");

        Self {
            policy_decisions,
            policy_denies,
            mesh_rejects,
            signature_verifications,
            replay_operations,
            request_latency,
            ledger_length,
        }
    }
}
