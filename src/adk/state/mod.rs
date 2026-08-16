use crate::schema::Message;
use std::collections::HashMap;

pub struct AgentState {
    pub messages: Vec<Message>,
    pub extra: HashMap<String, serde_json::Value>,
    remaining_iterations: usize,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            extra: HashMap::new(),
            remaining_iterations: 10,
        }
    }

    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.remaining_iterations = max;
        self
    }

    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn decrement_iterations(&mut self) -> bool {
        if self.remaining_iterations == 0 {
            return false;
        }
        self.remaining_iterations -= 1;
        true
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}
