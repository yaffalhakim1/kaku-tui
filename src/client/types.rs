// ponytail: minimal types for Phase 1 (health + create_session).
// Expanded in Phase 4 to cover full Event + Part union (needed for streaming).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Health {
    #[allow(dead_code)]
    pub healthy: bool,
    #[allow(dead_code)]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    pub directory: String,
    pub title: String,
    pub version: String,
    pub time: SessionTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTime {
    pub created: u64,
    pub updated: u64,
}

// =====================================================================
// Phase 3 — sync prompt response
// =====================================================================
//
// opencode returns a fully-formed AssistantMessage + its parts.
// We don't model it as a discriminated union — for v0 we only care
// about the assistant text. Tool/Reasoning parts land in Phase 4
// alongside the streaming rewrite.
//
// ponytail: extend the type when we need a field, not before.

#[derive(Debug, Clone, Deserialize)]
pub struct PromptResponse {
    pub info: MessageInfo,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "role")]
pub enum MessageInfo {
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantMessage {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub session_id: String,
    #[allow(dead_code)]
    pub parent_id: String,
}

// Discriminated union — opencode returns one of these per part.
// We only handle Text in Phase 3. Phase 4 keeps this union and
// branches on `type` to handle tool calls etc.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename = "text")]
    Text(TextPart),
    #[serde(rename = "tool")]
    Tool(serde_json::Value),
    #[serde(rename = "reasoning")]
    Reasoning(serde_json::Value),
    #[serde(rename = "step-start")]
    StepStart(serde_json::Value),
    #[serde(rename = "step-finish")]
    StepFinish(serde_json::Value),
    #[serde(other)]
    Other,
}

impl Part {
    pub fn text(&self) -> Option<&str> {
        match self {
            Part::Text(t) => Some(&t.text),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextPart {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
    pub text: String,
    // synthetic + ignored are flags used by opencode; we don't act on them.
    #[serde(default)]
    #[allow(dead_code)]
    pub synthetic: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    pub ignored: Option<bool>,
}

// ── Reference types (lightweight — used only for builder ergonomics) ──

/// (providerID, modelID) pair sent on per-prompt `model` overrides.
/// Opencode's body shape:
///   { "model": { "providerID": "anthropic", "modelID": "claude-opus-4-5" } }
#[derive(Debug, Clone)]
pub struct ModelRef {
    pub provider_id: String,
    pub model_id: String,
}

impl ModelRef {
    pub fn new(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
        }
    }
    /// Parse `provider/model` shorthand. Returns None when the slash is
    /// missing — caller decides whether to error.
    pub fn parse(s: &str) -> Option<Self> {
        let (p, m) = s.split_once('/')?;
        Some(Self::new(p.trim(), m.trim()))
    }
    /// Render back to `provider/model` for display.
    pub fn display(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }
}
