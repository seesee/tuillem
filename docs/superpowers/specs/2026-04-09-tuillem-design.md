# tuillem — TUI LLM Frontend Design Spec

## Overview

tuillem is a terminal-based chat interface for interacting with multiple LLM providers. It provides a two-pane layout (session sidebar + conversation view), persistent storage via SQLite, YAML-driven configuration for providers and tools, and an external-process plugin system for extensibility.

Built in Rust with Ratatui. Architected for eventual open-source release, but built first as a personal power tool.

## Architecture

**Layered architecture with actor-like components.** Each major subsystem runs as an independent async task communicating via typed `tokio::mpsc` channels. A central coordinator in `tuillem-core` routes actions between layers.

```
┌─────────────┐     actions      ┌──────────────┐
│  tuillem-tui │ ───────────────> │              │
│  (UI layer)  │ <─────────────── │ tuillem-core │
│              │   state updates  │ (coordinator)│
└─────────────┘                  │              │
                                 └──────┬───────┘
                        ┌───────────────┼───────────────┐
                        v               v               v
                ┌──────────────┐ ┌────────────┐ ┌──────────────┐
                │tuillem-provider│ │ tuillem-db │ │tuillem-plugin│
                │ (LLM APIs)   │ │ (SQLite)   │ │ (tool host)  │
                └──────────────┘ └────────────┘ └──────────────┘
```

**Why this approach:** Clean separation for testing, natural streaming support via channels, plugin isolation (each plugin is a separate process), and easy to swap implementations.

## Crate Structure

```
tuillem/
├── Cargo.toml              # workspace root
├── crates/
│   ├── tuillem-core/       # App state, actions, coordinator
│   ├── tuillem-tui/        # Ratatui UI layer
│   ├── tuillem-provider/   # LLM provider abstraction + impls
│   ├── tuillem-db/         # SQLite storage layer
│   ├── tuillem-config/     # YAML config parsing & validation
│   ├── tuillem-plugin/     # Plugin host for external tools
│   └── tuillem-markdown/   # Terminal markdown rendering
├── src/
│   └── main.rs             # Binary — wires crates together
├── config.example.yaml
└── migrations/             # SQLite schema migrations
```

Each crate has a single responsibility, compiles independently, and is testable in isolation.

## UI Layer (tuillem-tui)

### Layout

```
┌──────────────┬─────────────────────────────────┐
│              │  Model: claude-sonnet-4-20250514          │
│  Sessions    │─────────────────────────────────│
│              │                    How do I... ← │  (user, right-aligned)
│  [search]    │                                 │
│              │  Here's how...                  │  (assistant, left-aligned)
│  > Topic 1   │  ▶ Thinking...  [collapsed]     │
│    Topic 2   │                                 │
│    Topic 3   │  The answer is...               │
│    Topic 4   │                                 │
│              │                                 │
│  ──────────  │                                 │
│  Tags:       │                                 │
│   #research  │─────────────────────────────────│
│   #code      │ > Type a message...    [model ▾]│
│              │                                 │
└──────────────┴─────────────────────────────────┘
```

### Message Display

- **User messages** — right-aligned with distinct background colour
- **Assistant messages** — left-aligned with different background
- **Thinking blocks** — collapsed by default. Show a throbber/spinner while actively streaming. Expand/collapse via keybind (default: `t`) or mouse click. Rendered in dimmed/muted style when expanded
- **Tool calls** — shown inline with expandable results
- **Model indicator** — each message shows which model produced it; model switches are visually marked

### Sidebar

- Flat chronological list of sessions
- Search box at top with prefix filters: `tag:research`, `model:claude`
- Sessions display: title, timestamp, tags
- Tags section at bottom for quick filtering

### Input

- Multi-line text input at bottom of conversation pane
- Enter to submit (configurable), Shift+Enter for newline
- Model selector accessible via keybind
- **External editor support** — keybind (default: `Ctrl+E`) opens `$EDITOR` (overridable in config) with a temp file. On editor exit, file content becomes the prompt. Follows the `git commit` pattern.

### Navigation

- **Keyboard-first design**
- Tab cycles focus: sidebar → conversation → input
- Vim bindings: `j/k` scroll, `gg/G` top/bottom, `Ctrl+d/u` half-page, `/` search
- Cursor keys, Page Up/Down, Home/End all work
- Configurable keybinding preset: `vim | emacs | default`
- **Mouse support** — scroll wheel, click to select sessions, expand/collapse thinking blocks. Enabled by default, togglable in config

## Provider Layer (tuillem-provider)

### Provider Trait

```rust
trait Provider: Send + Sync {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream>;
    fn models(&self) -> Vec<ModelInfo>;
    fn supports_streaming(&self) -> bool;
    fn supports_thinking(&self) -> bool;
}
```

