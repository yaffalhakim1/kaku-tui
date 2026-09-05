// ponytail: command palette for kaku-tui.
//
// User types `/command [args]` or `:command [args]` in the prompt, presses
// Enter — handle_key intercepts it before sending to the LLM. Output
// lands in the chat as a `Role::System` message.
//
// Both `/` and `:` map to the same command set. `/` is the Claude Code
// convention; `:` is the vim/fzf/lazygit convention. Either works.
//
// All commands are local-only: nothing calls the opencode server except
// `/new`, which spins up a fresh session. No state mutation outside
// AppState.

use crate::app::{AppState, DisplayMessage, Role};
use crate::client::OpencodeClient;

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Clear,
    Quit,
    New,
    Sessions,
    /// `/model` with no arg → show current; `/model <provider/id>` → switch.
    Model(Option<String>),
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Keep the TUI alive.
    Continue,
    /// Caller should exit the process (Stdprocess::exit(0)).
    Quit,
}

impl Outcome {
    pub fn is_quit(&self) -> bool {
        matches!(self, Outcome::Quit)
    }
}

/// Parse the trimmed input line. Returns Unknown for unrecognized prefixes,
/// Empty if no command. Both `/` and `:` prefixes are accepted.
///
/// ponytail: we deliberately split ONLY on the first whitespace. Anything
/// after the first word is treated as free-form arg, even if multi-line.
/// For commands that don't care about args this is wasted; for commands
/// that do (none yet) we can pull out args later.
pub fn parse(text: &str) -> Command {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Command::Unknown(String::new()); // signal no-op
    }
    // Strip leading prefix char if present.
    let rest = trimmed
        .strip_prefix('/')
        .or_else(|| trimmed.strip_prefix(':'))
        .unwrap_or(trimmed);
    let mut parts = rest.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").to_lowercase();
    let arg = parts.next().unwrap_or("").trim().to_string();
    match name.as_str() {
        "help" | "?" | "h" => Command::Help,
        "clear" | "cls" | "c" => Command::Clear,
        "quit" | "exit" | "q" => Command::Quit,
        "new" => Command::New,
        "sessions" | "session" => Command::Sessions,
        // ponytail: arg is preserved so /model and /model X both work.
        // Empty string means "no arg" — execute shows current.
        "model" | "m" => {
            if arg.is_empty() {
                Command::Model(None)
            } else {
                Command::Model(Some(arg))
            }
        }
        other => Command::Unknown(other.to_string()),
    }
}

