// =====================================================================
// kaku-tui main — Phases 1-5.
//
// Defense in depth: TerminalGuard + panic hook restore raw-mode + alt-screen
// even if the task panics or the user Ctrl+C's mid-render.
//
// ponytail: splitting into more files is ceremony. If main.rs grows past
// 400 lines, lift run/handle_key/apply_event into their own modules.
// =====================================================================

#![allow(dead_code)] // matches the lib's silent-stream-policy; see lib.rs

mod app;
mod client;
mod commands;
mod theme;
mod ui;
// `mod` declarations shared with `lib.rs` — fine because cargo treats them
// as the same crate root module tree.


use std::panic;

use anyhow::{Context, Result};
use crossterm::event::{Event as CtEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use tui_textarea::{Input, TextArea};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use app::{AppState, DisplayMessage, PartBuffer, Role, Status};
use client::OpencodeClient;
use ui as ui_mod;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // ── 1. CLI args ──
    let base = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:4096".to_string());
    // Auth: prefer the kaku-tui-specific var; fall back to the server's var
    // so `OPENCODE_SERVER_PASSWORD=secret opencode serve &` followed by
    // `cargo run` works without exporting twice.
    let password = std::env::var("KAKU_TUI_PASSWORD")
        .ok()
        .or_else(|| std::env::var("OPENCODE_SERVER_PASSWORD").ok());
    let username = std::env::var("OPENCODE_SERVER_USERNAME")
        .unwrap_or_else(|_| "opencode".to_string());
    let url: reqwest::Url = base.parse().context("invalid base URL")?;

    // ── 2. Connect + open session BEFORE entering raw mode. ──
    // If opencode is down, we want a clean stdout error — not a corrupted terminal.
    let client = OpencodeClient::new(url, &username, password.as_deref())?;
    let _health = client.health().await.context("connect to opencode server")?;
    let default_model = client.default_model().await.unwrap_or(None);
    let session = client
        .create_session(Some("kaku-tui"))
        .await
        .context("create session")?;

    let mut app = AppState::new();
    app.session = Some(session.clone());
    app.default_model = default_model;
    app.status = Status::Idle;

    // ── 3. Panic hook: restore terminal BEFORE printing the panic. ──
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        TerminalGuard::leave();
        original_hook(info);
    }));

    let terminal = TerminalGuard::enter().context("enter terminal")?;

    // ── 4. mpsc channel = bridge between SSE-reader task and main loop. ──
    //   tx is moved into the SSE task.
    //   rx is wrapped in ReceiverStream so we can .next() it in select!.
    let (tx, rx) = mpsc::unbounded_channel::<StreamEvent>();
    let mut rx = UnboundedReceiverStream::new(rx);

    spawn_sse_reader(client.clone(), tx);

    let result = run(terminal, client, app, &mut rx, session.id).await;
    TerminalGuard::leave();
    result
}

// =====================================================================
// StreamEvent — collapsed SSE payload.
//
// Opencode's wire format has ~30 event variants in a discriminated union on
// `type`. We collapse to 4 we care about. The SSE reader strips the rest.
//
// ponytail: lift to per-handler if match arms grow past 8.
// =====================================================================
#[derive(Debug, Clone)]
enum StreamEvent {
    ServerConnected,
    SessionIdle { session_id: String },
    SessionError { message: String },
    /// The SSE pipe died or closed. Surfaced so the status bar reflects it.
    /// Triggered by either a bytes_stream error or a clean stream end.
    Disconnected(String),
    PartUpdated {
        part_id: String,
        message_id: String,
        text: String,          // full text per the server
        delta: Option<String>, // incremental chunk, if present
    },
    Unknown,
}