`ChatResponseStream` is an async stream yielding typed deltas (text chunks, thinking chunks, tool call chunks) for progressive UI rendering.

### Provider Types

- `anthropic` — Anthropic Messages API. Native support for thinking blocks, tool use
- `openai` — OpenAI-compatible API. Covers OpenAI, LMStudio, and any compatible endpoint
- `openrouter` — OpenRouter API. OpenAI-compatible with model discovery
- `ollama` — Ollama native API. Model management (pull, list), native chat endpoint

### Model Switching

When switching models mid-conversation:

1. UI presents a selection dialog: full history, last N messages, or summary
2. Coordinator reformats message history for the target provider's API format
3. Switch point recorded in DB with context strategy used
4. New model indicator shown in conversation view

## Storage Layer (tuillem-db)

### Schema

```sql
-- Sessions
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,          -- UUID
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,     -- ISO 8601
    updated_at TEXT NOT NULL,
    metadata TEXT                 -- JSON
);

-- Tags
CREATE TABLE session_tags (
    session_id TEXT NOT NULL REFERENCES sessions(id),
    tag TEXT NOT NULL,
    PRIMARY KEY (session_id, tag)
);

-- Messages
CREATE TABLE messages (
    id TEXT PRIMARY KEY,          -- UUID
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,           -- user | assistant | system | tool
    content TEXT,                 -- visible message text
    model_id TEXT,
    provider_name TEXT,
    created_at TEXT NOT NULL,
    token_usage_in INTEGER,
    token_usage_out INTEGER,
    latency_ms INTEGER,
    parent_message_id TEXT REFERENCES messages(id)
);

-- Message blocks (thinking, tool calls, etc.)
CREATE TABLE message_blocks (
    id TEXT PRIMARY KEY,          -- UUID
    message_id TEXT NOT NULL REFERENCES messages(id),
    block_type TEXT NOT NULL,     -- text | thinking | tool_call | tool_result
    content TEXT,
    sequence INTEGER NOT NULL,
    compressed INTEGER NOT NULL DEFAULT 0
);

-- Model switches
CREATE TABLE model_switches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    from_model TEXT NOT NULL,
    to_model TEXT NOT NULL,
    at_message_id TEXT NOT NULL REFERENCES messages(id),
    context_strategy TEXT NOT NULL,  -- full | truncated | summary
    switched_at TEXT NOT NULL
);

-- Full-text search
CREATE VIRTUAL TABLE messages_fts USING fts5(content, content=messages, content_rowid=rowid);
CREATE VIRTUAL TABLE blocks_fts USING fts5(content, content=message_blocks, content_rowid=rowid);

-- FTS sync triggers (insert/update/delete on messages and message_blocks
-- keep the FTS indexes up to date automatically)
```

Note: FTS5 content-sync triggers must be created in the migration that establishes these tables. Each insert, update, and delete on the source tables needs a corresponding trigger to maintain the FTS index.
```

### Key Features

- **Full-fidelity storage** — every message, thinking block, tool call, token count, latency
- **Selective compression** — `tuillem prune --thinking --older-than 30d` NULLs content on matching blocks, sets `compressed = 1`
- **FTS5 search** — powers sidebar search across messages and blocks
- **Conversation branching** — `parent_message_id` enables edit-and-regenerate
- **Migrations** — version table + numbered SQL files in `migrations/`

## Plugin & Tool System (tuillem-plugin)

### Configuration

```yaml
tools:
  - name: web-search
    description: "Search the web and return results"
    command: "python3 scripts/web-search.py"
    input_schema:
      type: object
      properties:
        query: { type: string }
    timeout: 30s
    confirm: false

  - name: shell
    description: "Run a shell command and return output"
    command: "scripts/shell-tool.sh"
    input_schema:
      type: object
      properties:
        command: { type: string }
    confirm: true
