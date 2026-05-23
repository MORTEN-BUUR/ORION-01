use crate::dsl::{Condition, Policy, RuleEffect};
use crate::scope::ScopeEnforcement;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("action denied by policy")]
    Denied,
    #[error("action not in scope: {0}")]
    NotInScope(String),
    #[error("invariant violated: {0}")]
    InvariantViolated(String),
}

pub struct ActionValidator;

impl ActionValidator {
    pub fn validate(
        policy: &Policy,
        action: &str,
        payload: &Value,
        context: &HashMap<String, Value>,
    ) -> Result<(), ValidationError> {
        ScopeEnforcement::check_payload(&policy.scope, action, payload)
            .map_err(|e| ValidationError::NotInScope(e.to_string()))?;

        let violations = crate::invariants::InvariantChecker::check(&policy.invariants, context);
        if !violations.is_empty() {
            return Err(ValidationError::InvariantViolated(violations.join(", ")));
        }

        let mut matched_deny = false;
        let mut matched_allow = false;

        for rule in &policy.rules {
            if rule.action != "*" && rule.action != action {
                continue;
            }
            if Self::conditions_match(&rule.conditions, payload) {
                match rule.effect {
                    RuleEffect::Allow => matched_allow = true,
                    RuleEffect::Deny => matched_deny = true,
                }
            }
        }

        if policy.scope.deny_on_match && matched_deny {
            return Err(ValidationError::Denied);
        }
        if matched_deny && !matched_allow {
            return Err(ValidationError::Denied);
        }

        Ok(())
    }

    fn conditions_match(conditions: &[Condition], payload: &Value) -> bool {
        for cond in conditions {
            let field_val = payload.get(&cond.field);
            if !Self::eval_condition(field_val, &cond.op, &cond.value) {
                return false;
            }
        }
        true
    }

    fn eval_condition(field_val: Option<&Value>, op: &str, expected: &Value) -> bool {
        let fv = match field_val {
            Some(v) => v,
            None => return false,
        };
        match op {
            "eq" | "==" => fv == expected,
            "ne" | "!=" => fv != expected,
            "contains" => {
                if let (Some(a), Some(b)) = (fv.as_str(), expected.as_str()) {
                    a.contains(b)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
