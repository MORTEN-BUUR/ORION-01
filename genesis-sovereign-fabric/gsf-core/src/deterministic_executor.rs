use crate::state_machine::{StateMachine, StateTransition};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct DeterministicExecutor {
    state_machine: StateMachine,
    context: HashMap<String, Value>,
}

impl DeterministicExecutor {
    pub fn new(initial_state: &str) -> Self {
        Self {
            state_machine: StateMachine::new(initial_state),
            context: HashMap::new(),
        }
    }

    pub fn with_transition(mut self, from: &str, to: &str, action: &str) -> Self {
        self.state_machine.allow_transition(from, to, action);
        self
    }

    pub fn execute(
        &mut self,
        action: &str,
        payload: Value,
    ) -> Result<StateTransition, crate::state_machine::StateMachineError> {
        self.state_machine.apply(action, payload)
    }

    pub fn current_state(&self) -> &str {
        self.state_machine.current()
    }

    pub fn set_context(&mut self, key: String, value: Value) {
        self.context.insert(key, value);
    }

    pub fn get_context(&self, key: &str) -> Option<&Value> {
        self.context.get(key)
    }

    pub fn context(&self) -> &HashMap<String, Value> {
        &self.context
    }
}
