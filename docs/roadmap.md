# kaku-tui — Roadmap to Claude Code core parity

A 6-week plan from current v0 to "the daily-driver chat client you'd
miss if you switched from Claude Code". Not exhaustive feature
parity (autocomplete, MCP, file picker, lsp) — only what you'd use
every session.

---

## How to read this

Each week = 1 commit per feature. Small, focused, verified. If a
feature costs more than estimated, we cut it before adding new
work — no silent scope-creep. The "what stays skipped" list at the
bottom is a hard line; moving anything off it requires an explicit
decision.

Risks are flagged with [risk]. Watch those.

---

## Week 1 — Polish + token tracking

**Goal:** every session now shows real cost + token usage. Conversation
listing works for `/new` / `/resume`.

| # | Feature | Server | Effort |
|---|---|---|---|
| 1.1 | Token count + cost in status bar | `StepFinishPart` carries both | Small |
| 1.2 | `/sessions` list | `GET /session` (server-side) | Small |
| 1.3 | `/resume <id>` | `app.session.id = id` swap | Small |
| 1.4 | Multi-line input polish (visual overflow, history) | local | Small |
| 1.5 | Visual cleanup pass | local | Small |

**Why first:** every other week benefits. Token count is "did this
just cost me $2?" — daily value. `/resume` is the foundation of the
Week 4 history picker.

**Risk:** `GET /session` may not exist or may return a different
shape. We have a smoke harness we can adapt; if it doesn't pan
out, we fall back to tracking sessions client-side in Week 4.

---

## Week 2 — Markdown in chat

**Goal:** assistant messages render as actual markdown. Code blocks
get readable styling. No more raw `**bold**` text.

