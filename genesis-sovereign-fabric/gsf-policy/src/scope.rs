use crate::dsl::PolicyScope;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScopeError {
    #[error("action {0} not in scope")]
    ActionNotInScope(String),
    #[error("domain {0} not in scope")]
    DomainNotInScope(String),
}

pub struct ScopeEnforcement;

impl ScopeEnforcement {
    pub fn check_action(scope: &PolicyScope, action: &str) -> Result<(), ScopeError> {
        if scope.actions.is_empty() {
            return Ok(());
        }
        if scope.actions.iter().any(|a| a == action) {
            Ok(())
        } else {
            Err(ScopeError::ActionNotInScope(action.to_string()))
        }
    }

    pub fn check_domain(scope: &PolicyScope, domain: &str) -> Result<(), ScopeError> {
        if scope.domains.is_empty() {
            return Ok(());
        }
        if scope.domains.iter().any(|d| d == domain) {
            Ok(())
        } else {
            Err(ScopeError::DomainNotInScope(domain.to_string()))
        }
    }

    pub fn check_payload(
        scope: &PolicyScope,
        action: &str,
        payload: &Value,
    ) -> Result<(), ScopeError> {
        Self::check_action(scope, action)?;
        if let Some(domain) = payload.get("domain").and_then(|v: &serde_json::Value| v.as_str()) {
            Self::check_domain(scope, domain)?;
        }
        Ok(())
    }
}