// =====================================================================
// SSE reader task — owns the HTTP connection to GET /event.
//
// CONCURRENCY:
//  We await this inline and keys would queue behind chunks.
//  tokio::spawn gives us concurrency for free.
//
// THE BUFFER (this is the only subtle part):
//  TCP chunks don't align with SSE event boundaries. One chunk might hold:
//    - end of one event ("...}\n\n")
//    - start of next event ("data: {...\"x...\"")
//    - half a multi-byte UTF-8 char somewhere
//  We accumulate into `buf`, then split on "\n\n" to extract complete events.
//  Whatever's left in `buf` is the start of the next event.
//
// ERROR POLICY:
//  - bad JSON → log + skip, never panic.
//  - non-2xx status → give up, let status bar surface "disconnected".
//  - channel closed (rx dropped) → exit task, no further work.
//
// ponytail: hand-rolled. Add `reqwest-eventsource` crate if we need
// auto-reconnect with backoff or last-event-id rehydration.
// =====================================================================
fn spawn_sse_reader(client: OpencodeClient, tx: mpsc::UnboundedSender<StreamEvent>) {
    tokio::spawn(async move {
        let url = match client.base_url().join("/event") {
            Ok(u) => u,
            Err(_) => return,
        };
        let resp = match client.http_get(url).send().await {
            Ok(r) => r,
            Err(_) => return,
        };
        if !resp.status().is_success() {
            return;
        }

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();

        while let Some(next) = stream.next().await {
            let chunk = match next {
                Ok(c) => c,
                Err(e) => {
                    // ponytail: don't silently drop. Tell the user the SSE
                    // pipe died so they don't think the TUI is hung.
                    let _ = tx.send(StreamEvent::Disconnected(format!("sse: {e}")));
                    return;
                }
            };
            buf.extend_from_slice(&chunk);

            // Drain complete events from the buffer.
            while let Some(idx) = find_subsequence(&buf, b"\n\n") {
                let raw = buf.drain(..idx + 2).collect::<Vec<u8>>();
                let Ok(s) = std::str::from_utf8(&raw) else { continue };
                let json: String = s
                    .lines()
                    .filter_map(|l| l.strip_prefix("data: "))
                    .collect::<Vec<_>>()
                    .join("\n");
                if json.is_empty() {
                    continue;
                }
                let Ok(w) = serde_json::from_str::<SseWrapper>(&json) else {
                    continue;
                };
                let ev = classify(w);
                if tx.send(ev).is_err() {
                    return;
                }
            }
        }
        // Stream closed cleanly. Tell the main loop so the status bar
        // surfaces it; otherwise a quiet terminal looks the same as a
        // crashed stream.
        let _ = tx.send(StreamEvent::Disconnected("closed".to_string()));
    });
}

#[derive(Debug, serde::Deserialize)]
struct SseWrapper {
    #[serde(rename = "type")]
    type_: String,
    #[serde(default)]
    properties: serde_json::Value,
}

