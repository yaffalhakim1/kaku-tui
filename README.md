# kaku-tui

A [ratatui](https://ratatui.rs) client for [opencode](https://opencode.ai), styled after [kaku](https://kaku.fun).

## Run

```bash
opencode serve &
cargo run --release -- "http://127.0.0.1:4096"
```

If your server runs behind a password:

```bash
OPENCODE_SERVER_PASSWORD=secret opencode serve &
KAKU_TUI_PASSWORD=$OPENCODE_SERVER_PASSWORD cargo run --release -- "http://127.0.0.1:4096"
```

Type, press Enter, watch the response stream in. Esc aborts. Esc on idle quits. Ctrl+C quits.

## Try the SSE pipeline without the TUI

```bash
KAKU_TUI_PASSWORD=$OPENCODE_SERVER_PASSWORD cargo run --example sse_smoke -- 4096
```

Logs every event opencode emits: `server.connected`, then `message.part.updated` per token, then `session.idle`.

## How it hangs together

Three sources in `tokio::select!`:

- keyboard events from crossterm
- SSE frames from a `tokio::spawn`-ed task on `GET /event`, pushed through an `mpsc::unbounded_channel`
- a 500 ms tick

State lives in one `AppState` struct. The UI takes `&AppState` and renders. Server events and keypresses mutate `AppState` and re-render.

## Colors

| Token | Value | Where |
|---|---|---|
| `FG` | `Gray` | body text |
| `FG_DIM` | `DarkGray` | borders |
| `FG_MUTE` | `Indexed(245)` | hint copy |
| `ACCENT` | `Yellow` | cursor, status highlights |
| `USER` | `Cyan` | user messages |
| `ASSIST` | `White` | assistant messages |

Rounded borders. One cell of padding. No animations.

## What it doesn't do

One session per launch. Plain text only. Dark theme only. SSE drop shows in the status bar; no auto-reconnect.

## License

MIT.
