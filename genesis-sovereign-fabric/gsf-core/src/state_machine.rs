use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub action: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum StateMachineError {
    #[error("invalid transition: {from} -> {to} for action {action}")]
    InvalidTransition {
        from: String,
        to: String,
        action: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct StateMachine {
    current: String,
    transitions: Vec<(String, String, String)>,
}

impl StateMachine {
    pub fn new(initial: &str) -> Self {
        Self {
            current: initial.to_string(),
            transitions: Vec::new(),
        }
    }

    pub fn from_state(state: &str) -> Self {
        Self {
            current: state.to_string(),
            transitions: Vec::new(),
        }
    }

    pub fn allow_transition(&mut self, from: &str, to: &str, action: &str) {
        self.transitions
            .push((from.to_string(), to.to_string(), action.to_string()));
    }

    pub fn current(&self) -> &str {
        &self.current
    }

    pub fn apply(
        &mut self,
        action: &str,
        payload: serde_json::Value,
    ) -> Result<StateTransition, StateMachineError> {
        let from = self.current.clone();
        let to = self
            .transitions
            .iter()
            .find(|(f, _t, a)| f == &from && a == action)
            .map(|(_, t, _)| t.clone())
            .unwrap_or_else(|| from.clone());

        if !self.transitions.iter().any(|(f, t, a)| f == &from && t == &to && a == action)
            && to != from
        {
            return Err(StateMachineError::InvalidTransition {
                from: from.clone(),
                to: to.clone(),
                action: action.to_string(),
            });
        }

        self.current = to.clone();
        Ok(StateTransition {
            from,
            to,
            action: action.to_string(),
            payload,
        })
    }

    pub fn set_state(&mut self, state: &str) {
        self.current = state.to_string();
    }
}
