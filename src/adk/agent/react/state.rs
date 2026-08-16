//! ReAct Agent State Management
//!
//! This module provides the state structure and pre-handlers for the Graph-based ReAct Agent.

use crate::compose::runnable::Context;
use crate::schema::Message;

/// ReAct state that persists across agent iterations.
///
/// This state is passed through the Graph and modified by pre-handlers
/// at each node execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReActState {
    /// All messages accumulated so far in the conversation
    pub messages: Vec<Message>,
    /// If set, the agent should return directly without calling the model again
    pub return_directly_tool_call_id: Option<String>,
}

impl Default for ReActState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReActState {
    /// Creates a new empty ReActState
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            return_directly_tool_call_id: None,
        }
    }

    /// Clear all messages, keeping only system prompt if present
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }
}

/// PreHandler for ChatModel node: appends input messages to state
///
/// # Arguments
/// * `_ctx` - The runtime context (unused in this handler)
/// * `input` - Input messages to append to state
/// * `state` - The ReAct state to update
///
/// # Returns
/// The updated messages vector for passing to the chat model
pub fn chat_model_pre_handler(
    _ctx: &Context,
    input: &[Message],
    state: &mut ReActState,
) -> Result<Vec<Message>, String> {
    state.messages.extend(input.iter().cloned());
    Ok(state.messages.clone())
}

/// PreHandler for Tools node: appends tool result message to state
///
/// # Arguments
/// * `_ctx` - The runtime context (unused in this handler)
/// * `input` - Tool result message to append to state
/// * `state` - The ReAct state to update
///
/// # Returns
/// The tool result message for passing to the next node
pub fn tools_node_pre_handler(
    _ctx: &Context,
    input: &Message,
    state: &mut ReActState,
) -> Result<Message, String> {
    state.messages.push(input.clone());

    // Check if any tool call has return_directly flag set
    if let Some(tool_calls) = &input.tool_calls {
        for tc in tool_calls {
            // Check function.arguments for return_directly flag
            if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                if args
                    .get("return_directly")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    state.return_directly_tool_call_id = Some(tc.id.clone());
                    break;
                }
            }
        }
    }

    Ok(input.clone())
}

/// PreHandler for conditional branch: checks if message has tool calls
///
/// # Arguments
/// * `_ctx` - The runtime context (unused)
/// * `input` - The message to check
/// * `state` - The ReAct state (unused)
///
/// # Returns
/// Some("tools".to_string()) if tool calls exist, None otherwise
pub fn has_tool_calls_branch(
    _ctx: &Context,
    input: &Message,
    _state: &mut ReActState,
) -> Result<Option<String>, String> {
    if input.tool_calls.is_some() && !input.tool_calls.as_ref().unwrap().is_empty() {
        Ok(Some("tools".to_string()))
    } else {
        Ok(None) // Go to END
    }
}

/// PreHandler for tools return_directly check
///
/// # Arguments
/// * `_ctx` - The runtime context (unused)
/// * `input` - The tool result message (unused)
/// * `state` - The ReAct state to check
///
/// # Returns
/// Some("direct_return".to_string()) if should return directly, None to continue to chat
pub fn return_directly_branch(
    _ctx: &Context,
    _input: &Message,
    state: &mut ReActState,
) -> Result<Option<String>, String> {
    if state.return_directly_tool_call_id.is_some() {
        state.return_directly_tool_call_id = None; // Clear after reading
        Ok(Some("direct_return".to_string()))
    } else {
        Ok(None) // Continue to chat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Content, FunctionCall, RoleType, ToolCall};

    #[test]
    fn test_react_state_new() {
        let state = ReActState::new();
        assert!(state.messages.is_empty());
        assert!(state.return_directly_tool_call_id.is_none());
    }

    #[test]
    fn test_chat_model_pre_handler() {
        let mut state = ReActState::new();
        let ctx = Context::new();

        let input = vec![Message::user("Hello"), Message::assistant("Hi there!")];

        let result = chat_model_pre_handler(&ctx, &input, &mut state).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(state.messages.len(), 2);
    }

    #[test]
    fn test_tools_node_pre_handler_with_tool_calls() {
        let mut state = ReActState::new();
        let ctx = Context::new();

        let message = Message {
            role: RoleType::Assistant,
            content: Content::Text("calling tool".to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "my_tool".to_string(),
                    arguments: r#"{"return_directly": true}"#.to_string(),
                },
            }]),
        };

        let result = tools_node_pre_handler(&ctx, &message, &mut state).unwrap();
        let _ = result;
        assert!(state.return_directly_tool_call_id.is_some());
        assert_eq!(
            state.return_directly_tool_call_id.as_ref().unwrap(),
            "call_1"
        );
    }

    #[test]
    fn test_has_tool_calls_branch_true() {
        let mut state = ReActState::new();
        let ctx = Context::new();

        let message = Message {
            role: RoleType::Assistant,
            content: Content::Text("calling tool".to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "my_tool".to_string(),
                    arguments: "{}".to_string(),
                },
            }]),
        };

        let result = has_tool_calls_branch(&ctx, &message, &mut state).unwrap();
        assert_eq!(result, Some("tools".to_string()));
    }

    #[test]
    fn test_has_tool_calls_branch_false() {
        let mut state = ReActState::new();
        let ctx = Context::new();

        let message = Message::assistant("Hello!");

        let result = has_tool_calls_branch(&ctx, &message, &mut state).unwrap();
        assert_eq!(result, None);
    }
}
