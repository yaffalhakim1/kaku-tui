# kaku-tui

A [ratatui](https://ratatui.rs) TUI client for [opencode](https://opencode.ai), styled after [kaku](https://kaku.fun): quiet, dark, single accent, generous spacing.

![tui demo](https://placehold.co/640x240/1a1a1a/e0e0e0?text=coming+soon)

## What it does

- Connects to a running `opencode serve` (HTTP + SSE).
- Sends prompts via `POST /session/:id/prompt_async`, streams tokens back via `GET /event`.
- Renders the chat in a kaku-style frame: rounded borders, dim chrome, yellow accent.
- Single keybind set: **Enter** sends, **Shift+Enter** newline, **Esc** aborts (or quits when idle), **Ctrl+C** quits.

## Run

```bash
# 1. Start opencode in another terminal
opencode serve

# 2. (optional) auth — set this if your server has OPENCODE_SERVER_PASSWORD
export OPENCODE_SERVER_PASSWORD=secret

# 3. Build + run
cargo run --release -- "http://127.0.0.1:4096"

# (or with auth)
KAKU_TUI_PASSWORD=$OPENCODE_SERVER_PASSWORD cargo run --release -- "http://127.0.0.1:4096"
```

The server URL and password are CLI arg + env var. No config file yet.

## Smoke test

Verify the SSE pipeline end-to-end without the TUI:

```bash
KAKU_TUI_PASSWORD=$OPENCODE_SERVER_PASSWORD cargo run --example sse_smoke -- 4096
```

You'll see ~30 events streamed in: `server.connected` → `message.part.updated` (with `delta`) → `session.idle`.

## Architecture

Three concurrent sources in one async runtime:

1. **Keyboard events** (crossterm `EventStream`).
2. **SSE events** (a `tokio::spawn`-ed task on `/event`, bridged via `mpsc::unbounded_channel`).
3. **A 500ms blink timer** for the streaming cursor.

`tokio::select!` in `main.rs` races them. State lives in one `AppState` struct; UI is a pure `(&AppState, &mut TextArea) -> Frame` function.

```
run loop: draw + select!{ keys, sse_channel, blink_tick }
                            │
                            ├─ keyboard → handle_key  → mutate AppState
                            ├─ SSE       → apply_event → mutate AppState
                            └─ tick       → toggle cursor_visible
```

## Visual rules

In `src/theme.rs`:

| Token | Color | Use |
|---|---|---|
| `FG` | `Color::Gray` | not pure white; kaku's softer text |
| `FG_DIM` | `Color::DarkGray` | borders, meta text |
| `FG_MUTE` | `Indexed(245)` | hint copy |
| `ACCENT` | `Color::Yellow` | cursor, status highlights |
| `USER` | `Cyan` | user messages |
| `ASSIST` | `White` | assistant messages |

Borders are `BorderType::Rounded`, padding 1 cell, never more. No animations beyond the 500ms cursor blink.

## Limitations (v0 scope)

- One session, in-process, created on launch.
- Plain text only — no markdown, no syntax highlighting.
- SSE drop → status bar flips to `Error`; no auto-reconnect.
- Light mode not supported (dark only).

Each has a "trigger" in the design doc — see commits/discussions for when to add them.

## License

MIT.