/// Execute the command. Mutates `app` (pushes System messages, changes
/// session, etc.). Performs the necessary HTTP call for `/new`.
///
/// ponytail: this is a free async function, not a method on AppState,
/// because we want it testable + it has a side-effecting call to the
/// opencode client. AppState is the only mutation surface.
pub async fn execute(cmd: Command, app: &mut AppState, client: &OpencodeClient) -> Outcome {
    match cmd {
        Command::Unknown(ref s) if s.is_empty() => Outcome::Continue, // empty input
        Command::Unknown(s) => {
            app.messages.push(DisplayMessage {
                role: Role::System,
                text: format!("unknown command: /{s}\ntry /help"),
            });
            Outcome::Continue
        }
        Command::Help => {
            app.messages.push(DisplayMessage {
                role: Role::System,
                text: HELP_TEXT.to_string(),
            });
            Outcome::Continue
        }
        Command::Clear => {
            // Reset visible chat. Don't touch the server session — the
            // user keeps their conversation, just hides it from screen.
            // Useful for screenshots + when the buffer scrolls past the
            // interesting part.
            app.messages.clear();
            app.parts.clear();
            app.streaming_message_index = None;
            app.last_user_text = None;
            Outcome::Continue
        }
        Command::Quit => Outcome::Quit,
        Command::New => {
            match client.create_session(Some("kaku-tui")).await {
                Ok(s) => {
                    app.session = Some(s.clone());
                    app.messages.clear();
                    app.parts.clear();
                    app.streaming_message_index = None;
                    app.last_user_text = None;
                    app.messages.push(DisplayMessage {
                        role: Role::System,
                        text: format!("fresh session: {} ({})", s.id, s.title),
                    });
                }
                Err(e) => {
                    app.messages.push(DisplayMessage {
                        role: Role::System,
                        text: format!("could not create session: {e:#}"),
                    });
                }
            }
            Outcome::Continue
        }
        Command::Sessions => {
            // We only track the current session. Listing all sessions
            // requires GET /session — lifted to a follow-up if needed.
            let msg = match &app.session {
                Some(s) => format!(
                    "current session:\n  id: {}\n  title: {}\n  created: {}",
                    s.id,
                    s.title,
                    chrono_like_unix(s.time.created)
                ),
                None => "no active session".to_string(),
            };
            app.messages.push(DisplayMessage {
                role: Role::System,
                text: msg,
            });
            Outcome::Continue
        }
        Command::Model(arg) => {
            // Two modes:
            //   /model          → show current override + default
            //   /model p/id     → validate and store as override
            //
            // Validation is intentionally light: opencode will reject
            // a bad provider/model at request time, and the server's
            // session.error event flips the status bar to Error. We
            // only catch the obvious typo (missing slash).
            match arg {
                None => {
                    let current = app
                        .current_model_override
                        .as_deref()
                        .unwrap_or("(none — using server default)");
                    let default = app
                        .default_model
                        .as_deref()
                        .unwrap_or("(none — server config has no default)");
                    app.messages.push(DisplayMessage {
                        role: Role::System,
                        text: format!("model:\n  override: {current}\n  default: {default}"),
                    });
                    Outcome::Continue
                }
                Some(spec) => {
                    if !spec.contains('/') {
                        app.messages.push(DisplayMessage {
                            role: Role::System,
                            text: format!(
                                "bad model spec: {spec}\n  expected: provider/id (e.g. anthropic/claude-opus-4-5)"
                            ),
                        });
                        return Outcome::Continue;
                    }
                    let trimmed = spec.trim().to_string();
                    app.current_model_override = Some(trimmed.clone());
                    app.messages.push(DisplayMessage {
                        role: Role::System,
                        text: format!("model → {trimmed}"),
                    });
                    Outcome::Continue
                }
            }
        }
    }
}

/// Format a unix-ms timestamp as a compact YYYY-MM-DD HH:MM string.
/// ponytail: avoid pulling chrono for one format. The math here is
/// just a UTC transformation; if we need locale display we add chrono
/// later.
fn chrono_like_unix(ms: u64) -> String {
    let secs = ms / 1000;
    let days = secs / 86400;
    let rem = secs % 86400;
    let hh = (rem / 3600) % 24;
    let mm = (rem / 60) % 60;
    let ss = rem % 60;
    // Days since 1970-01-01 — approximate Y-M-D for display only.
    // Ceil division via `+` on integer, kept readable.
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
}

/// Days-since-epoch → (year, month, day) using the standard Gregorian
/// leap-year rules. Inline on purpose: we render this rarely and pulling
/// in `time` or `chrono` for one display string is overkill.
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    let mut remaining = days;
    loop {
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let ylen = if leap { 366 } else { 365 };
        if remaining >= ylen {
            remaining -= ylen;
            y += 1;
        } else {
            break;
        }
    }
    let mdays = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mut mo = 1;
    let mut d = remaining + 1;
    for (i, dm) in mdays.iter().enumerate() {
        let dim = if i == 1 && leap { 29 } else { *dm };
        if d <= dim as u64 {
            mo = i as u64 + 1;
            break;
        }
        d -= dim as u64;
        mo = i as u64 + 2;
    }
    (y, mo, d)
}

const HELP_TEXT: &str = "commands (prefix / or :):

  /help      show this help
  /clear     clear the visible chat (server history kept)
  /quit      exit kaku-tui  (also esc:quit, ctrl+c)
  /new       open a fresh opencode session
  /sessions  show current session info
  /model     show the active model

examples:  /help    :q    /new    /model";
