use crate::dsl::Invariant;
use serde_json::Value;
use std::collections::HashMap;

pub struct InvariantChecker;

impl InvariantChecker {
    pub fn check(invariants: &[Invariant], context: &HashMap<String, Value>) -> Vec<String> {
        let mut violations = Vec::new();
        for inv in invariants {
            if !inv.must_hold {
                continue;
            }
            if let Some(val) = context.get(&inv.expression) {
                if val == &Value::Bool(false) {
                    violations.push(inv.id.clone());
                }
            }
        }
        violations
    }
}
