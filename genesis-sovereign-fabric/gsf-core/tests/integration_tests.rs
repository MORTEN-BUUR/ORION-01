use ed25519_dalek::SigningKey;
use gsf_core::*;

#[test]
fn test_signed_ledger_append_and_verify() {
    let key = SigningKey::from_bytes(&[1u8; 32]);
    let mut ledger = SignedLedger::new(key.clone());
    let verifier = ledger.verify_key();

    let e1 = ledger.append("action1", serde_json::json!({"x": 1})).unwrap();
    let e2 = ledger.append("action2", serde_json::json!({"y": 2})).unwrap();

    assert!(ledger.verify_chain(&verifier).is_ok());
    assert_eq!(ledger.entries().len(), 2);
    assert_eq!(e1.action, "action1");
    assert_eq!(e2.action, "action2");
}

#[test]
fn test_policy_deny() {
    use gsf_policy::{ActionValidator, Policy};
    use gsf_policy::dsl::{PolicyScope, Rule, RuleEffect, Condition};

    let policy = Policy {
        version: "1.0".to_string(),
        scope: PolicyScope {
            domains: vec![],
            actions: vec!["run".to_string()],
            deny_on_match: false,
        },
        rules: vec![Rule {
            id: "deny".to_string(),
            action: "run".to_string(),
            effect: RuleEffect::Deny,
            conditions: vec![Condition {
                field: "domain".to_string(),
                op: "eq".to_string(),
                value: serde_json::json!("forbidden"),
            }],
        }],
        invariants: vec![],
    };

    let mut context = std::collections::HashMap::new();
    let result = ActionValidator::validate(
        &policy,
        "run",
        &serde_json::json!({"domain": "forbidden"}),
        &context,
    );
    assert!(result.is_err());
}

#[test]
fn test_replay_equivalence() {
    let key = SigningKey::from_bytes(&[2u8; 32]);
    let mut ledger = SignedLedger::new(key.clone());
    let verifier = ledger.verify_key();

    ledger.append("a", serde_json::json!({})).unwrap();
    ledger.append("b", serde_json::json!({})).unwrap();

    let engine = ReplayEngine::from_ledger(&ledger);
    let equiv = engine.verify_replay_equivalence(&ledger, &verifier).unwrap();
    assert!(equiv);
}