| # | Feature | Dep | Effort |
|---|---|---|---|
| 2.1 | `pulldown-cmark` integration | local | Medium |
| 2.2 | Code blocks (fenced ```) with dim style | 2.1 | Small |
| 2.3 | Inline code with bg + monospace | 2.1 | Small |
| 2.4 | Bold / italic / strikethrough | 2.1 | Small |
| 2.5 | Headers, lists, links (basic) | 2.1 | Small |

**Why second:** biggest single UX win for the day-to-day. The gap
between "raw text" and "formatted chat" is huge.

**Risk:** [risk] `pulldown-cmark` events need a non-trivial mapping
to `ratatui::text::Text`. May take 1-2 days of fiddling with styles.
If it goes badly, we can ship only code-block rendering in Week 2
and pick up the rest later.

---

## Week 3 — Multi-session tabs

**Goal:** one opencode session per tab. Switch with `1` `2` `3` or
`Tab` / `Shift+Tab`. New tab = `/new`. Close tab = `Ctrl+W`.

| # | Feature | Effort |
|---|---|---|
| 3.1 | `AppState.tabs: Vec<Tab>` where each tab has its own session id, messages, parts | Medium |
| 3.2 | Tab bar at top: `[● tab1] [tab2] [+]` | Medium |
| 3.3 | Hotkeys: `1-9` jump, `Tab` next, `Shift+Tab` prev, `Ctrl+T` new, `Ctrl+W` close | Small |
| 3.4 | Per-tab state isolation (active streaming, parts, etc.) | Medium |

**Why third:** once you have tabs, the chat feels real. But tabs
are also the first state-machine complexity we hit. After this,
"which session is the message for?" stops being a trivial question.

**Risk:** [risk] the SSE stream is global. A `message.part.updated`
event arrives with a `sessionID` we currently ignore. Multi-tab
means we MUST filter by `sessionID` or events from one tab leak to
another. Filter happens in `apply_event` — straightforward, but
must be done before tabs ship.

---

## Week 4 — Conversation history

**Goal:** `/history` opens a picker of past sessions, with filter.
Resume into a past session from the picker.

| # | Feature | Dep | Effort |
|---|---|---|---|
| 4.1 | `GET /session` (the same call as 1.2, used as list source) | 1.2 | Small |
| 4.2 | `/history` opens a modal selector overlay (kaku config_tui style: bordered popup with field actions in title) | 3.1 | Medium |
| 4.3 | Filter input (search by title) | 4.2 | Small |
| 4.4 | Preview pane showing session summary | 4.2 | Medium |

**Why fourth:** builds on Week 3 (we now have a place to put "inbox"
sessions vs active sessions). Filter UX gets us most of the value
quickly.

**Risk:** [risk] if the server doesn't return enough metadata per
session, the picker is mostly useless. The Session type has `title`,
`time.created`, `summary.{additions,deletions,files}` — should be
enough. If `summary` is rarely populated, we show title + time only.

**Drop if rough:** if tabs + history together are too much state,
we cut history from v0. Most users only need 1-2 active sessions,
not 50 in a picker. Tabs alone are still a big win.

---

## Week 5 — Slash commands beyond 5

**Goal:** bring command count from 5 to ~12. Add command discovery.

| # | Feature | Effort |
|---|---|---|
| 5.1 | `/compact` — server-side context compaction (free, just a flag) | Small |
| 5.2 | `/cost` — show session cost so far (we have the data from Week 1) | Small |
| 5.3 | `/retry` — re-send last user prompt | Small |
| 5.4 | `/undo` — drop last user+assistant pair from view, server history kept | Small |
| 5.5 | `/edit <id>` — multi-turn message editing | Medium |
| 5.6 | Tab-complete on `/` (cycle through commands) | Small |
| 5.7 | `/help` autocomplete subcommands (e.g. `/theme dark`) | Small |

**Why fifth:** the command set is what makes a chat client feel
"completable". After Week 5, daily use feels familiar.

**Risk:** `/edit` is the heaviest item — needs to delete a message
server-side, possibly renumber subsequent messages, and re-stream
the conversation. May bleed into Week 6.

---

## Week 6 — UX deep polish

**Goal:** the rough edges that bug a daily user are gone.

| # | Feature | Effort |
|---|---|---|
| 6.1 | Auto-reconnect on SSE drop (with exponential backoff) | Medium |
| 6.2 | Mouse support: click in textarea, click to copy selection | Medium |
| 6.3 | Theme picker (`/theme dark|light`) | Small |
| 6.4 | Working dir in header band (`~/path/to/cwd` dim) | Small |
| 6.5 | Config file (`~/.config/kaku-tui/config.toml`) for prefs | Medium |
| 6.6 | Inline `@` mention (file path completion) | [risky] skip if not done in Week 5 |

**Why last:** the work is mostly fixing things we already shipped.
Auto-reconnect is the biggest single reliability win; without it,
the TUI dies silently if the opencode server restarts.

**Risk:** [risk] mouse support is fiddly. If it consumes more than
a day, drop it — terminal users usually don't need it.

---

## What stays skipped

| Feature | Why skip |
|---|---|
| Inline AI ghost text | needs terminal-internal interception; high effort, niche value |
| Floating AI panel | needs windowing primitives; kaku-gui is the only place that has them |
| Tool execution (bash, edit, fetch) | need a real agent runtime, not a chat client |
| Multi-provider OAuth | kaku's main pain; users pick at server config |
| MCP integration | opens a whole protocol surface |
| File picker via `@` | Claude Code's works because of LSP; complex to replicate |
| Doctor panel | kaku's signature feature but very Mac-specific |
| Permission UI | the "y/n" prompts; we get there as a follow-up to tools |
| LSP / formatter / todo | kaku has these; opencode's TUI shows them; not chat-relevant |

---

## Risk register

| Risk | Mitigation |
|---|---|
| Markdown rendering fights ratatui | ship code-blocks-only as a fallback; defer the rest |
| Multi-session tabs leak events between tabs | filter SSE events by `sessionID` in `apply_event` BEFORE tabs ship |
| `GET /session` returns less metadata than expected | show what we have; history picker falls back to title + time |
| Auto-reconnect re-drops in a loop | exponential backoff with cap; surface in status bar when retrying |
| `pulldown-cmark` integration becomes its own rabbit hole | scope-cut to 1 week of polish only, no exotic features |
| Week 4 history picker drags into multi-week UI work | drop from v0; document as Week 7+ follow-up |

---

## What "done" means at the end of 6 weeks

- [x] Token count + cost visible during streaming
- [x] Markdown renders in assistant messages (code blocks at minimum)
- [x] Multi-session tabs with hotkey switching
- [x] Conversation history picker
- [x] 12+ slash commands
- [x] Auto-reconnect on SSE drop
- [x] Mouse support OR a documented "won't add" reason
- [x] `/config` file for persistent prefs

A user switching from Claude Code to kaku-tui would not feel a
gap in their daily workflow. The things they'd miss (autocomplete,
MCP, file picker) are explicitly out of scope for v0.
