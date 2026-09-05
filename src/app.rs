// ponytail: AppState is the single source of truth. UI takes &AppState (+ mutable textarea).
// Streaming state lives here so apply_event can mutate it; UI is read-only.

use std::collections::HashMap;

use crate::client::Session;

#[derive(Debug, Clone)]
pub enum Status {
    Idle,
    Busy,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: Role,
    pub text: String,
}

/// Per-part buffer — accumulates the streamed text for a single Part.
/// Mirror of what the server has, in case of catch-up events.
#[derive(Debug, Clone, Default)]
pub struct PartBuffer {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub session: Option<Session>,
    pub status: Status,
    pub messages: Vec<DisplayMessage>,
    pub input: String,

    // ── Phase 4 streaming fields ──
    /// part_id → accumulated text. Source of truth for the in-flight assistant message.
    pub parts: HashMap<String, PartBuffer>,
    /// part_id → message_id (resolved once on first sight).
    pub part_to_message: HashMap<String, String>,
    /// Index into `messages` of the message currently being streamed into.
    pub streaming_message_index: Option<usize>,
    /// Set on Esc-while-busy; the SSE handler clears this on SessionIdle.
    pub abort_requested: bool,
    /// Echoes the user text just sent — used to filter the server's replay of
    /// the same text as a `message.part.updated` for the user message.
    pub last_user_text: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            session: None,
            status: Status::Idle,
            messages: Vec::new(),
            input: String::new(),
            parts: HashMap::new(),
            part_to_message: HashMap::new(),
            streaming_message_index: None,
            abort_requested: false,
            last_user_text: None,
        }
    }
}
