//! EU AI Act Article 14: Human oversight.
//! Human-in-the-loop, human-on-the-loop, human-in-command.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;

/// Oversight mode per Article 14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OversightType {
    /// Human reviews and approves before each decision.
    HumanInTheLoop,

    /// Human monitors with ability to intervene.
    HumanOnTheLoop,

    /// Human maintains ultimate decision authority.
    HumanInCommand,
}

/// Pending decision awaiting human approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDecision {
    pub id: String,
    pub action: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub oversight_type: OversightType,
    pub context: HashMap<String, serde_json::Value>,
}

impl PendingDecision {
    pub fn new(action: impl Into<String>, payload: serde_json::Value, oversight_type: OversightType) -> Self {
        let action_s = action.into();
        let mut hasher = Sha256::new();
        hasher.update(action_s.as_bytes());
        hasher.update(payload.to_string().as_bytes());
        hasher.update(Utc::now().to_rfc3339().as_bytes());
        let id = format!("{:x}", hasher.finalize());
        Self {
            id: id.clone(),
            action: action_s,
            payload,
            created_at: Utc::now(),
            oversight_type,
            context: HashMap::new(),
        }
    }
}

/// Outcome of human oversight.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OversightOutcome {
    Approved,
    Overridden { reason: String },
    Halted { reason: String },
}

/// State of oversight queue.
#[derive(Debug, Default)]
pub struct OversightState {
    pending: std::sync::RwLock<Vec<PendingDecision>>,
}

impl OversightState {
    pub fn new() -> Self {
        Self {
            pending: std::sync::RwLock::new(Vec::new()),
        }
    }

    pub fn enqueue(&self, decision: PendingDecision) -> String {
        let id = decision.id.clone();
        self.pending.write().unwrap().push(decision);
        id
    }

    pub fn get(&self, id: &str) -> Option<PendingDecision> {
        self.pending
            .read()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
    }

    pub fn approve(&self, id: &str) -> Result<PendingDecision, OversightError> {
        let mut pending = self.pending.write().unwrap();
        let pos = pending.iter().position(|d| d.id == id);
        match pos {
            Some(i) => Ok(pending.remove(i)),
            None => Err(OversightError::NotFound(id.to_string())),
        }
    }

    /// Remove without executing (Article 14: halt/override).
    pub fn halt(&self, id: &str) -> Result<PendingDecision, OversightError> {
        self.approve(id)
    }

    pub fn list_pending(&self) -> Vec<PendingDecision> {
        self.pending.read().unwrap().clone()
    }
}

#[derive(Debug, Error)]
pub enum OversightError {
    #[error("pending decision not found: {0}")]
    NotFound(String),
}