```

### Protocol

1. Plugin host spawns the configured command as a child process
2. Writes JSON to stdin: `{"name": "tool-name", "input": {"key": "value"}}`
3. Reads JSON from stdout: `{"output": "result text", "error": null}`
4. Process exits after each invocation (stateless by default)

### Tool Invocation Flow

1. Model produces a tool call in its response
2. Coordinator matches tool name against configured tools
3. If `confirm: true`, user is prompted for approval
4. Plugin host spawns process, passes input, reads output (with timeout)
5. Result fed back to model as tool result message
6. Full tool call + result stored in `message_blocks`

### Safety

- `confirm: true` for dangerous tools (user must approve each invocation)
- Configurable timeout (kills hung processes)
- Future: MCP bridge as another tool type

## Markdown Rendering (tuillem-markdown)

### Approach

`pulldown-cmark` parses markdown into events, rendered to styled Ratatui `Text`/`Spans`.

### Supported Elements

- **Code blocks** — syntax highlighted via `syntect`, language label, visible border, distinct background
- **Inline code** — distinct background/colour
- **Bold, italic, strikethrough** — terminal attributes
- **Headings** — bold + colour by level
- **Lists** — bullet and numbered with indentation
- **Links** — rendered as `text (url)` with link colour
- **Blockquotes** — indented with left border character
- **Tables** — column-width measured, rendered with box-drawing characters

### Special Rendering

- **Thinking blocks** — dimmed/muted style, collapsible wrapper with throbber when streaming
- **User messages** — right-aligned within the conversation pane
- **Assistant messages** — left-aligned

## Configuration (tuillem-config)

Default location: `~/.config/tuillem/config.yaml` (XDG-compliant)

Database default: `~/.local/share/tuillem/tuillem.db`

```yaml
# General
editor: vim
keybindings: vim               # vim | emacs | default

# Theme
theme: dark                    # dark | light | <custom name>
themes:
  custom:
    bg: "#1e1e2e"
    fg: "#cdd6f4"
    sidebar_bg: "#181825"
    sidebar_fg: "#cdd6f4"
    sidebar_selected: "#89b4fa"
    user_msg_bg: "#313244"
    assistant_msg_bg: "#1e1e2e"
    thinking_fg: "#6c7086"
    accent: "#89b4fa"
    error: "#f38ba8"
    success: "#a6e3a1"
    warning: "#f9e2af"
    border: "#45475a"
    code_bg: "#11111b"
    code_fg: "#cdd6f4"
    heading: "#89b4fa"
    link: "#74c7ec"
    tag: "#f5c2e7"

# Providers
providers:
  - name: anthropic
    type: anthropic
    api_key: sk-ant-...
    default_model: claude-sonnet-4-20250514
    models:
      - claude-sonnet-4-20250514
      - claude-opus-4-0520

  - name: ollama
    type: ollama
    base_url: http://localhost:11434

# Defaults
defaults:
  provider: anthropic
  model: claude-sonnet-4-20250514
  system_prompt: "You are a helpful assistant."

# Tools
tools:
  - name: web-search
    description: "Search the web"
    command: "python3 ~/.config/tuillem/tools/web-search.py"
    input_schema:
      type: object
      properties:
        query: { type: string }
    timeout: 30s
    confirm: false

# Database
database:
  path: ~/.local/share/tuillem/tuillem.db

# UI
ui:
  sidebar_width: 30
  show_thinking: false
  show_token_usage: true
  mouse: true
```

### Theme System

- **Built-in themes** — `dark` and `light` compiled into the binary
- **Custom themes** — defined under `themes:` key in config
- Partial overrides — unspecified colours fall back to the base theme (`dark`)
- Theme struct maps directly to Ratatui `Style` objects

### Config Validation

- Typed structs via `serde` with `#[serde(default)]` for optional fields
- Clear error messages for malformed YAML, missing required fields, invalid provider types
- Validated at startup before any subsystem initializes

## Testing Strategy

- **tuillem-db** — integration tests against real in-memory SQLite. CRUD, FTS5, migrations, pruning
- **tuillem-provider** — unit tests with mock HTTP (`wiremock`). Streaming parsing, error handling, format conversion
- **tuillem-config** — unit tests for parsing, validation, defaults, error messages
- **tuillem-plugin** — integration tests spawning real test scripts. Timeout, confirm, error handling
- **tuillem-markdown** — unit tests: markdown input → expected Ratatui `Text` output. Code blocks, lists, tables, thinking blocks
- **tuillem-core** — integration tests: mock providers + real DB, action→state transitions, model switching, tool call flow
- **tuillem-tui** — snapshot tests via Ratatui `TestBackend`. Render known state, assert output

No mocking the database — always use real SQLite (in-memory for speed).

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | TUI framework + terminal backend |
| `tokio` | Async runtime |
| `rusqlite` | SQLite with FTS5 |
| `reqwest` | HTTP client for LLM APIs |
| `serde` + `serde_yaml` | Config parsing |
| `pulldown-cmark` | Markdown parsing |
| `syntect` | Syntax highlighting |
| `uuid` | ID generation |
| `chrono` | Timestamps |
| `wiremock` | HTTP mocking for tests |
| `tokio-stream` | Async streaming |
| `futures` | Stream utilities |

## Out of Scope (for now)

- Image rendering (sixel/kitty protocol)
- Conversation sharing/export
- Remote sync between devices
- Voice input
- Built-in MCP server support (future plugin)