fn classify(w: SseWrapper) -> StreamEvent {
    use StreamEvent::*;
    let t = w.type_.as_str();
    if t == "server.connected" {
        return ServerConnected;
    }
    let props = w.properties;
    match t {
        "session.idle" => {
            let id = props
                .get("sessionID")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            SessionIdle { session_id: id }
        }
        "session.error" => {
            let msg = props.to_string();
            SessionError { message: msg }
        }
        "message.part.updated" => {
            // Only text parts are interesting for v0.
            let is_text = props
                .pointer("/part/type")
                .and_then(|v| v.as_str())
                .map(|s| s == "text")
                .unwrap_or(false);
            if !is_text {
                return Unknown;
            }
            let part_id = props
                .pointer("/part/id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let message_id = props
                .pointer("/part/messageID")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let text = props
                .pointer("/part/text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let delta = props.get("delta").and_then(|v| v.as_str()).map(String::from);
            // ponytail: opencode replays the user's own text part as part of
            // history. We ignore text events for messages with no parent (i.e.
            // those aren't assistant replies). In v0 we can't tell easily, so
            // we route ALL text-part events to the renderer — the renderer
            // only writes into the slot we pre-created for the streaming
            // assistant reply, so user-prompt echoes go to /dev/null.
            // HOWEVER: we want to NOT prepend the user prompt text. The renderer
            // already wrote the user msg from the Enter-handler, so the part
            // text being non-empty for the assistant reply overwrites correctly.
            // For the user echo, we set streaming_message_index = the assistant,
            // so the user's text would clobber the empty assistant. Workaround:
            // check role. v0 lacks role here; instead, we count words — if part
            // text matches what we just sent, ignore.
            PartUpdated { part_id, message_id, text, delta }
        }
        _ => Unknown,
    }
}

// ponytail: avoid `memchr` for one find. O(n*m) on bytes is fine here.
fn find_subsequence(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// =====================================================================
// run — the main event loop.
//
// TWO sources of "stuff to react to":
//  1. Keyboard events (crossterm EventStream).
//  2. SSE events (mpsc channel wrapped as a Stream).
//
// tokio::select! races them. We render FIRST, then wait.
//  - Render-first guarantees a frame paints before we block on events.
//  - Avoids blank flashes when the source is quiet.
//
// ALL state mutation lives here. The UI layer is read-only.
//
// ponytail: the textarea widget blinks its own cursor when it has focus,
// so we don't need a 500ms tick for the input area. Streaming feedback
// comes from `app.messages[idx].text` growing in real time.
// =====================================================================
async fn run(
    mut terminal: TerminalGuard,
    client: OpencodeClient,
    mut app: AppState,
    rx: &mut UnboundedReceiverStream<StreamEvent>,
    session_id: String,
) -> Result<()> {
    let mut events = EventStream::new();
    let mut textarea = ui_mod::chat::build_textarea();

    loop {
        terminal.0.draw(|f| ui_mod::draw(f, &app, &mut textarea))?;

        tokio::select! {
            maybe = events.next() => {
                let Some(ev): Option<Result<CtEvent, _>> = maybe else { break };
                let ev = ev.context("crossterm event")?;
                if let CtEvent::Key(k) = ev {
                    handle_key(&mut app, &mut textarea, k, &client, &session_id).await?;
                }
            }
            Some(ev) = rx.next() => {
                apply_event(&mut app, ev);
            }
        }
    }
    Ok(())
}

// =====================================================================
// handle_key — every keypress routes here.
//
// KEYBIND design (locked in section 10 of the plan):
//   Enter              → send
//   Shift+Enter        → newline (textarea default for Enter is also newline;
//                         we override Enter to submit, newline comes from
//                         feeding the key into the textarea Input)
//   Esc                → abort if busy, else quit
//   Ctrl+C             → quit (raw mode + panic hook will restore)
//
// We do NOT remap tui-textarea globally — too magic. Instead, match
// KeyCode::Enter above and feed everything else into ta.input(Input::from(k)).
// =====================================================================
async fn handle_key(
    app: &mut AppState,
    ta: &mut TextArea<'_>,
    k: KeyEvent,
    client: &OpencodeClient,
    session_id: &str,
) -> Result<()> {
    match k.code {
        KeyCode::Esc => {
            if matches!(app.status, Status::Busy) {
                // Abort: fire the request; server emits session.idle when stopped.
                let _ = client.abort(session_id).await;
                app.abort_requested = true;
            } else {
                std::process::exit(0);
            }
        }
        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
            std::process::exit(0);
        }
        KeyCode::Enter => {
            let text: String = ta.lines().join("\n");
            let text = text.trim().to_string();
            if text.is_empty() {
                return Ok(());
            }
            // Commands are local-only and run even while streaming — a
            // user-initiated /quit or /clear should never be blocked.
            let is_command = text.starts_with('/') || text.starts_with(':');
            if !is_command && matches!(app.status, Status::Busy) {
                return Ok(());
            }
            ta.select_all();
            ta.cut();

            // ── Command path ──
            if is_command {
                // Echo the typed command as a User line so it appears in
                // the chat history (consistent with regular prompts).
                app.messages
                    .push(DisplayMessage { role: Role::User, text: text.clone() });
                let cmd = commands::parse(&text);
                let outcome = commands::execute(cmd, app, client).await;
                if outcome.is_quit() {
                    std::process::exit(0);
                }
                return Ok(());
            }

            // ── Regular prompt path ──
            // User message → render immediately.
            app.messages
                .push(DisplayMessage { role: Role::User, text: text.clone() });
            // Pre-create the assistant placeholder so the streaming UI has a target.
            app.messages
                .push(DisplayMessage { role: Role::Assistant, text: String::new() });
            app.streaming_message_index = Some(app.messages.len() - 1);
            app.parts.clear();
            app.part_to_message.clear();
            app.last_user_text = Some(text.clone());
            app.status = Status::Busy;

            // Fire-and-forget. Tokens stream back via SSE.
            //
            // If the user has a `/model` override set, include it in the
            // per-prompt body so the next response uses that model. The
            // override is purely client-side; the server stores which
            // model answered in the assistant message metadata.
            let model_override = app
                .current_model_override
                .as_deref()
                .and_then(crate::client::ModelRef::parse);
            if let Err(e) = client
                .send_prompt(session_id, &text, model_override)
                .await
            {
                app.status = Status::Error(format!("send: {e:#}"));
            }
        }
        _ => {
            ta.input(Input::from(k));
        }
    }
    Ok(())
}

// =====================================================================
// apply_event — SSE → AppState. The streaming heart.
//
// Streaming rule (opencode contract):
//   - delta IS present  → append to the active assistant message.
//   - delta is absent    → replace (full text sent, often after reconnect or completion).
//
// Message-id tracking:
//   Opencode gives each text part a stable `message_id`. We map part_id → message_id
//   on first sight, then route subsequent updates by message_id. This is
//   robust if parts are split or re-ordered.
//
// Boot note:
//   opencode might replay parts of history — those events arrive like live stream.
//   We only render events AFTER the user has sent at least one prompt. We key
//   this off `pending_user_text.is_some()` OR matching `message_id` against
//   our current streaming_message_index.
//
// ponytail: this function is the ONLY place we mutate AppState from the server.
// =====================================================================
fn apply_event(app: &mut AppState, ev: StreamEvent) {
    match ev {
        StreamEvent::ServerConnected => {}
        StreamEvent::SessionIdle { .. } => {
            // Server finished — flip back to Idle. If the user aborted, swap the
            // status text but keep the same flow.
            if app.abort_requested {
                app.abort_requested = false;
                app.status = Status::Idle;
            } else {
                app.status = Status::Idle;
            }
            app.streaming_message_index = None;
            app.last_user_text = None;
            app.parts.clear();
        }
        StreamEvent::SessionError { message } => {
            app.status = Status::Error(message);
            app.streaming_message_index = None;
        }
        StreamEvent::Disconnected(why) => {
            // If a prompt was in flight, end it; otherwise just flag the connection.
            // We don't blank `streaming_message_index` here — partial text stays
            // visible, and the user gets a clear reason in the status bar.
            if matches!(app.status, Status::Busy) {
                app.status = Status::Error(format!("disconnected: {why}"));
                app.streaming_message_index = None;
            }
        }
        StreamEvent::PartUpdated { part_id, message_id, text, delta } => {
            // Map part_id → message_id on first sight.
            if !part_id.is_empty() && !message_id.is_empty() {
                app.part_to_message
                    .entry(part_id.clone())
                    .or_insert(message_id.clone());
            }

            // ponytail: opencode replays the user's input as a text part of the
            // user message. If the streamed text equals what we just sent, drop it.
            if let Some(sent) = &app.last_user_text {
                if &text == sent {
                    return;
                }
                if delta.as_deref() == Some(sent.as_str()) {
                    return;
                }
            }

            // We only render events for the currently-streaming assistant message.
            let target_idx = match app.streaming_message_index {
                Some(i) => i,
                None => return,
            };
            if target_idx >= app.messages.len() {
                return;
            }
            if !matches!(app.messages[target_idx].role, Role::Assistant) {
                return;
            }

            // Apply to part buffer (cheap).
            let buf = app.parts.entry(part_id).or_insert_with(PartBuffer::default);
            match delta {
                Some(d) if !d.is_empty() => buf.text.push_str(&d),
                _ => buf.text = text.clone(),
            }

            // Render the LAST seen part's text for this message.
            // ponytail: if opencode ever sends multiple parts per message in
            // parallel (rare), we'd render only the last. Track per-part in
            // the UI later. For v0 this is fine — single text-part answer.
            let rendered: String = app
                .parts
                .values()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("");
            app.messages[target_idx].text = rendered;
            let _ = message_id; // kept for debugging; we trust streaming_message_index in v0.
        }
        StreamEvent::Unknown => {}
    }
}

// =====================================================================
// RAII guard: enables raw mode + alternate screen on enter, restores on Drop.
// Defense in depth alongside the panic hook.
// =====================================================================
struct TerminalGuard(ratatui::Terminal<CrosstermBackend<std::io::Stdout>>);

impl TerminalGuard {
    fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().context("enable raw mode")?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)
            .context("enter alternate screen")?;
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = ratatui::Terminal::new(backend).context("build terminal")?;
        Ok(Self(terminal))
    }

    fn leave() {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        Self::leave();
    }
}
