# tuillem Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a terminal-based LLM chat frontend with multi-provider support, SQLite persistence, and an external-process plugin system.

**Architecture:** Layered async architecture. Independent crates communicate via typed tokio::mpsc channels. A coordinator routes actions between the UI, provider, storage, and plugin layers. Each crate is independently testable.

**Tech Stack:** Rust, ratatui 0.30, tokio, rusqlite (bundled-full for FTS5), pulldown-cmark, syntect, serde_yaml, reqwest, crossterm

---

## File Structure

```
tuillem/
├── Cargo.toml                          # workspace root
├── CLAUDE.md                           # project conventions
├── config.example.yaml                 # example configuration
├── src/
│   └── main.rs                         # binary entry point
├── migrations/
│   └── 001_initial.sql                 # initial schema
├── crates/
│   ├── tuillem-config/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # config types + YAML parsing
│   ├── tuillem-db/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # DB handle, migrations
│   │       ├── sessions.rs             # session CRUD
│   │       ├── messages.rs             # message + block CRUD
│   │       └── search.rs              # FTS5 search
│   ├── tuillem-markdown/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # public API
│   │       ├── parser.rs              # pulldown-cmark → intermediate repr
│   │       ├── renderer.rs            # intermediate → ratatui Text
│   │       └── highlight.rs           # syntect code highlighting
│   ├── tuillem-provider/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Provider trait, types
│   │       ├── anthropic.rs           # Anthropic Messages API
│   │       ├── openai.rs              # OpenAI-compatible API
│   │       └── ollama.rs              # Ollama native API
│   ├── tuillem-plugin/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # plugin host, spawn, protocol
│   ├── tuillem-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # public API
│   │       ├── actions.rs             # action/event enums
│   │       ├── state.rs               # app state struct
│   │       └── coordinator.rs         # message routing
│   └── tuillem-tui/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # public API, run loop
│           ├── app.rs                  # top-level layout
│           ├── sidebar.rs             # session list + search
│           ├── conversation.rs        # message display
│           ├── input.rs               # text input + editor
│           └── theme.rs               # theme types + built-in themes
```

---

## Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `CLAUDE.md`
- Create: `src/main.rs`
- Create: all crate `Cargo.toml` files
- Create: all crate `src/lib.rs` stubs

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/tuillem-config",
    "crates/tuillem-db",
    "crates/tuillem-markdown",
    "crates/tuillem-provider",
    "crates/tuillem-plugin",
    "crates/tuillem-core",
    "crates/tuillem-tui",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/yourusername/tuillem"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"

[package]
name = "tuillem"
version.workspace = true
edition.workspace = true

[dependencies]
tuillem-config = { path = "crates/tuillem-config" }
tuillem-db = { path = "crates/tuillem-db" }
tuillem-markdown = { path = "crates/tuillem-markdown" }
tuillem-provider = { path = "crates/tuillem-provider" }
tuillem-plugin = { path = "crates/tuillem-plugin" }
tuillem-core = { path = "crates/tuillem-core" }
tuillem-tui = { path = "crates/tuillem-tui" }
tokio = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
```

- [ ] **Step 2: Create crate Cargo.toml files**

`crates/tuillem-config/Cargo.toml`:
```toml
[package]
name = "tuillem-config"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_yaml = { workspace = true }
thiserror = { workspace = true }
directories = "6"
```

`crates/tuillem-db/Cargo.toml`:
```toml
[package]
name = "tuillem-db"
version.workspace = true
edition.workspace = true

[dependencies]
rusqlite = { version = "0.39", features = ["bundled-full"] }
uuid = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = "3"
```

`crates/tuillem-markdown/Cargo.toml`:
```toml
[package]
name = "tuillem-markdown"
version.workspace = true
edition.workspace = true

[dependencies]
pulldown-cmark = "0.13"
syntect = { version = "5", default-features = false, features = ["default-syntaxes", "default-themes", "parsing", "html"] }
ratatui = "0.30"
```

`crates/tuillem-provider/Cargo.toml`:
```toml
[package]
name = "tuillem-provider"
version.workspace = true
edition.workspace = true

[dependencies]
tuillem-config = { path = "../tuillem-config" }
tokio = { workspace = true }
tokio-stream = "0.1"
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
async-trait = "0.1"
futures = "0.3"
pin-project-lite = "0.2"

[dev-dependencies]
wiremock = "0.6"
tokio = { workspace = true, features = ["test-util"] }
```

`crates/tuillem-plugin/Cargo.toml`:
```toml
[package]
name = "tuillem-plugin"
version.workspace = true
edition.workspace = true

[dependencies]
tuillem-config = { path = "../tuillem-config" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

`crates/tuillem-core/Cargo.toml`:
```toml
[package]
name = "tuillem-core"
version.workspace = true
edition.workspace = true

[dependencies]
tuillem-config = { path = "../tuillem-config" }
tuillem-db = { path = "../tuillem-db" }
tuillem-provider = { path = "../tuillem-provider" }
tuillem-plugin = { path = "../tuillem-plugin" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

`crates/tuillem-tui/Cargo.toml`:
```toml
[package]
name = "tuillem-tui"
version.workspace = true
edition.workspace = true

[dependencies]
tuillem-config = { path = "../tuillem-config" }
tuillem-core = { path = "../tuillem-core" }
tuillem-markdown = { path = "../tuillem-markdown" }
ratatui = "0.30"
crossterm = "0.29"
tokio = { workspace = true }
unicode-width = "0.2"

[dev-dependencies]
insta = "1"
```

- [ ] **Step 3: Create stub lib.rs for each crate**

Each crate gets a minimal `src/lib.rs`:

`crates/tuillem-config/src/lib.rs`:
```rust
//! Configuration parsing and validation for tuillem.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

`crates/tuillem-db/src/lib.rs`:
```rust
//! SQLite storage layer for tuillem.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

`crates/tuillem-markdown/src/lib.rs`:
```rust
//! Terminal markdown rendering for tuillem.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

`crates/tuillem-provider/src/lib.rs`:
```rust
//! LLM provider abstraction for tuillem.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

`crates/tuillem-plugin/src/lib.rs`:
```rust
//! External process plugin host for tuillem.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

`crates/tuillem-core/src/lib.rs`:
```rust
//! Core coordinator and app state for tuillem.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

`crates/tuillem-tui/src/lib.rs`:
```rust
//! Ratatui TUI layer for tuillem.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

- [ ] **Step 4: Create binary entry point**

`src/main.rs`:
```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    println!("tuillem v{}", tuillem_config::version());
    Ok(())
}
```

- [ ] **Step 5: Create CLAUDE.md**

```markdown
# tuillem

TUI frontend for LLMs, built in Rust with ratatui.

## Architecture

Layered async architecture using tokio::mpsc channels between independent crates:
- `tuillem-config` — YAML config parsing (serde_yaml)
- `tuillem-db` — SQLite storage with FTS5 (rusqlite bundled-full)
- `tuillem-markdown` — Terminal markdown rendering (pulldown-cmark + syntect)
- `tuillem-provider` — LLM provider abstraction (reqwest, async-trait)
- `tuillem-plugin` — External process plugin host (tokio::process)
- `tuillem-core` — Coordinator, app state, action routing
- `tuillem-tui` — Ratatui UI layer (crossterm backend)

## Conventions

- Rust edition 2024
- Error handling: `thiserror` for library crates, `anyhow` in binary
- Async runtime: tokio (full features)
- Tests: real SQLite (in-memory), wiremock for HTTP, insta for TUI snapshots
- No mocking the database
- TDD: write failing test first, then implement
- Commit after each meaningful unit of work

## Commands

- `cargo build` — build all crates
- `cargo test --workspace` — run all tests
- `cargo test -p tuillem-db` — run tests for a specific crate
- `cargo clippy --workspace` — lint
- `cargo fmt --all` — format

## Config

- Config file: `~/.config/tuillem/config.yaml`
- Database: `~/.local/share/tuillem/tuillem.db`
- XDG-compliant paths via `directories` crate

## Design

See `docs/superpowers/specs/2026-04-09-tuillem-design.md` for the full design spec.
```

- [ ] **Step 6: Verify workspace compiles**

Run: `cargo build --workspace`
Expected: successful build with no errors

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: scaffold workspace with all crate stubs"
```

---

## Task 2: Configuration (tuillem-config)

**Files:**
- Create: `crates/tuillem-config/src/lib.rs` (replace stub)
- Create: `config.example.yaml`
- Test: `crates/tuillem-config/src/lib.rs` (inline tests)

- [ ] **Step 1: Write tests for config parsing**

Add to `crates/tuillem-config/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_editor")]
    pub editor: String,
    #[serde(default = "default_keybindings")]
    pub keybindings: KeybindingPreset,
    #[serde(default = "default_theme_name")]
    pub theme: String,
    #[serde(default)]
    pub themes: HashMap<String, ThemeColors>,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KeybindingPreset {
    Vim,
    Emacs,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub fg: Option<String>,
    #[serde(default)]
    pub sidebar_bg: Option<String>,
    #[serde(default)]
    pub sidebar_fg: Option<String>,
    #[serde(default)]
    pub sidebar_selected: Option<String>,
    #[serde(default)]
    pub user_msg_bg: Option<String>,
    #[serde(default)]
    pub assistant_msg_bg: Option<String>,
    #[serde(default)]
    pub thinking_fg: Option<String>,
    #[serde(default)]
    pub accent: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub success: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub border: Option<String>,
    #[serde(default)]
    pub code_bg: Option<String>,
    #[serde(default)]
    pub code_fg: Option<String>,
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    Openai,
    Openrouter,
    Ollama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default = "default_timeout")]
    pub timeout: String,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,
    #[serde(default)]
    pub show_thinking: bool,
    #[serde(default = "default_true")]
    pub show_token_usage: bool,
    #[serde(default = "default_true")]
    pub mouse: bool,
}

fn default_editor() -> String {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string())
}

fn default_keybindings() -> KeybindingPreset {
    KeybindingPreset::Default
}

fn default_theme_name() -> String {
    "dark".to_string()
}

fn default_timeout() -> String {
    "30s".to_string()
}

fn default_db_path() -> String {
    directories::ProjectDirs::from("", "", "tuillem")
        .map(|d| d.data_dir().join("tuillem.db").to_string_lossy().to_string())
        .unwrap_or_else(|| "tuillem.db".to_string())
}

fn default_sidebar_width() -> u16 {
    30
}

fn default_true() -> bool {
    true
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            system_prompt: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_width: default_sidebar_width(),
            show_thinking: false,
            show_token_usage: true,
            mouse: true,
        }
    }
}

impl Config {
    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        let config: Config = serde_yaml::from_str(yaml)?;
        config.validate()?;
        Ok(config)
    }

    pub fn from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("", "", "tuillem")
            .map(|d| d.config_dir().join("config.yaml"))
            .unwrap_or_else(|| PathBuf::from("config.yaml"))
    }

    fn validate(&self) -> Result<(), ConfigError> {
        for provider in &self.providers {
            match provider.provider_type {
                ProviderType::Anthropic | ProviderType::Openai | ProviderType::Openrouter => {
                    if provider.api_key.is_none() {
                        return Err(ConfigError::Validation(format!(
                            "provider '{}' requires an api_key",
                            provider.name
                        )));
                    }
                }
                ProviderType::Ollama => {}
            }
        }
        if let Some(ref default_provider) = self.defaults.provider {
            if !self.providers.iter().any(|p| &p.name == default_provider) {
                return Err(ConfigError::Validation(format!(
                    "default provider '{}' not found in providers list",
                    default_provider
                )));
            }
        }
        Ok(())
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config() {
        let yaml = "{}";
        let config = Config::from_yaml(yaml).unwrap();
        assert_eq!(config.keybindings, KeybindingPreset::Default);
        assert_eq!(config.theme, "dark");
        assert!(config.providers.is_empty());
        assert!(config.ui.mouse);
        assert_eq!(config.ui.sidebar_width, 30);
    }

    #[test]
    fn test_full_config() {
        let yaml = r#"
editor: nvim
keybindings: vim
theme: custom
themes:
  custom:
    bg: "#1e1e2e"
    fg: "#cdd6f4"
    accent: "#89b4fa"
providers:
  - name: anthropic
    type: anthropic
    api_key: sk-ant-test
    default_model: claude-sonnet-4-20250514
    models:
      - claude-sonnet-4-20250514
      - claude-opus-4-0520
  - name: local
    type: ollama
    base_url: http://localhost:11434
defaults:
  provider: anthropic
  model: claude-sonnet-4-20250514
  system_prompt: "You are helpful."
tools:
  - name: web-search
    description: "Search the web"
    command: "python3 search.py"
    timeout: 10s
    confirm: false
ui:
  sidebar_width: 35
  show_thinking: true
  mouse: false
"#;
        let config = Config::from_yaml(yaml).unwrap();
        assert_eq!(config.editor, "nvim");
        assert_eq!(config.keybindings, KeybindingPreset::Vim);
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.providers[0].provider_type, ProviderType::Anthropic);
        assert_eq!(config.providers[1].provider_type, ProviderType::Ollama);
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.ui.sidebar_width, 35);
        assert!(config.ui.show_thinking);
        assert!(!config.ui.mouse);
        let theme = config.themes.get("custom").unwrap();
        assert_eq!(theme.bg.as_deref(), Some("#1e1e2e"));
    }

    #[test]
    fn test_validation_missing_api_key() {
        let yaml = r#"
providers:
  - name: anthropic
    type: anthropic
"#;
        let err = Config::from_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("requires an api_key"));
    }

    #[test]
    fn test_validation_invalid_default_provider() {
        let yaml = r#"
providers:
  - name: anthropic
    type: anthropic
    api_key: sk-test
defaults:
  provider: nonexistent
"#;
        let err = Config::from_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("not found in providers list"));
    }

    #[test]
    fn test_ollama_no_api_key_required() {
        let yaml = r#"
providers:
  - name: local
    type: ollama
    base_url: http://localhost:11434
"#;
        let config = Config::from_yaml(yaml).unwrap();
        assert_eq!(config.providers[0].provider_type, ProviderType::Ollama);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p tuillem-config`
Expected: all 5 tests pass

- [ ] **Step 3: Create config.example.yaml**

```yaml
# tuillem configuration
# Default location: ~/.config/tuillem/config.yaml

# Editor for composing long prompts (Ctrl+E)
# Falls back to $VISUAL, then $EDITOR, then vi
editor: vim

# Keybinding preset: vim | emacs | default
keybindings: vim

# Theme: dark | light | <custom name>
theme: dark

# Custom themes (override any subset of colours)
# themes:
#   catppuccin:
#     bg: "#1e1e2e"
#     fg: "#cdd6f4"
#     sidebar_bg: "#181825"
#     sidebar_fg: "#cdd6f4"
#     sidebar_selected: "#89b4fa"
#     user_msg_bg: "#313244"
#     assistant_msg_bg: "#1e1e2e"
#     thinking_fg: "#6c7086"
#     accent: "#89b4fa"
#     error: "#f38ba8"
#     success: "#a6e3a1"
#     warning: "#f9e2af"
#     border: "#45475a"
#     code_bg: "#11111b"
#     code_fg: "#cdd6f4"
#     heading: "#89b4fa"
#     link: "#74c7ec"
#     tag: "#f5c2e7"

# LLM Providers
providers:
  - name: anthropic
    type: anthropic
    api_key: sk-ant-your-key-here
    default_model: claude-sonnet-4-20250514
    models:
      - claude-sonnet-4-20250514
      - claude-opus-4-0520

  # - name: openai
  #   type: openai
  #   api_key: sk-your-key-here
  #   default_model: gpt-4o
  #   models:
  #     - gpt-4o
  #     - gpt-4o-mini

  # - name: openrouter
  #   type: openrouter
  #   api_key: sk-or-your-key-here

  # - name: ollama
  #   type: ollama
  #   base_url: http://localhost:11434

  # - name: lmstudio
  #   type: openai
  #   base_url: http://localhost:1234/v1
  #   api_key: lm-studio

# Defaults for new conversations
defaults:
  provider: anthropic
  model: claude-sonnet-4-20250514
  system_prompt: "You are a helpful assistant."

# External tools (invoked by models)
tools: []
  # - name: web-search
  #   description: "Search the web and return results"
  #   command: "python3 ~/.config/tuillem/tools/web-search.py"
  #   input_schema:
  #     type: object
  #     properties:
  #       query: { type: string }
  #   timeout: 30s
  #   confirm: false

# Database location
database:
  path: ~/.local/share/tuillem/tuillem.db

# UI settings
ui:
  sidebar_width: 30
  show_thinking: false
  show_token_usage: true
  mouse: true
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(config): add YAML config parsing with validation and defaults"
```

---

## Task 3: Database Layer (tuillem-db)

**Files:**
- Create: `migrations/001_initial.sql`
- Create: `crates/tuillem-db/src/lib.rs` (replace stub)
- Create: `crates/tuillem-db/src/sessions.rs`
- Create: `crates/tuillem-db/src/messages.rs`
- Create: `crates/tuillem-db/src/search.rs`

- [ ] **Step 1: Create initial migration**

`migrations/001_initial.sql`:
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata TEXT
);

CREATE TABLE session_tags (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (session_id, tag)
);

CREATE INDEX idx_session_tags_tag ON session_tags(tag);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT,
    model_id TEXT,
    provider_name TEXT,
    created_at TEXT NOT NULL,
    token_usage_in INTEGER,
    token_usage_out INTEGER,
    latency_ms INTEGER,
    parent_message_id TEXT REFERENCES messages(id)
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);

CREATE TABLE message_blocks (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    block_type TEXT NOT NULL CHECK(block_type IN ('text', 'thinking', 'tool_call', 'tool_result')),
    content TEXT,
    sequence INTEGER NOT NULL,
    compressed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_blocks_message ON message_blocks(message_id, sequence);

CREATE TABLE model_switches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    from_model TEXT NOT NULL,
    to_model TEXT NOT NULL,
    at_message_id TEXT NOT NULL REFERENCES messages(id),
    context_strategy TEXT NOT NULL CHECK(context_strategy IN ('full', 'truncated', 'summary')),
    switched_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE messages_fts USING fts5(content, content=messages, content_rowid=rowid);
CREATE VIRTUAL TABLE blocks_fts USING fts5(content, content=message_blocks, content_rowid=rowid);

-- FTS sync triggers for messages
CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;
CREATE TRIGGER messages_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- FTS sync triggers for message_blocks
CREATE TRIGGER blocks_ai AFTER INSERT ON message_blocks BEGIN
    INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER blocks_ad AFTER DELETE ON message_blocks BEGIN
    INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;
CREATE TRIGGER blocks_au AFTER UPDATE ON message_blocks BEGIN
    INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- Schema version tracking
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'));
```

- [ ] **Step 2: Write the DB handle and migration runner**

`crates/tuillem-db/src/lib.rs`:
```rust
//! SQLite storage layer for tuillem.

pub mod messages;
pub mod search;
pub mod sessions;

use rusqlite::Connection;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &str) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn migrate(&self) -> Result<(), DbError> {
        let current_version = self.current_schema_version();
        if current_version < 1 {
            let sql = include_str!("../../../migrations/001_initial.sql");
            self.conn.execute_batch(sql)?;
        }
        Ok(())
    }

    fn current_schema_version(&self) -> i64 {
        self.conn
            .query_row(
                "SELECT MAX(version) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Db::open_in_memory().unwrap();
        let version: i64 = db
            .conn()
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_migrate_idempotent() {
        let db = Db::open_in_memory().unwrap();
        // Running migrate again should not error
        db.migrate().unwrap();
    }
}
```

- [ ] **Step 3: Run tests to verify migration works**

Run: `cargo test -p tuillem-db`
Expected: 2 tests pass

- [ ] **Step 4: Write sessions module with tests**

`crates/tuillem-db/src/sessions.rs`:
```rust
use crate::{Db, DbError};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<String>,
    pub tags: Vec<String>,
}

impl Db {
    pub fn create_session(&self, title: &str) -> Result<Session, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        self.conn.execute(
            "INSERT INTO sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, title, now_str, now_str],
        )?;
        Ok(Session {
            id,
            title: title.to_string(),
            created_at: now,
            updated_at: now,
            metadata: None,
            tags: vec![],
        })
    }

    pub fn get_session(&self, id: &str) -> Result<Session, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at, metadata FROM sessions WHERE id = ?1",
        )?;
        let session = stmt
            .query_row(rusqlite::params![id], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    metadata: row.get(4)?,
                    tags: vec![],
                })
            })
            .map_err(|_| DbError::NotFound(format!("session {}", id)))?;

        let tags = self.get_session_tags(&session.id)?;
        Ok(Session { tags, ..session })
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at, metadata FROM sessions ORDER BY updated_at DESC",
        )?;
        let sessions: Vec<Session> = stmt
            .query_map([], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    metadata: row.get(4)?,
                    tags: vec![],
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut result = vec![];
        for session in sessions {
            let tags = self.get_session_tags(&session.id)?;
            result.push(Session { tags, ..session });
        }
        Ok(result)
    }

    pub fn update_session_title(&self, id: &str, title: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![title, now, id],
        )?;
        if rows == 0 {
            return Err(DbError::NotFound(format!("session {}", id)));
        }
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<(), DbError> {
        let rows = self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![id])?;
        if rows == 0 {
            return Err(DbError::NotFound(format!("session {}", id)));
        }
        Ok(())
    }

    pub fn add_session_tag(&self, session_id: &str, tag: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO session_tags (session_id, tag) VALUES (?1, ?2)",
            rusqlite::params![session_id, tag],
        )?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, session_id],
        )?;
        Ok(())
    }

    pub fn remove_session_tag(&self, session_id: &str, tag: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM session_tags WHERE session_id = ?1 AND tag = ?2",
            rusqlite::params![session_id, tag],
        )?;
        Ok(())
    }

    fn get_session_tags(&self, session_id: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM session_tags WHERE session_id = ?1 ORDER BY tag")?;
        let tags = stmt
            .query_map(rusqlite::params![session_id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use crate::Db;

    #[test]
    fn test_create_and_get_session() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("Test Chat").unwrap();
        assert_eq!(session.title, "Test Chat");

        let fetched = db.get_session(&session.id).unwrap();
        assert_eq!(fetched.title, "Test Chat");
        assert!(fetched.tags.is_empty());
    }

    #[test]
    fn test_list_sessions_ordered_by_updated() {
        let db = Db::open_in_memory().unwrap();
        let s1 = db.create_session("First").unwrap();
        let _s2 = db.create_session("Second").unwrap();
        db.update_session_title(&s1.id, "First Updated").unwrap();

        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].title, "First Updated");
    }

    #[test]
    fn test_session_tags() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("Tagged").unwrap();
        db.add_session_tag(&session.id, "research").unwrap();
        db.add_session_tag(&session.id, "code").unwrap();
        db.add_session_tag(&session.id, "research").unwrap(); // duplicate ignored

        let fetched = db.get_session(&session.id).unwrap();
        assert_eq!(fetched.tags, vec!["code", "research"]);

        db.remove_session_tag(&session.id, "code").unwrap();
        let fetched = db.get_session(&session.id).unwrap();
        assert_eq!(fetched.tags, vec!["research"]);
    }

    #[test]
    fn test_delete_session() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("To Delete").unwrap();
        db.delete_session(&session.id).unwrap();
        assert!(db.get_session(&session.id).is_err());
    }

    #[test]
    fn test_delete_nonexistent_session() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.delete_session("nonexistent").is_err());
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p tuillem-db`
Expected: all tests pass (lib + sessions)

- [ ] **Step 6: Write messages module with tests**

`crates/tuillem-db/src/messages.rs`:
```rust
use crate::{Db, DbError};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DbError> {
        match s {
            "user" => Ok(Role::User),
            "assistant" => Ok(Role::Assistant),
            "system" => Ok(Role::System),
            "tool" => Ok(Role::Tool),
            _ => Err(DbError::Migration(format!("unknown role: {}", s))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Text,
    Thinking,
    ToolCall,
    ToolResult,
}

impl BlockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Text => "text",
            BlockType::Thinking => "thinking",
            BlockType::ToolCall => "tool_call",
            BlockType::ToolResult => "tool_result",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DbError> {
        match s {
            "text" => Ok(BlockType::Text),
            "thinking" => Ok(BlockType::Thinking),
            "tool_call" => Ok(BlockType::ToolCall),
            "tool_result" => Ok(BlockType::ToolResult),
            _ => Err(DbError::Migration(format!("unknown block type: {}", s))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: Role,
    pub content: Option<String>,
    pub model_id: Option<String>,
    pub provider_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub token_usage_in: Option<i64>,
    pub token_usage_out: Option<i64>,
    pub latency_ms: Option<i64>,
    pub parent_message_id: Option<String>,
    pub blocks: Vec<MessageBlock>,
}

#[derive(Debug, Clone)]
pub struct MessageBlock {
    pub id: String,
    pub message_id: String,
    pub block_type: BlockType,
    pub content: Option<String>,
    pub sequence: i32,
    pub compressed: bool,
}

pub struct NewMessage<'a> {
    pub session_id: &'a str,
    pub role: Role,
    pub content: Option<&'a str>,
    pub model_id: Option<&'a str>,
    pub provider_name: Option<&'a str>,
    pub parent_message_id: Option<&'a str>,
}

pub struct NewBlock<'a> {
    pub block_type: BlockType,
    pub content: &'a str,
    pub sequence: i32,
}

impl Db {
    pub fn create_message(&self, msg: &NewMessage, blocks: &[NewBlock]) -> Result<Message, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO messages (id, session_id, role, content, model_id, provider_name, created_at, parent_message_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                msg.session_id,
                msg.role.as_str(),
                msg.content,
                msg.model_id,
                msg.provider_name,
                now_str,
                msg.parent_message_id,
            ],
        )?;

        let mut created_blocks = vec![];
        for block in blocks {
            let block_id = Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO message_blocks (id, message_id, block_type, content, sequence)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    block_id,
                    id,
                    block.block_type.as_str(),
                    block.content,
                    block.sequence,
                ],
            )?;
            created_blocks.push(MessageBlock {
                id: block_id,
                message_id: id.clone(),
                block_type: block.block_type.clone(),
                content: Some(block.content.to_string()),
                sequence: block.sequence,
                compressed: false,
            });
        }

        // Update session updated_at
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now_str, msg.session_id],
        )?;

        Ok(Message {
            id,
            session_id: msg.session_id.to_string(),
            role: msg.role.clone(),
            content: msg.content.map(|s| s.to_string()),
            model_id: msg.model_id.map(|s| s.to_string()),
            provider_name: msg.provider_name.map(|s| s.to_string()),
            created_at: now,
            token_usage_in: None,
            token_usage_out: None,
            latency_ms: None,
            parent_message_id: msg.parent_message_id.map(|s| s.to_string()),
            blocks: created_blocks,
        })
    }

    pub fn update_message_usage(
        &self,
        message_id: &str,
        tokens_in: i64,
        tokens_out: i64,
        latency_ms: i64,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE messages SET token_usage_in = ?1, token_usage_out = ?2, latency_ms = ?3 WHERE id = ?4",
            rusqlite::params![tokens_in, tokens_out, latency_ms, message_id],
        )?;
        Ok(())
    }

    pub fn get_session_messages(&self, session_id: &str) -> Result<Vec<Message>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, model_id, provider_name, created_at,
                    token_usage_in, token_usage_out, latency_ms, parent_message_id
             FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;
        let messages: Vec<Message> = stmt
            .query_map(rusqlite::params![session_id], |row| {
                let created_str: String = row.get(6)?;
                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: Role::from_str(&row.get::<_, String>(2)?).unwrap(),
                    content: row.get(3)?,
                    model_id: row.get(4)?,
                    provider_name: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    token_usage_in: row.get(7)?,
                    token_usage_out: row.get(8)?,
                    latency_ms: row.get(9)?,
                    parent_message_id: row.get(10)?,
                    blocks: vec![],
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut result = vec![];
        for msg in messages {
            let blocks = self.get_message_blocks(&msg.id)?;
            result.push(Message { blocks, ..msg });
        }
        Ok(result)
    }

    fn get_message_blocks(&self, message_id: &str) -> Result<Vec<MessageBlock>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, message_id, block_type, content, sequence, compressed
             FROM message_blocks WHERE message_id = ?1 ORDER BY sequence ASC",
        )?;
        let blocks = stmt
            .query_map(rusqlite::params![message_id], |row| {
                Ok(MessageBlock {
                    id: row.get(0)?,
                    message_id: row.get(1)?,
                    block_type: BlockType::from_str(&row.get::<_, String>(2)?).unwrap(),
                    content: row.get(3)?,
                    sequence: row.get(4)?,
                    compressed: row.get::<_, i32>(5)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(blocks)
    }

    pub fn compress_thinking_blocks(&self, older_than_days: i64) -> Result<usize, DbError> {
        let cutoff = (Utc::now() - chrono::Duration::days(older_than_days)).to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE message_blocks SET content = NULL, compressed = 1
             WHERE block_type = 'thinking' AND compressed = 0
             AND message_id IN (SELECT id FROM messages WHERE created_at < ?1)",
            rusqlite::params![cutoff],
        )?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use crate::Db;
    use crate::messages::{BlockType, NewBlock, NewMessage, Role};

    fn setup() -> Db {
        let db = Db::open_in_memory().unwrap();
        db.create_session("Test").unwrap();
        db
    }

    fn get_session_id(db: &Db) -> String {
        db.list_sessions().unwrap()[0].id.clone()
    }

    #[test]
    fn test_create_message_with_blocks() {
        let db = setup();
        let sid = get_session_id(&db);

        let msg = db
            .create_message(
                &NewMessage {
                    session_id: &sid,
                    role: Role::Assistant,
                    content: Some("Hello!"),
                    model_id: Some("claude-sonnet-4-20250514"),
                    provider_name: Some("anthropic"),
                    parent_message_id: None,
                },
                &[
                    NewBlock {
                        block_type: BlockType::Thinking,
                        content: "Let me think...",
                        sequence: 0,
                    },
                    NewBlock {
                        block_type: BlockType::Text,
                        content: "Hello!",
                        sequence: 1,
                    },
                ],
            )
            .unwrap();

        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.blocks.len(), 2);
        assert_eq!(msg.blocks[0].block_type, BlockType::Thinking);
        assert_eq!(msg.blocks[1].block_type, BlockType::Text);
    }

    #[test]
    fn test_get_session_messages_ordered() {
        let db = setup();
        let sid = get_session_id(&db);

        db.create_message(
            &NewMessage {
                session_id: &sid,
                role: Role::User,
                content: Some("Hi"),
                model_id: None,
                provider_name: None,
                parent_message_id: None,
            },
            &[],
        )
        .unwrap();

        db.create_message(
            &NewMessage {
                session_id: &sid,
                role: Role::Assistant,
                content: Some("Hello!"),
                model_id: Some("claude-sonnet-4-20250514"),
                provider_name: Some("anthropic"),
                parent_message_id: None,
            },
            &[NewBlock {
                block_type: BlockType::Text,
                content: "Hello!",
                sequence: 0,
            }],
        )
        .unwrap();

        let messages = db.get_session_messages(&sid).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[1].blocks.len(), 1);
    }

    #[test]
    fn test_update_message_usage() {
        let db = setup();
        let sid = get_session_id(&db);
        let msg = db
            .create_message(
                &NewMessage {
                    session_id: &sid,
                    role: Role::Assistant,
                    content: Some("Hi"),
                    model_id: Some("gpt-4o"),
                    provider_name: Some("openai"),
                    parent_message_id: None,
                },
                &[],
            )
            .unwrap();

        db.update_message_usage(&msg.id, 100, 50, 1200).unwrap();

        let messages = db.get_session_messages(&sid).unwrap();
        assert_eq!(messages[0].token_usage_in, Some(100));
        assert_eq!(messages[0].token_usage_out, Some(50));
        assert_eq!(messages[0].latency_ms, Some(1200));
    }

    #[test]
    fn test_compress_thinking_blocks() {
        let db = setup();
        let sid = get_session_id(&db);
        db.create_message(
            &NewMessage {
                session_id: &sid,
                role: Role::Assistant,
                content: Some("Answer"),
                model_id: None,
                provider_name: None,
                parent_message_id: None,
            },
            &[NewBlock {
                block_type: BlockType::Thinking,
                content: "secret thoughts",
                sequence: 0,
            }],
        )
        .unwrap();

        // Compress everything (0 days = compress all)
        let count = db.compress_thinking_blocks(0).unwrap();
        assert_eq!(count, 1);

        let messages = db.get_session_messages(&sid).unwrap();
        assert!(messages[0].blocks[0].compressed);
        assert!(messages[0].blocks[0].content.is_none());
    }

    #[test]
    fn test_cascade_delete() {
        let db = setup();
        let sid = get_session_id(&db);
        db.create_message(
            &NewMessage {
                session_id: &sid,
                role: Role::User,
                content: Some("Hi"),
                model_id: None,
                provider_name: None,
                parent_message_id: None,
            },
            &[NewBlock {
                block_type: BlockType::Text,
                content: "Hi",
                sequence: 0,
            }],
        )
        .unwrap();

        db.delete_session(&sid).unwrap();
        let messages = db.get_session_messages(&sid).unwrap();
        assert!(messages.is_empty());
    }
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p tuillem-db`
Expected: all tests pass

- [ ] **Step 8: Write search module with tests**

`crates/tuillem-db/src/search.rs`:
```rust
use crate::{Db, DbError};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub session_id: String,
    pub session_title: String,
    pub message_id: String,
    pub content_snippet: String,
    pub role: String,
}

impl Db {
    pub fn search_messages(&self, query: &str) -> Result<Vec<SearchResult>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.session_id, s.title, m.id, snippet(messages_fts, 0, '**', '**', '...', 32), m.role
             FROM messages_fts
             JOIN messages m ON m.rowid = messages_fts.rowid
             JOIN sessions s ON s.id = m.session_id
             WHERE messages_fts MATCH ?1
             ORDER BY rank
             LIMIT 50",
        )?;
        let results = stmt
            .query_map(rusqlite::params![query], |row| {
                Ok(SearchResult {
                    session_id: row.get(0)?,
                    session_title: row.get(1)?,
                    message_id: row.get(2)?,
                    content_snippet: row.get(3)?,
                    role: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    pub fn search_sessions_by_tag(&self, tag: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id FROM session_tags WHERE tag = ?1")?;
        let ids = stmt
            .query_map(rusqlite::params![tag], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use crate::Db;
    use crate::messages::{BlockType, NewBlock, NewMessage, Role};

    #[test]
    fn test_fts_search() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("Rust Help").unwrap();

        db.create_message(
            &NewMessage {
                session_id: &session.id,
                role: Role::User,
                content: Some("How do I use iterators in Rust?"),
                model_id: None,
                provider_name: None,
                parent_message_id: None,
            },
            &[],
        )
        .unwrap();

        db.create_message(
            &NewMessage {
                session_id: &session.id,
                role: Role::Assistant,
                content: Some("Iterators in Rust are lazy and composable."),
                model_id: Some("claude-sonnet-4-20250514"),
                provider_name: Some("anthropic"),
                parent_message_id: None,
            },
            &[NewBlock {
                block_type: BlockType::Text,
                content: "Iterators in Rust are lazy and composable.",
                sequence: 0,
            }],
        )
        .unwrap();

        let results = db.search_messages("iterators").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].session_title, "Rust Help");
    }

    #[test]
    fn test_search_no_results() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("Chat").unwrap();
        db.create_message(
            &NewMessage {
                session_id: &session.id,
                role: Role::User,
                content: Some("Hello world"),
                model_id: None,
                provider_name: None,
                parent_message_id: None,
            },
            &[],
        )
        .unwrap();

        let results = db.search_messages("quantum").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_by_tag() {
        let db = Db::open_in_memory().unwrap();
        let s1 = db.create_session("One").unwrap();
        let s2 = db.create_session("Two").unwrap();
        db.add_session_tag(&s1.id, "rust").unwrap();
        db.add_session_tag(&s2.id, "rust").unwrap();
        db.add_session_tag(&s2.id, "async").unwrap();

        let rust_sessions = db.search_sessions_by_tag("rust").unwrap();
        assert_eq!(rust_sessions.len(), 2);

        let async_sessions = db.search_sessions_by_tag("async").unwrap();
        assert_eq!(async_sessions.len(), 1);
    }
}
```

- [ ] **Step 9: Run all DB tests**

Run: `cargo test -p tuillem-db`
Expected: all tests pass

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat(db): add SQLite storage layer with sessions, messages, FTS5 search"
```

---

## Task 4: Markdown Rendering (tuillem-markdown)

**Files:**
- Create: `crates/tuillem-markdown/src/lib.rs` (replace stub)
- Create: `crates/tuillem-markdown/src/parser.rs`
- Create: `crates/tuillem-markdown/src/renderer.rs`
- Create: `crates/tuillem-markdown/src/highlight.rs`

- [ ] **Step 1: Write the highlight module**

`crates/tuillem-markdown/src/highlight.rs`:
```rust
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight<'a>(&self, code: &'a str, language: &str) -> Vec<Vec<Span<'a>>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        code.lines()
            .map(|line| {
                match h.highlight_line(line, &self.syntax_set) {
                    Ok(ranges) => ranges
                        .into_iter()
                        .map(|(style, text)| {
                            let fg = Color::Rgb(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            );
                            Span::styled(text.to_string(), Style::default().fg(fg))
                        })
                        .collect(),
                    Err(_) => vec![Span::raw(line.to_string())],
                }
            })
            .collect()
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let h = Highlighter::new();
        let lines = h.highlight("fn main() {}", "rust");
        assert!(!lines.is_empty());
        assert!(!lines[0].is_empty());
    }

    #[test]
    fn test_highlight_unknown_language() {
        let h = Highlighter::new();
        let lines = h.highlight("some text", "nonexistent_lang_xyz");
        assert!(!lines.is_empty());
    }
}
```

- [ ] **Step 2: Write the parser module**

`crates/tuillem-markdown/src/parser.rs`:
```rust
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, CodeBlockKind};

#[derive(Debug, Clone, PartialEq)]
pub enum MdElement {
    Heading(u8, String),
    Paragraph(Vec<InlineElement>),
    CodeBlock { language: String, code: String },
    InlineCode(String),
    List(Vec<ListItem>),
    OrderedList(Vec<ListItem>),
    BlockQuote(Vec<MdElement>),
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    ThematicBreak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub content: Vec<InlineElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineElement {
    Text(String),
    Bold(String),
    Italic(String),
    Strikethrough(String),
    Code(String),
    Link { text: String, url: String },
}

pub fn parse(markdown: &str) -> Vec<MdElement> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);
    let events: Vec<Event> = parser.collect();

    let mut elements = vec![];
    let mut i = 0;

    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading { level, .. }) => {
                let level = *level as u8;
                i += 1;
                let mut text = String::new();
                while i < events.len() {
                    match &events[i] {
                        Event::Text(t) => text.push_str(t),
                        Event::End(TagEnd::Heading(_)) => break,
                        _ => {}
                    }
                    i += 1;
                }
                elements.push(MdElement::Heading(level, text));
            }
            Event::Start(Tag::Paragraph) => {
                i += 1;
                let inlines = collect_inlines(&events, &mut i);
                elements.push(MdElement::Paragraph(inlines));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                i += 1;
                let mut code = String::new();
                while i < events.len() {
                    match &events[i] {
                        Event::Text(t) => code.push_str(t),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                    i += 1;
                }
                elements.push(MdElement::CodeBlock { language, code });
            }
            Event::Start(Tag::List(None)) => {
                i += 1;
                let items = collect_list_items(&events, &mut i);
                elements.push(MdElement::List(items));
            }
            Event::Start(Tag::List(Some(_))) => {
                i += 1;
                let items = collect_list_items(&events, &mut i);
                elements.push(MdElement::OrderedList(items));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                i += 1;
                let mut inner_text = String::new();
                while i < events.len() {
                    match &events[i] {
                        Event::Text(t) => inner_text.push_str(t),
                        Event::SoftBreak | Event::HardBreak => inner_text.push('\n'),
                        Event::End(TagEnd::BlockQuote(_)) => break,
                        _ => {}
                    }
                    i += 1;
                }
                elements.push(MdElement::BlockQuote(vec![MdElement::Paragraph(vec![
                    InlineElement::Text(inner_text),
                ])]));
            }
            Event::Start(Tag::Table(_alignments)) => {
                i += 1;
                let (headers, rows) = collect_table(&events, &mut i);
                elements.push(MdElement::Table { headers, rows });
            }
            Event::Rule => {
                elements.push(MdElement::ThematicBreak);
            }
            _ => {}
        }
        i += 1;
    }
    elements
}

fn collect_inlines(events: &[Event], i: &mut usize) -> Vec<InlineElement> {
    let mut inlines = vec![];
    while *i < events.len() {
        match &events[*i] {
            Event::Text(t) => inlines.push(InlineElement::Text(t.to_string())),
            Event::Code(t) => inlines.push(InlineElement::Code(t.to_string())),
            Event::Start(Tag::Strong) => {
                *i += 1;
                let mut text = String::new();
                while *i < events.len() {
                    match &events[*i] {
                        Event::Text(t) => text.push_str(t),
                        Event::End(TagEnd::Strong) => break,
                        _ => {}
                    }
                    *i += 1;
                }
                inlines.push(InlineElement::Bold(text));
            }
            Event::Start(Tag::Emphasis) => {
                *i += 1;
                let mut text = String::new();
                while *i < events.len() {
                    match &events[*i] {
                        Event::Text(t) => text.push_str(t),
                        Event::End(TagEnd::Emphasis) => break,
                        _ => {}
                    }
                    *i += 1;
                }
                inlines.push(InlineElement::Italic(text));
            }
            Event::Start(Tag::Strikethrough) => {
                *i += 1;
                let mut text = String::new();
                while *i < events.len() {
                    match &events[*i] {
                        Event::Text(t) => text.push_str(t),
                        Event::End(TagEnd::Strikethrough) => break,
                        _ => {}
                    }
                    *i += 1;
                }
                inlines.push(InlineElement::Strikethrough(text));
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let url = dest_url.to_string();
                *i += 1;
                let mut text = String::new();
                while *i < events.len() {
                    match &events[*i] {
                        Event::Text(t) => text.push_str(t),
                        Event::End(TagEnd::Link) => break,
                        _ => {}
                    }
                    *i += 1;
                }
                inlines.push(InlineElement::Link { text, url });
            }
            Event::SoftBreak => inlines.push(InlineElement::Text(" ".to_string())),
            Event::HardBreak => inlines.push(InlineElement::Text("\n".to_string())),
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Item) => break,
            _ => {}
        }
        *i += 1;
    }
    inlines
}

fn collect_list_items(events: &[Event], i: &mut usize) -> Vec<ListItem> {
    let mut items = vec![];
    while *i < events.len() {
        match &events[*i] {
            Event::Start(Tag::Item) => {
                *i += 1;
                // Skip inner paragraph start if present
                if let Some(Event::Start(Tag::Paragraph)) = events.get(*i) {
                    *i += 1;
                }
                let inlines = collect_inlines(events, i);
                items.push(ListItem { content: inlines });
            }
            Event::End(TagEnd::List(_)) => break,
            _ => {}
        }
        *i += 1;
    }
    items
}

fn collect_table(events: &[Event], i: &mut usize) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers = vec![];
    let mut rows = vec![];
    let mut current_row: Vec<String> = vec![];
    let mut in_head = false;

    while *i < events.len() {
        match &events[*i] {
            Event::Start(Tag::TableHead) => in_head = true,
            Event::End(TagEnd::TableHead) => {
                headers = current_row.clone();
                current_row.clear();
                in_head = false;
            }
            Event::Start(Tag::TableRow) => current_row.clear(),
            Event::End(TagEnd::TableRow) => {
                if !in_head {
                    rows.push(current_row.clone());
                }
                current_row.clear();
            }
            Event::Start(Tag::TableCell) => {}
            Event::End(TagEnd::TableCell) => {}
            Event::Text(t) => current_row.push(t.to_string()),
            Event::End(TagEnd::Table) => break,
            _ => {}
        }
        *i += 1;
    }
    (headers, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let elements = parse("# Hello World");
        assert_eq!(elements, vec![MdElement::Heading(1, "Hello World".to_string())]);
    }

    #[test]
    fn test_parse_paragraph_with_bold() {
        let elements = parse("This is **bold** text");
        match &elements[0] {
            MdElement::Paragraph(inlines) => {
                assert_eq!(inlines.len(), 3);
                assert_eq!(inlines[0], InlineElement::Text("This is ".to_string()));
                assert_eq!(inlines[1], InlineElement::Bold("bold".to_string()));
                assert_eq!(inlines[2], InlineElement::Text(" text".to_string()));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let elements = parse(md);
        match &elements[0] {
            MdElement::CodeBlock { language, code } => {
                assert_eq!(language, "rust");
                assert_eq!(code, "fn main() {}\n");
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn test_parse_list() {
        let md = "- one\n- two\n- three";
        let elements = parse(md);
        match &elements[0] {
            MdElement::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_parse_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
        let elements = parse(md);
        match &elements[0] {
            MdElement::Table { headers, rows } => {
                assert_eq!(headers, &["A", "B"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], vec!["1", "2"]);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn test_parse_link() {
        let md = "Click [here](https://example.com)";
        let elements = parse(md);
        match &elements[0] {
            MdElement::Paragraph(inlines) => {
                assert!(inlines.iter().any(|i| matches!(i, InlineElement::Link { .. })));
            }
            _ => panic!("expected paragraph"),
        }
    }
}
```

- [ ] **Step 3: Write the renderer module**

`crates/tuillem-markdown/src/renderer.rs`:
```rust
use crate::highlight::Highlighter;
use crate::parser::{InlineElement, MdElement};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

pub struct MdRenderer {
    highlighter: Highlighter,
    heading_color: Color,
    link_color: Color,
    code_bg: Color,
    code_fg: Color,
    blockquote_color: Color,
    border_color: Color,
}

impl MdRenderer {
    pub fn new() -> Self {
        Self {
            highlighter: Highlighter::new(),
            heading_color: Color::Rgb(137, 180, 250),
            link_color: Color::Rgb(116, 199, 236),
            code_bg: Color::Rgb(17, 17, 27),
            code_fg: Color::Rgb(205, 214, 244),
            blockquote_color: Color::Rgb(108, 112, 134),
            border_color: Color::Rgb(69, 71, 90),
        }
    }

    pub fn render<'a>(&self, elements: &[MdElement]) -> Text<'a> {
        let mut lines: Vec<Line> = vec![];

        for element in elements {
            match element {
                MdElement::Heading(level, text) => {
                    let prefix = "#".repeat(*level as usize);
                    lines.push(Line::from(Span::styled(
                        format!("{} {}", prefix, text),
                        Style::default()
                            .fg(self.heading_color)
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                }
                MdElement::Paragraph(inlines) => {
                    lines.push(self.render_inlines(inlines));
                    lines.push(Line::from(""));
                }
                MdElement::CodeBlock { language, code } => {
                    let lang_display = if language.is_empty() {
                        "text".to_string()
                    } else {
                        language.clone()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("┌─ {} ", lang_display),
                        Style::default().fg(self.border_color),
                    )));

                    let highlighted = self.highlighter.highlight(code.trim_end(), &lang_display);
                    for hl_line in highlighted {
                        let mut spans = vec![Span::styled(
                            "│ ",
                            Style::default().fg(self.border_color),
                        )];
                        spans.extend(hl_line.into_iter().map(|s| {
                            Span::styled(
                                s.content.to_string(),
                                s.style.bg(self.code_bg),
                            )
                        }));
                        lines.push(Line::from(spans));
                    }

                    lines.push(Line::from(Span::styled(
                        "└─",
                        Style::default().fg(self.border_color),
                    )));
                    lines.push(Line::from(""));
                }
                MdElement::List(items) => {
                    for (_, item) in items.iter().enumerate() {
                        let mut spans = vec![Span::raw("  • ")];
                        spans.extend(self.inline_to_spans(&item.content));
                        lines.push(Line::from(spans));
                    }
                    lines.push(Line::from(""));
                }
                MdElement::OrderedList(items) => {
                    for (idx, item) in items.iter().enumerate() {
                        let mut spans = vec![Span::raw(format!("  {}. ", idx + 1))];
                        spans.extend(self.inline_to_spans(&item.content));
                        lines.push(Line::from(spans));
                    }
                    lines.push(Line::from(""));
                }
                MdElement::BlockQuote(inner) => {
                    let inner_text = self.render(inner);
                    for line in inner_text.lines {
                        let mut spans = vec![Span::styled(
                            "▎ ",
                            Style::default().fg(self.blockquote_color),
                        )];
                        for span in line.spans {
                            spans.push(Span::styled(
                                span.content.to_string(),
                                span.style.fg(self.blockquote_color),
                            ));
                        }
                        lines.push(Line::from(spans));
                    }
                }
                MdElement::Table { headers, rows } => {
                    let all_rows: Vec<&Vec<String>> =
                        std::iter::once(headers).chain(rows.iter()).collect();
                    let col_count = headers.len();
                    let mut widths = vec![0usize; col_count];
                    for row in &all_rows {
                        for (j, cell) in row.iter().enumerate() {
                            if j < col_count {
                                widths[j] = widths[j].max(cell.len());
                            }
                        }
                    }

                    // Top border
                    let top: String = widths
                        .iter()
                        .map(|w| "─".repeat(w + 2))
                        .collect::<Vec<_>>()
                        .join("┬");
                    lines.push(Line::from(Span::styled(
                        format!("┌{}┐", top),
                        Style::default().fg(self.border_color),
                    )));

                    // Header
                    let header_cells: String = headers
                        .iter()
                        .enumerate()
                        .map(|(j, h)| format!(" {:<width$} ", h, width = widths[j]))
                        .collect::<Vec<_>>()
                        .join("│");
                    lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(self.border_color)),
                        Span::styled(
                            header_cells,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("│", Style::default().fg(self.border_color)),
                    ]));

                    // Separator
                    let sep: String = widths
                        .iter()
                        .map(|w| "─".repeat(w + 2))
                        .collect::<Vec<_>>()
                        .join("┼");
                    lines.push(Line::from(Span::styled(
                        format!("├{}┤", sep),
                        Style::default().fg(self.border_color),
                    )));

                    // Data rows
                    for row in rows {
                        let cells: String = row
                            .iter()
                            .enumerate()
                            .map(|(j, cell)| {
                                let w = if j < widths.len() { widths[j] } else { cell.len() };
                                format!(" {:<width$} ", cell, width = w)
                            })
                            .collect::<Vec<_>>()
                            .join("│");
                        lines.push(Line::from(vec![
                            Span::styled("│", Style::default().fg(self.border_color)),
                            Span::raw(cells),
                            Span::styled("│", Style::default().fg(self.border_color)),
                        ]));
                    }

                    // Bottom border
                    let bottom: String = widths
                        .iter()
                        .map(|w| "─".repeat(w + 2))
                        .collect::<Vec<_>>()
                        .join("┴");
                    lines.push(Line::from(Span::styled(
                        format!("└{}┘", bottom),
                        Style::default().fg(self.border_color),
                    )));
                    lines.push(Line::from(""));
                }
                MdElement::ThematicBreak => {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(self.border_color),
                    )));
                    lines.push(Line::from(""));
                }
                MdElement::InlineCode(_) => {} // handled at inline level
            }
        }
        Text::from(lines)
    }

    fn render_inlines<'a>(&self, inlines: &[InlineElement]) -> Line<'a> {
        Line::from(self.inline_to_spans(inlines))
    }

    fn inline_to_spans<'a>(&self, inlines: &[InlineElement]) -> Vec<Span<'a>> {
        inlines
            .iter()
            .map(|inline| match inline {
                InlineElement::Text(t) => Span::raw(t.clone()),
                InlineElement::Bold(t) => {
                    Span::styled(t.clone(), Style::default().add_modifier(Modifier::BOLD))
                }
                InlineElement::Italic(t) => {
                    Span::styled(t.clone(), Style::default().add_modifier(Modifier::ITALIC))
                }
                InlineElement::Strikethrough(t) => Span::styled(
                    t.clone(),
                    Style::default().add_modifier(Modifier::CROSSED_OUT),
                ),
                InlineElement::Code(t) => Span::styled(
                    format!(" {} ", t),
                    Style::default().fg(self.code_fg).bg(self.code_bg),
                ),
                InlineElement::Link { text, url } => Span::styled(
                    format!("{} ({})", text, url),
                    Style::default()
                        .fg(self.link_color)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            })
            .collect()
    }
}

impl Default for MdRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_render_heading() {
        let r = MdRenderer::new();
        let elements = parse("# Test");
        let text = r.render(&elements);
        let first_line = &text.lines[0];
        assert!(first_line.to_string().contains("# Test"));
    }

    #[test]
    fn test_render_code_block() {
        let r = MdRenderer::new();
        let elements = parse("```rust\nlet x = 1;\n```");
        let text = r.render(&elements);
        let content: String = text.lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(content.contains("rust"));
        assert!(content.contains("let x = 1;"));
    }

    #[test]
    fn test_render_table() {
        let r = MdRenderer::new();
        let elements = parse("| A | B |\n|---|---|\n| 1 | 2 |");
        let text = r.render(&elements);
        let content: String = text.lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(content.contains("A"));
        assert!(content.contains("1"));
    }

    #[test]
    fn test_render_list() {
        let r = MdRenderer::new();
        let elements = parse("- alpha\n- beta");
        let text = r.render(&elements);
        let content: String = text.lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(content.contains("alpha"));
        assert!(content.contains("beta"));
    }
}
```

- [ ] **Step 4: Write the lib.rs public API**

`crates/tuillem-markdown/src/lib.rs`:
```rust
//! Terminal markdown rendering for tuillem.

pub mod highlight;
pub mod parser;
pub mod renderer;

use ratatui::text::Text;

pub fn render_markdown(markdown: &str) -> Text<'_> {
    let elements = parser::parse(markdown);
    let renderer = renderer::MdRenderer::new();
    renderer.render(&elements)
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown_e2e() {
        let md = "# Hello\n\nThis is **bold** and *italic*.\n\n```python\nprint('hi')\n```\n";
        let text = render_markdown(md);
        assert!(!text.lines.is_empty());
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p tuillem-markdown`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(markdown): add terminal markdown renderer with syntax highlighting and tables"
```

---

## Task 5: Provider Abstraction (tuillem-provider)

**Files:**
- Create: `crates/tuillem-provider/src/lib.rs` (replace stub)
- Create: `crates/tuillem-provider/src/anthropic.rs`
- Create: `crates/tuillem-provider/src/openai.rs`
- Create: `crates/tuillem-provider/src/ollama.rs`

- [ ] **Step 1: Write the provider trait and types**

`crates/tuillem-provider/src/lib.rs`:
```rust
//! LLM provider abstraction for tuillem.

pub mod anthropic;
pub mod ollama;
pub mod openai;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use thiserror::Error;
use tokio_stream::Stream;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },
    #[error("stream error: {0}")]
    Stream(String),
    #[error("configuration error: {0}")]
    Config(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum StreamDelta {
    Text(String),
    Thinking(String),
    ToolCallStart { id: String, name: String },
    ToolCallDelta(String),
    ToolCallEnd,
    Usage { input_tokens: u64, output_tokens: u64 },
    Done,
}

pub type ChatResponseStream =
    Pin<Box<dyn Stream<Item = Result<StreamDelta, ProviderError>> + Send>>;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    pub context_window: Option<u64>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream, ProviderError>;
    fn models(&self) -> Vec<ModelInfo>;
    fn name(&self) -> &str;
}

pub fn create_provider(
    config: &tuillem_config::ProviderConfig,
) -> Result<Box<dyn Provider>, ProviderError> {
    match config.provider_type {
        tuillem_config::ProviderType::Anthropic => {
            let api_key = config
                .api_key
                .as_deref()
                .ok_or_else(|| ProviderError::Config("Anthropic requires api_key".into()))?;
            Ok(Box::new(anthropic::AnthropicProvider::new(
                api_key,
                config.models.clone(),
            )))
        }
        tuillem_config::ProviderType::Openai | tuillem_config::ProviderType::Openrouter => {
            let api_key = config
                .api_key
                .as_deref()
                .ok_or_else(|| ProviderError::Config("OpenAI-compatible requires api_key".into()))?;
            let base_url = config
                .base_url
                .as_deref()
                .unwrap_or(match config.provider_type {
                    tuillem_config::ProviderType::Openrouter => "https://openrouter.ai/api/v1",
                    _ => "https://api.openai.com/v1",
                });
            Ok(Box::new(openai::OpenAiProvider::new(
                &config.name,
                api_key,
                base_url,
                config.models.clone(),
            )))
        }
        tuillem_config::ProviderType::Ollama => {
            let base_url = config
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            Ok(Box::new(ollama::OllamaProvider::new(
                base_url,
                config.models.clone(),
            )))
        }
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

- [ ] **Step 2: Write the Anthropic provider**

`crates/tuillem-provider/src/anthropic.rs`:
```rust
use crate::{
    ChatMessage, ChatRequest, ChatResponseStream, ModelInfo, Provider, ProviderError, StreamDelta,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    models: Vec<String>,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, models: Vec<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            models: if models.is_empty() {
                vec!["claude-sonnet-4-20250514".to_string()]
            } else {
                models
            },
        }
    }

    fn format_messages(&self, messages: &[ChatMessage]) -> serde_json::Value {
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();
        serde_json::Value::Array(msgs)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream, ProviderError> {
        let mut body = json!({
            "model": request.model,
            "messages": self.format_messages(&request.messages),
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(system) = &request.system {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                status,
                message: text,
            });
        }

        let stream = response.bytes_stream();
        let event_stream = stream.map(move |chunk| {
            let chunk = chunk.map_err(ProviderError::Http)?;
            let text = String::from_utf8_lossy(&chunk);
            let mut deltas = vec![];

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        deltas.push(StreamDelta::Done);
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        let event_type = event["type"].as_str().unwrap_or("");
                        match event_type {
                            "content_block_delta" => {
                                let delta = &event["delta"];
                                match delta["type"].as_str() {
                                    Some("text_delta") => {
                                        if let Some(text) = delta["text"].as_str() {
                                            deltas.push(StreamDelta::Text(text.to_string()));
                                        }
                                    }
                                    Some("thinking_delta") => {
                                        if let Some(thinking) = delta["thinking"].as_str() {
                                            deltas
                                                .push(StreamDelta::Thinking(thinking.to_string()));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "message_delta" => {
                                if let (Some(input), Some(output)) = (
                                    event["usage"]["input_tokens"].as_u64(),
                                    event["usage"]["output_tokens"].as_u64(),
                                ) {
                                    deltas.push(StreamDelta::Usage {
                                        input_tokens: input,
                                        output_tokens: output,
                                    });
                                }
                            }
                            "message_stop" => {
                                deltas.push(StreamDelta::Done);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Return the first meaningful delta (simplified — real impl would use a proper SSE parser)
            Ok(deltas.into_iter().next().unwrap_or(StreamDelta::Done))
        });

        Ok(Box::pin(event_stream))
    }

    fn models(&self) -> Vec<ModelInfo> {
        self.models
            .iter()
            .map(|id| ModelInfo {
                id: id.clone(),
                name: id.clone(),
                supports_streaming: true,
                supports_thinking: true,
                context_window: Some(200_000),
            })
            .collect()
    }

    fn name(&self) -> &str {
        "anthropic"
    }
}
```

- [ ] **Step 3: Write the OpenAI-compatible provider**

`crates/tuillem-provider/src/openai.rs`:
```rust
use crate::{
    ChatMessage, ChatRequest, ChatResponseStream, ModelInfo, Provider, ProviderError, StreamDelta,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;

pub struct OpenAiProvider {
    provider_name: String,
    client: Client,
    api_key: String,
    base_url: String,
    models: Vec<String>,
}

impl OpenAiProvider {
    pub fn new(name: &str, api_key: &str, base_url: &str, models: Vec<String>) -> Self {
        Self {
            provider_name: name.to_string(),
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            models,
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream, ProviderError> {
        let mut messages: Vec<serde_json::Value> = vec![];

        if let Some(system) = &request.system {
            messages.push(json!({ "role": "system", "content": system }));
        }

        for msg in &request.messages {
            messages.push(json!({ "role": msg.role, "content": msg.content }));
        }

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(max) = request.max_tokens {
            body["max_tokens"] = json!(max);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                status,
                message: text,
            });
        }

        let stream = response.bytes_stream();
        let event_stream = stream.map(move |chunk| {
            let chunk = chunk.map_err(ProviderError::Http)?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        return Ok(StreamDelta::Done);
                    }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) =
                            event["choices"][0]["delta"]["content"].as_str()
                        {
                            return Ok(StreamDelta::Text(content.to_string()));
                        }
                    }
                }
            }
            Ok(StreamDelta::Done)
        });

        Ok(Box::pin(event_stream))
    }

    fn models(&self) -> Vec<ModelInfo> {
        self.models
            .iter()
            .map(|id| ModelInfo {
                id: id.clone(),
                name: id.clone(),
                supports_streaming: true,
                supports_thinking: false,
                context_window: None,
            })
            .collect()
    }

    fn name(&self) -> &str {
        &self.provider_name
    }
}
```

- [ ] **Step 4: Write the Ollama provider**

`crates/tuillem-provider/src/ollama.rs`:
```rust
use crate::{
    ChatMessage, ChatRequest, ChatResponseStream, ModelInfo, Provider, ProviderError, StreamDelta,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    models: Vec<String>,
}

impl OllamaProvider {
    pub fn new(base_url: &str, models: Vec<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            models,
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream, ProviderError> {
        let mut messages: Vec<serde_json::Value> = vec![];

        if let Some(system) = &request.system {
            messages.push(json!({ "role": "system", "content": system }));
        }

        for msg in &request.messages {
            messages.push(json!({ "role": msg.role, "content": msg.content }));
        }

        let body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                status,
                message: text,
            });
        }

        let stream = response.bytes_stream();
        let event_stream = stream.map(move |chunk| {
            let chunk = chunk.map_err(ProviderError::Http)?;
            let text = String::from_utf8_lossy(&chunk);

            // Ollama streams newline-delimited JSON
            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                    if event["done"].as_bool() == Some(true) {
                        return Ok(StreamDelta::Done);
                    }
                    if let Some(content) = event["message"]["content"].as_str() {
                        return Ok(StreamDelta::Text(content.to_string()));
                    }
                }
            }
            Ok(StreamDelta::Done)
        });

        Ok(Box::pin(event_stream))
    }

    fn models(&self) -> Vec<ModelInfo> {
        self.models
            .iter()
            .map(|id| ModelInfo {
                id: id.clone(),
                name: id.clone(),
                supports_streaming: true,
                supports_thinking: false,
                context_window: None,
            })
            .collect()
    }

    fn name(&self) -> &str {
        "ollama"
    }
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p tuillem-provider`
Expected: successful build

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(provider): add LLM provider abstraction with Anthropic, OpenAI, and Ollama"
```

---

## Task 6: Plugin System (tuillem-plugin)

**Files:**
- Create: `crates/tuillem-plugin/src/lib.rs` (replace stub)

- [ ] **Step 1: Write the plugin host with tests**

`crates/tuillem-plugin/src/lib.rs`:
```rust
//! External process plugin host for tuillem.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("tool execution failed: {0}")]
    Execution(String),
    #[error("tool timed out after {0:?}")]
    Timeout(Duration),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInput {
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolOutput {
    pub output: Option<String>,
    pub error: Option<String>,
}

pub struct PluginHost {
    tools: HashMap<String, tuillem_config::ToolConfig>,
}

impl PluginHost {
    pub fn new(tools: Vec<tuillem_config::ToolConfig>) -> Self {
        let map = tools.into_iter().map(|t| (t.name.clone(), t)).collect();
        Self { tools: map }
    }

    pub fn list_tools(&self) -> Vec<&tuillem_config::ToolConfig> {
        self.tools.values().collect()
    }

    pub fn get_tool(&self, name: &str) -> Option<&tuillem_config::ToolConfig> {
        self.tools.get(name)
    }

    pub fn requires_confirmation(&self, name: &str) -> bool {
        self.tools.get(name).is_some_and(|t| t.confirm)
    }

    pub async fn invoke(&self, name: &str, input: serde_json::Value) -> Result<ToolOutput, PluginError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let timeout = parse_duration(&tool.timeout);
        let tool_input = ToolInput {
            name: name.to_string(),
            input,
        };
        let input_json = serde_json::to_string(&tool_input)?;

        // Split command into program and args
        let parts: Vec<&str> = tool.command.split_whitespace().collect();
        let (program, args) = parts
            .split_first()
            .ok_or_else(|| PluginError::Execution("empty command".to_string()))?;

        let mut cmd = Command::new(program);
        cmd.args(args.iter());
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        for (k, v) in &tool.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input_json.as_bytes()).await?;
            stdin.shutdown().await?;
        }

        let result = tokio::time::timeout(timeout, async {
            let mut stdout = String::new();
            let mut stderr = String::new();

            if let Some(mut out) = child.stdout.take() {
                out.read_to_string(&mut stdout).await?;
            }
            if let Some(mut err) = child.stderr.take() {
                err.read_to_string(&mut stderr).await?;
            }

            let status = child.wait().await?;

            if !status.success() {
                return Ok(ToolOutput {
                    output: None,
                    error: Some(if stderr.is_empty() {
                        format!("process exited with {}", status)
                    } else {
                        stderr
                    }),
                });
            }

            // Try to parse as JSON ToolOutput, fall back to raw stdout
            match serde_json::from_str::<ToolOutput>(&stdout) {
                Ok(parsed) => Ok(parsed),
                Err(_) => Ok(ToolOutput {
                    output: Some(stdout),
                    error: None,
                }),
            }
        })
        .await;

        match result {
            Ok(inner) => inner.map_err(|e: std::io::Error| PluginError::Io(e)),
            Err(_) => {
                let _ = child.kill().await;
                Err(PluginError::Timeout(timeout))
            }
        }
    }
}

fn parse_duration(s: &str) -> Duration {
    let s = s.trim();
    if let Some(secs) = s.strip_suffix('s') {
        Duration::from_secs(secs.parse().unwrap_or(30))
    } else if let Some(mins) = s.strip_suffix('m') {
        Duration::from_secs(mins.parse::<u64>().unwrap_or(1) * 60)
    } else {
        Duration::from_secs(s.parse().unwrap_or(30))
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tuillem_config::ToolConfig;
    use std::collections::HashMap;

    fn echo_tool() -> ToolConfig {
        ToolConfig {
            name: "echo".to_string(),
            description: "Echo input back".to_string(),
            command: "cat".to_string(), // cat reads stdin and writes to stdout
            input_schema: None,
            timeout: "5s".to_string(),
            confirm: false,
            env: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_invoke_tool() {
        let host = PluginHost::new(vec![echo_tool()]);
        let result = host
            .invoke("echo", serde_json::json!({"hello": "world"}))
            .await
            .unwrap();
        assert!(result.output.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let host = PluginHost::new(vec![]);
        let result = host.invoke("nonexistent", serde_json::json!({})).await;
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }

    #[test]
    fn test_requires_confirmation() {
        let mut tool = echo_tool();
        tool.confirm = true;
        let host = PluginHost::new(vec![tool]);
        assert!(host.requires_confirmation("echo"));
    }

    #[test]
    fn test_list_tools() {
        let host = PluginHost::new(vec![echo_tool()]);
        assert_eq!(host.list_tools().len(), 1);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Duration::from_secs(30));
        assert_eq!(parse_duration("2m"), Duration::from_secs(120));
        assert_eq!(parse_duration("45"), Duration::from_secs(45));
    }

    #[tokio::test]
    async fn test_timeout() {
        let tool = ToolConfig {
            name: "slow".to_string(),
            description: "Slow tool".to_string(),
            command: "sleep 60".to_string(),
            input_schema: None,
            timeout: "1s".to_string(),
            confirm: false,
            env: HashMap::new(),
        };
        let host = PluginHost::new(vec![tool]);
        let result = host.invoke("slow", serde_json::json!({})).await;
        assert!(matches!(result, Err(PluginError::Timeout(_))));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p tuillem-plugin`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(plugin): add external process plugin host with timeout and confirmation"
```

---

## Task 7: Core Coordinator (tuillem-core)

**Files:**
- Create: `crates/tuillem-core/src/lib.rs` (replace stub)
- Create: `crates/tuillem-core/src/actions.rs`
- Create: `crates/tuillem-core/src/state.rs`
- Create: `crates/tuillem-core/src/coordinator.rs`

- [ ] **Step 1: Define actions and events**

`crates/tuillem-core/src/actions.rs`:
```rust
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum Action {
    // Session management
    CreateSession { title: String },
    SelectSession { id: String },
    DeleteSession { id: String },
    RenameSession { id: String, title: String },
    AddTag { session_id: String, tag: String },
    RemoveTag { session_id: String, tag: String },

    // Messages
    SendMessage { content: String },
    RegenerateLastResponse,

    // Model
    SwitchModel { provider: String, model: String },

    // Search
    Search { query: String },

    // Tools
    ConfirmToolCall { approved: bool },

    // UI
    Quit,
}

#[derive(Debug, Clone)]
pub enum Event {
    // Session events
    SessionCreated { id: String, title: String },
    SessionSelected { id: String },
    SessionDeleted { id: String },
    SessionRenamed { id: String, title: String },
    SessionsLoaded { sessions: Vec<SessionSummary> },

    // Message events
    MessagesLoaded { messages: Vec<MessageView> },
    StreamDelta { text: String },
    ThinkingDelta { text: String },
    StreamDone { message_id: String },
    ResponseError { error: String },

    // Search
    SearchResults { results: Vec<SearchResultView> },

    // Tool events
    ToolCallRequested {
        tool_name: String,
        input: Value,
        requires_confirm: bool,
    },
    ToolCallResult { output: String },

    // Model
    ModelSwitched { provider: String, model: String },
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MessageView {
    pub id: String,
    pub role: String,
    pub content: Option<String>,
    pub model_id: Option<String>,
    pub provider_name: Option<String>,
    pub blocks: Vec<BlockView>,
    pub token_usage_in: Option<i64>,
    pub token_usage_out: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct BlockView {
    pub block_type: String,
    pub content: Option<String>,
    pub compressed: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResultView {
    pub session_id: String,
    pub session_title: String,
    pub snippet: String,
}
```

- [ ] **Step 2: Define app state**

`crates/tuillem-core/src/state.rs`:
```rust
use crate::actions::{MessageView, SearchResultView, SessionSummary};

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub sessions: Vec<SessionSummary>,
    pub active_session_id: Option<String>,
    pub messages: Vec<MessageView>,
    pub streaming_text: String,
    pub streaming_thinking: String,
    pub is_streaming: bool,
    pub current_provider: String,
    pub current_model: String,
    pub search_results: Vec<SearchResultView>,
    pub search_query: String,
    pub pending_tool_call: Option<PendingToolCall>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub tool_name: String,
    pub input: serde_json::Value,
}

impl AppState {
    pub fn new(provider: String, model: String) -> Self {
        Self {
            current_provider: provider,
            current_model: model,
            ..Default::default()
        }
    }

    pub fn apply_event(&mut self, event: &crate::actions::Event) {
        use crate::actions::Event;
        match event {
            Event::SessionCreated { id, title } => {
                self.sessions.insert(
                    0,
                    SessionSummary {
                        id: id.clone(),
                        title: title.clone(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        tags: vec![],
                    },
                );
                self.active_session_id = Some(id.clone());
            }
            Event::SessionSelected { id } => {
                self.active_session_id = Some(id.clone());
            }
            Event::SessionDeleted { id } => {
                self.sessions.retain(|s| s.id != *id);
                if self.active_session_id.as_deref() == Some(id) {
                    self.active_session_id = self.sessions.first().map(|s| s.id.clone());
                    self.messages.clear();
                }
            }
            Event::SessionRenamed { id, title } => {
                if let Some(s) = self.sessions.iter_mut().find(|s| s.id == *id) {
                    s.title = title.clone();
                }
            }
            Event::SessionsLoaded { sessions } => {
                self.sessions = sessions.clone();
            }
            Event::MessagesLoaded { messages } => {
                self.messages = messages.clone();
                self.streaming_text.clear();
                self.streaming_thinking.clear();
                self.is_streaming = false;
            }
            Event::StreamDelta { text } => {
                self.is_streaming = true;
                self.streaming_text.push_str(text);
            }
            Event::ThinkingDelta { text } => {
                self.is_streaming = true;
                self.streaming_thinking.push_str(text);
            }
            Event::StreamDone { .. } => {
                self.is_streaming = false;
                self.streaming_text.clear();
                self.streaming_thinking.clear();
            }
            Event::ResponseError { error } => {
                self.is_streaming = false;
                self.error = Some(error.clone());
            }
            Event::SearchResults { results } => {
                self.search_results = results.clone();
            }
            Event::ToolCallRequested {
                tool_name,
                input,
                requires_confirm,
            } => {
                if *requires_confirm {
                    self.pending_tool_call = Some(PendingToolCall {
                        tool_name: tool_name.clone(),
                        input: input.clone(),
                    });
                }
            }
            Event::ToolCallResult { .. } => {
                self.pending_tool_call = None;
            }
            Event::ModelSwitched { provider, model } => {
                self.current_provider = provider.clone();
                self.current_model = model.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Event;

    #[test]
    fn test_session_created() {
        let mut state = AppState::default();
        state.apply_event(&Event::SessionCreated {
            id: "abc".into(),
            title: "New Chat".into(),
        });
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.active_session_id.as_deref(), Some("abc"));
    }

    #[test]
    fn test_session_deleted_selects_next() {
        let mut state = AppState::default();
        state.apply_event(&Event::SessionCreated { id: "a".into(), title: "A".into() });
        state.apply_event(&Event::SessionCreated { id: "b".into(), title: "B".into() });
        state.apply_event(&Event::SessionSelected { id: "a".into() });
        state.apply_event(&Event::SessionDeleted { id: "a".into() });
        assert_eq!(state.active_session_id.as_deref(), Some("b"));
    }

    #[test]
    fn test_streaming_deltas() {
        let mut state = AppState::default();
        state.apply_event(&Event::StreamDelta { text: "Hello ".into() });
        state.apply_event(&Event::StreamDelta { text: "world".into() });
        assert!(state.is_streaming);
        assert_eq!(state.streaming_text, "Hello world");

        state.apply_event(&Event::StreamDone { message_id: "m1".into() });
        assert!(!state.is_streaming);
        assert!(state.streaming_text.is_empty());
    }

    #[test]
    fn test_model_switch() {
        let mut state = AppState::new("anthropic".into(), "claude-sonnet-4-20250514".into());
        state.apply_event(&Event::ModelSwitched {
            provider: "ollama".into(),
            model: "llama3".into(),
        });
        assert_eq!(state.current_provider, "ollama");
        assert_eq!(state.current_model, "llama3");
    }
}
```

- [ ] **Step 3: Write the coordinator**

`crates/tuillem-core/src/coordinator.rs`:
```rust
use crate::actions::{Action, BlockView, Event, MessageView, SearchResultView, SessionSummary};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::{error, info};
use tuillem_db::Db;
use tuillem_plugin::PluginHost;
use tuillem_provider::{ChatMessage, ChatRequest, Provider, StreamDelta};

pub struct Coordinator {
    db: Db,
    providers: std::collections::HashMap<String, Box<dyn Provider>>,
    plugin_host: PluginHost,
    current_provider: String,
    current_model: String,
    system_prompt: Option<String>,
    active_session_id: Option<String>,
}

impl Coordinator {
    pub fn new(
        db: Db,
        providers: std::collections::HashMap<String, Box<dyn Provider>>,
        plugin_host: PluginHost,
        default_provider: String,
        default_model: String,
        system_prompt: Option<String>,
    ) -> Self {
        Self {
            db,
            providers,
            plugin_host,
            current_provider: default_provider,
            current_model: default_model,
            system_prompt,
            active_session_id: None,
        }
    }

    pub async fn run(
        mut self,
        mut action_rx: mpsc::UnboundedReceiver<Action>,
        event_tx: mpsc::UnboundedSender<Event>,
    ) {
        // Load initial sessions
        if let Ok(sessions) = self.db.list_sessions() {
            let summaries = sessions
                .iter()
                .map(|s| SessionSummary {
                    id: s.id.clone(),
                    title: s.title.clone(),
                    updated_at: s.updated_at.to_rfc3339(),
                    tags: s.tags.clone(),
                })
                .collect();
            let _ = event_tx.send(Event::SessionsLoaded { sessions: summaries });
        }

        while let Some(action) = action_rx.recv().await {
            match action {
                Action::CreateSession { title } => {
                    match self.db.create_session(&title) {
                        Ok(session) => {
                            self.active_session_id = Some(session.id.clone());
                            let _ = event_tx.send(Event::SessionCreated {
                                id: session.id.clone(),
                                title: session.title,
                            });
                            let _ = event_tx.send(Event::MessagesLoaded { messages: vec![] });
                        }
                        Err(e) => error!("Failed to create session: {}", e),
                    }
                }
                Action::SelectSession { id } => {
                    self.active_session_id = Some(id.clone());
                    let _ = event_tx.send(Event::SessionSelected { id: id.clone() });
                    self.load_messages(&id, &event_tx);
                }
                Action::DeleteSession { id } => {
                    if self.db.delete_session(&id).is_ok() {
                        let _ = event_tx.send(Event::SessionDeleted { id });
                    }
                }
                Action::RenameSession { id, title } => {
                    if self.db.update_session_title(&id, &title).is_ok() {
                        let _ = event_tx.send(Event::SessionRenamed { id, title });
                    }
                }
                Action::AddTag { session_id, tag } => {
                    let _ = self.db.add_session_tag(&session_id, &tag);
                }
                Action::RemoveTag { session_id, tag } => {
                    let _ = self.db.remove_session_tag(&session_id, &tag);
                }
                Action::SendMessage { content } => {
                    self.handle_send_message(content, &event_tx).await;
                }
                Action::SwitchModel { provider, model } => {
                    self.current_provider = provider.clone();
                    self.current_model = model.clone();
                    let _ = event_tx.send(Event::ModelSwitched { provider, model });
                }
                Action::Search { query } => {
                    if let Ok(results) = self.db.search_messages(&query) {
                        let views = results
                            .into_iter()
                            .map(|r| SearchResultView {
                                session_id: r.session_id,
                                session_title: r.session_title,
                                snippet: r.content_snippet,
                            })
                            .collect();
                        let _ = event_tx.send(Event::SearchResults { results: views });
                    }
                }
                Action::ConfirmToolCall { approved: _ } => {
                    // Tool call confirmation handled in send_message flow
                }
                Action::RegenerateLastResponse => {
                    // Future: delete last assistant message and re-send
                }
                Action::Quit => break,
            }
        }
        info!("Coordinator shutting down");
    }

    fn load_messages(&self, session_id: &str, event_tx: &mpsc::UnboundedSender<Event>) {
        match self.db.get_session_messages(session_id) {
            Ok(messages) => {
                let views = messages
                    .into_iter()
                    .map(|m| MessageView {
                        id: m.id,
                        role: m.role.as_str().to_string(),
                        content: m.content,
                        model_id: m.model_id,
                        provider_name: m.provider_name,
                        blocks: m
                            .blocks
                            .into_iter()
                            .map(|b| BlockView {
                                block_type: b.block_type.as_str().to_string(),
                                content: b.content,
                                compressed: b.compressed,
                            })
                            .collect(),
                        token_usage_in: m.token_usage_in,
                        token_usage_out: m.token_usage_out,
                    })
                    .collect();
                let _ = event_tx.send(Event::MessagesLoaded { messages: views });
            }
            Err(e) => error!("Failed to load messages: {}", e),
        }
    }

    async fn handle_send_message(
        &self,
        content: String,
        event_tx: &mpsc::UnboundedSender<Event>,
    ) {
        let session_id = match &self.active_session_id {
            Some(id) => id.clone(),
            None => return,
        };

        // Store user message
        let _ = self.db.create_message(
            &tuillem_db::messages::NewMessage {
                session_id: &session_id,
                role: tuillem_db::messages::Role::User,
                content: Some(&content),
                model_id: None,
                provider_name: None,
                parent_message_id: None,
            },
            &[tuillem_db::messages::NewBlock {
                block_type: tuillem_db::messages::BlockType::Text,
                content: &content,
                sequence: 0,
            }],
        );

        // Build message history
        let history = match self.db.get_session_messages(&session_id) {
            Ok(msgs) => msgs
                .iter()
                .filter(|m| m.content.is_some())
                .map(|m| ChatMessage {
                    role: m.role.as_str().to_string(),
                    content: m.content.clone().unwrap_or_default(),
                })
                .collect::<Vec<_>>(),
            Err(_) => return,
        };

        let provider = match self.providers.get(&self.current_provider) {
            Some(p) => p,
            None => {
                let _ = event_tx.send(Event::ResponseError {
                    error: format!("Provider '{}' not found", self.current_provider),
                });
                return;
            }
        };

        let request = ChatRequest {
            model: self.current_model.clone(),
            messages: history,
            system: self.system_prompt.clone(),
            max_tokens: Some(4096),
            temperature: None,
        };

        match provider.send(request).await {
            Ok(mut stream) => {
                let mut full_text = String::new();
                let mut full_thinking = String::new();
                let mut input_tokens = 0u64;
                let mut output_tokens = 0u64;
                let start = std::time::Instant::now();

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(StreamDelta::Text(t)) => {
                            full_text.push_str(&t);
                            let _ = event_tx.send(Event::StreamDelta { text: t });
                        }
                        Ok(StreamDelta::Thinking(t)) => {
                            full_thinking.push_str(&t);
                            let _ = event_tx.send(Event::ThinkingDelta { text: t });
                        }
                        Ok(StreamDelta::Usage {
                            input_tokens: i,
                            output_tokens: o,
                        }) => {
                            input_tokens = i;
                            output_tokens = o;
                        }
                        Ok(StreamDelta::Done) => break,
                        Ok(_) => {}
                        Err(e) => {
                            let _ = event_tx.send(Event::ResponseError {
                                error: e.to_string(),
                            });
                            return;
                        }
                    }
                }

                let latency = start.elapsed().as_millis() as i64;

                // Store assistant message
                let mut blocks = vec![];
                let mut seq = 0;
                if !full_thinking.is_empty() {
                    blocks.push(tuillem_db::messages::NewBlock {
                        block_type: tuillem_db::messages::BlockType::Thinking,
                        content: &full_thinking,
                        sequence: seq,
                    });
                    seq += 1;
                }
                blocks.push(tuillem_db::messages::NewBlock {
                    block_type: tuillem_db::messages::BlockType::Text,
                    content: &full_text,
                    sequence: seq,
                });

                match self.db.create_message(
                    &tuillem_db::messages::NewMessage {
                        session_id: &session_id,
                        role: tuillem_db::messages::Role::Assistant,
                        content: Some(&full_text),
                        model_id: Some(&self.current_model),
                        provider_name: Some(&self.current_provider),
                        parent_message_id: None,
                    },
                    &blocks,
                ) {
                    Ok(msg) => {
                        let _ = self.db.update_message_usage(
                            &msg.id,
                            input_tokens as i64,
                            output_tokens as i64,
                            latency,
                        );
                        let _ = event_tx.send(Event::StreamDone {
                            message_id: msg.id,
                        });
                        // Reload messages to get clean state
                        self.load_messages(&session_id, event_tx);
                    }
                    Err(e) => error!("Failed to store message: {}", e),
                }
            }
            Err(e) => {
                let _ = event_tx.send(Event::ResponseError {
                    error: e.to_string(),
                });
            }
        }
    }
}
```

- [ ] **Step 4: Write lib.rs**

`crates/tuillem-core/src/lib.rs`:
```rust
//! Core coordinator and app state for tuillem.

pub mod actions;
pub mod coordinator;
pub mod state;

pub use actions::{Action, Event};
pub use coordinator::Coordinator;
pub use state::AppState;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p tuillem-core`
Expected: successful build

- [ ] **Step 6: Run state tests**

Run: `cargo test -p tuillem-core`
Expected: all state tests pass

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(core): add coordinator, app state, and action/event system"
```

---

## Task 8: TUI Layer (tuillem-tui)

**Files:**
- Create: `crates/tuillem-tui/src/lib.rs` (replace stub)
- Create: `crates/tuillem-tui/src/theme.rs`
- Create: `crates/tuillem-tui/src/app.rs`
- Create: `crates/tuillem-tui/src/sidebar.rs`
- Create: `crates/tuillem-tui/src/conversation.rs`
- Create: `crates/tuillem-tui/src/input.rs`

- [ ] **Step 1: Write the theme module**

`crates/tuillem-tui/src/theme.rs`:
```rust
use ratatui::style::{Color, Style, Modifier};
use tuillem_config::ThemeColors;

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub sidebar_bg: Color,
    pub sidebar_fg: Color,
    pub sidebar_selected: Color,
    pub user_msg_bg: Color,
    pub assistant_msg_bg: Color,
    pub thinking_fg: Color,
    pub accent: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub border: Color,
    pub code_bg: Color,
    pub code_fg: Color,
    pub heading: Color,
    pub link: Color,
    pub tag: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            sidebar_bg: Color::Rgb(24, 24, 37),
            sidebar_fg: Color::Rgb(205, 214, 244),
            sidebar_selected: Color::Rgb(137, 180, 250),
            user_msg_bg: Color::Rgb(49, 50, 68),
            assistant_msg_bg: Color::Rgb(30, 30, 46),
            thinking_fg: Color::Rgb(108, 112, 134),
            accent: Color::Rgb(137, 180, 250),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            border: Color::Rgb(69, 71, 90),
            code_bg: Color::Rgb(17, 17, 27),
            code_fg: Color::Rgb(205, 214, 244),
            heading: Color::Rgb(137, 180, 250),
            link: Color::Rgb(116, 199, 236),
            tag: Color::Rgb(245, 194, 231),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(239, 241, 245),
            fg: Color::Rgb(76, 79, 105),
            sidebar_bg: Color::Rgb(230, 233, 239),
            sidebar_fg: Color::Rgb(76, 79, 105),
            sidebar_selected: Color::Rgb(30, 102, 245),
            user_msg_bg: Color::Rgb(220, 224, 232),
            assistant_msg_bg: Color::Rgb(239, 241, 245),
            thinking_fg: Color::Rgb(140, 143, 161),
            accent: Color::Rgb(30, 102, 245),
            error: Color::Rgb(210, 15, 57),
            success: Color::Rgb(64, 160, 43),
            warning: Color::Rgb(223, 142, 29),
            border: Color::Rgb(188, 192, 204),
            code_bg: Color::Rgb(230, 233, 239),
            code_fg: Color::Rgb(76, 79, 105),
            heading: Color::Rgb(30, 102, 245),
            link: Color::Rgb(4, 165, 229),
            tag: Color::Rgb(234, 118, 203),
        }
    }

    pub fn from_config(name: &str, custom_themes: &std::collections::HashMap<String, ThemeColors>) -> Self {
        let base = match name {
            "light" => Self::light(),
            _ => Self::dark(),
        };

        if let Some(colors) = custom_themes.get(name) {
            base.apply_overrides(colors)
        } else {
            base
        }
    }

    fn apply_overrides(mut self, colors: &ThemeColors) -> Self {
        if let Some(c) = &colors.bg { self.bg = parse_hex(c); }
        if let Some(c) = &colors.fg { self.fg = parse_hex(c); }
        if let Some(c) = &colors.sidebar_bg { self.sidebar_bg = parse_hex(c); }
        if let Some(c) = &colors.sidebar_fg { self.sidebar_fg = parse_hex(c); }
        if let Some(c) = &colors.sidebar_selected { self.sidebar_selected = parse_hex(c); }
        if let Some(c) = &colors.user_msg_bg { self.user_msg_bg = parse_hex(c); }
        if let Some(c) = &colors.assistant_msg_bg { self.assistant_msg_bg = parse_hex(c); }
        if let Some(c) = &colors.thinking_fg { self.thinking_fg = parse_hex(c); }
        if let Some(c) = &colors.accent { self.accent = parse_hex(c); }
        if let Some(c) = &colors.error { self.error = parse_hex(c); }
        if let Some(c) = &colors.success { self.success = parse_hex(c); }
        if let Some(c) = &colors.warning { self.warning = parse_hex(c); }
        if let Some(c) = &colors.border { self.border = parse_hex(c); }
        if let Some(c) = &colors.code_bg { self.code_bg = parse_hex(c); }
        if let Some(c) = &colors.code_fg { self.code_fg = parse_hex(c); }
        if let Some(c) = &colors.heading { self.heading = parse_hex(c); }
        if let Some(c) = &colors.link { self.link = parse_hex(c); }
        if let Some(c) = &colors.tag { self.tag = parse_hex(c); }
        self
    }

    // Convenience style builders
    pub fn sidebar_style(&self) -> Style {
        Style::default().fg(self.sidebar_fg).bg(self.sidebar_bg)
    }

    pub fn sidebar_selected_style(&self) -> Style {
        Style::default().fg(self.sidebar_selected).bg(self.sidebar_bg).add_modifier(Modifier::BOLD)
    }

    pub fn user_message_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.user_msg_bg)
    }

    pub fn assistant_message_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.assistant_msg_bg)
    }

    pub fn thinking_style(&self) -> Style {
        Style::default().fg(self.thinking_fg)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }
}

fn parse_hex(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    Color::White
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_hex("00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_hex("invalid"), Color::White);
    }

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_eq!(theme.bg, Color::Rgb(30, 30, 46));
    }

    #[test]
    fn test_custom_theme_override() {
        let mut custom = std::collections::HashMap::new();
        custom.insert("myTheme".to_string(), ThemeColors {
            bg: Some("#000000".to_string()),
            fg: None,
            sidebar_bg: None,
            sidebar_fg: None,
            sidebar_selected: None,
            user_msg_bg: None,
            assistant_msg_bg: None,
            thinking_fg: None,
            accent: None,
            error: None,
            success: None,
            warning: None,
            border: None,
            code_bg: None,
            code_fg: None,
            heading: None,
            link: None,
            tag: None,
        });
        let theme = Theme::from_config("myTheme", &custom);
        assert_eq!(theme.bg, Color::Rgb(0, 0, 0));
        // Unspecified colors fall back to dark theme
        assert_eq!(theme.fg, Color::Rgb(205, 214, 244));
    }
}
```

- [ ] **Step 2: Write the sidebar widget**

`crates/tuillem-tui/src/sidebar.rs`:
```rust
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use tuillem_core::actions::SessionSummary;

pub struct Sidebar {
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_input: String,
    pub search_focused: bool,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            search_input: String::new(),
            search_focused: false,
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        sessions: &[SessionSummary],
        theme: &Theme,
    ) {
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(theme.border_style())
            .style(theme.sidebar_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 {
            return;
        }

        // Search box (top 1 line)
        let search_area = Rect::new(inner.x, inner.y, inner.width, 1);
        let search_style = if self.search_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.sidebar_fg)
        };
        let search_text = if self.search_input.is_empty() {
            Span::styled(" / search...", Style::default().fg(theme.thinking_fg))
        } else {
            Span::styled(format!(" /{}", self.search_input), search_style)
        };
        frame.render_widget(Paragraph::new(Line::from(search_text)), search_area);

        // Session list
        let list_area = Rect::new(inner.x, inner.y + 2, inner.width, inner.height.saturating_sub(2));

        let filtered: Vec<&SessionSummary> = if self.search_input.is_empty() {
            sessions.iter().collect()
        } else {
            let query = self.search_input.to_lowercase();
            sessions
                .iter()
                .filter(|s| {
                    s.title.to_lowercase().contains(&query)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .collect()
        };

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let style = if i == self.selected {
                    theme.sidebar_selected_style()
                } else {
                    theme.sidebar_style()
                };

                let mut spans = vec![Span::styled(&session.title, style)];
                if !session.tags.is_empty() {
                    let tag_str = session.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ");
                    spans.push(Span::styled(
                        format!(" {}", tag_str),
                        Style::default().fg(theme.tag),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(self.selected));

        let list = List::new(items).highlight_style(
            Style::default()
                .fg(theme.sidebar_selected)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, list_area, &mut state);
    }

    pub fn move_up(&mut self, count: usize) {
        self.selected = self.selected.saturating_sub(count);
    }

    pub fn move_down(&mut self, session_count: usize, count: usize) {
        if session_count > 0 {
            self.selected = (self.selected + count).min(session_count - 1);
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: Write the conversation widget**

`crates/tuillem-tui/src/conversation.rs`:
```rust
use crate::theme::Theme;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use tuillem_core::actions::{BlockView, MessageView};

pub struct Conversation {
    pub scroll_offset: u16,
    pub expanded_thinking: std::collections::HashSet<usize>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            expanded_thinking: std::collections::HashSet::new(),
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        messages: &[MessageView],
        streaming_text: &str,
        streaming_thinking: &str,
        is_streaming: bool,
        current_model: &str,
        theme: &Theme,
    ) {
        let block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(theme.bg));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Model indicator at top
        let model_line = Line::from(vec![
            Span::styled(" Model: ", Style::default().fg(theme.thinking_fg)),
            Span::styled(current_model, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]);
        frame.render_widget(Paragraph::new(model_line), Rect::new(inner.x, inner.y, inner.width, 1));

        // Messages area
        let msg_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));

        let mut lines: Vec<Line> = vec![];

        for (idx, msg) in messages.iter().enumerate() {
            let is_user = msg.role == "user";
            lines.push(Line::from(""));

            // Role label
            let role_label = if is_user { "You" } else { "Assistant" };
            let model_suffix = if !is_user {
                msg.model_id
                    .as_deref()
                    .map(|m| format!(" ({})", m))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let label_style = if is_user {
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{}{}", role_label, model_suffix), label_style),
            ]));

            // Thinking blocks (collapsible)
            for block in &msg.blocks {
                if block.block_type == "thinking" {
                    let is_expanded = self.expanded_thinking.contains(&idx);
                    if is_expanded {
                        if let Some(content) = &block.content {
                            lines.push(Line::from(Span::styled(
                                " Thinking:",
                                Style::default().fg(theme.thinking_fg).add_modifier(Modifier::ITALIC),
                            )));
                            for line in content.lines() {
                                lines.push(Line::from(Span::styled(
                                    format!("  {}", line),
                                    theme.thinking_style(),
                                )));
                            }
                        } else if block.compressed {
                            lines.push(Line::from(Span::styled(
                                " [thinking compressed]",
                                theme.thinking_style(),
                            )));
                        }
                    } else {
                        let label = if block.compressed {
                            " [thinking compressed]"
                        } else {
                            " [thinking hidden - press t to expand]"
                        };
                        lines.push(Line::from(Span::styled(label, theme.thinking_style())));
                    }
                }
            }

            // Message content
            if let Some(content) = &msg.content {
                let rendered = tuillem_markdown::render_markdown(content);
                for line in rendered.lines {
                    let alignment = if is_user {
                        Alignment::Right
                    } else {
                        Alignment::Left
                    };
                    lines.push(line.alignment(alignment));
                }
            }

            // Token usage
            if let (Some(tin), Some(tout)) = (msg.token_usage_in, msg.token_usage_out) {
                lines.push(Line::from(Span::styled(
                    format!(" tokens: {}in / {}out", tin, tout),
                    Style::default().fg(theme.thinking_fg),
                )));
            }
        }

        // Streaming content
        if is_streaming {
            lines.push(Line::from(""));
            if !streaming_thinking.is_empty() {
                let throbber = ['|', '/', '-', '\\'];
                let idx = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_millis()
                    / 250) as usize;
                lines.push(Line::from(Span::styled(
                    format!(" {} Thinking...", throbber[idx % 4]),
                    Style::default().fg(theme.thinking_fg).add_modifier(Modifier::ITALIC),
                )));
            }
            if !streaming_text.is_empty() {
                let rendered = tuillem_markdown::render_markdown(streaming_text);
                for line in rendered.lines {
                    lines.push(line);
                }
            }
        }

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        frame.render_widget(paragraph, msg_area);
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = u16::MAX; // Will be clamped by paragraph rendering
    }

    pub fn toggle_thinking(&mut self, message_index: usize) {
        if self.expanded_thinking.contains(&message_index) {
            self.expanded_thinking.remove(&message_index);
        } else {
            self.expanded_thinking.insert(message_index);
        }
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Write the input widget**

`crates/tuillem-tui/src/input.rs`:
```rust
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct Input {
    pub content: String,
    pub cursor_pos: usize,
    pub focused: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
            focused: true,
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        current_model: &str,
        is_streaming: bool,
        theme: &Theme,
    ) {
        let border_style = if self.focused {
            Style::default().fg(theme.accent)
        } else {
            theme.border_style()
        };

        let status = if is_streaming {
            Span::styled(" streaming... ", Style::default().fg(theme.warning))
        } else {
            Span::styled(
                format!(" {} ", current_model),
                Style::default().fg(theme.accent),
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Line::from(vec![
                Span::styled(" tuillem ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            ]))
            .title_bottom(Line::from(vec![
                Span::raw(" Enter: send | Shift+Enter: newline | Ctrl+E: editor "),
                status,
            ]).alignment(ratatui::layout::Alignment::Right));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let display_text = if self.content.is_empty() {
            Span::styled(
                "Type a message...",
                Style::default().fg(theme.thinking_fg),
            )
        } else {
            Span::raw(&self.content)
        };

        frame.render_widget(
            Paragraph::new(Line::from(display_text)),
            inner,
        );

        // Show cursor
        if self.focused {
            let cursor_x = inner.x + self.cursor_pos as u16;
            let cursor_y = inner.y;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos < self.content.len() {
            self.content.remove(self.cursor_pos);
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.content.remove(self.cursor_pos);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_pos < self.content.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_pos = self.content.len();
    }

    pub fn take_content(&mut self) -> String {
        let content = self.content.clone();
        self.content.clear();
        self.cursor_pos = 0;
        content
    }

    pub fn set_content(&mut self, content: String) {
        self.cursor_pos = content.len();
        self.content = content;
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_backspace() {
        let mut input = Input::new();
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.content, "hi");
        assert_eq!(input.cursor_pos, 2);

        input.backspace();
        assert_eq!(input.content, "h");
        assert_eq!(input.cursor_pos, 1);
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = Input::new();
        input.set_content("hello".to_string());
        assert_eq!(input.cursor_pos, 5);

        input.move_home();
        assert_eq!(input.cursor_pos, 0);

        input.move_end();
        assert_eq!(input.cursor_pos, 5);

        input.move_left();
        assert_eq!(input.cursor_pos, 4);
    }

    #[test]
    fn test_take_content() {
        let mut input = Input::new();
        input.set_content("message".to_string());
        let content = input.take_content();
        assert_eq!(content, "message");
        assert!(input.content.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }
}
```

- [ ] **Step 5: Write the app module (main layout + event loop)**

`crates/tuillem-tui/src/app.rs`:
```rust
use crate::conversation::Conversation;
use crate::input::Input;
use crate::sidebar::Sidebar;
use crate::theme::Theme;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;
use std::time::Duration;
use tokio::sync::mpsc;
use tuillem_core::{Action, AppState, Event};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Sidebar,
    Conversation,
    Input,
}

pub struct App {
    pub state: AppState,
    pub theme: Theme,
    pub sidebar: Sidebar,
    pub conversation: Conversation,
    pub input: Input,
    pub focus: Focus,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub should_quit: bool,
    pub editor_command: String,
}

impl App {
    pub fn new(
        state: AppState,
        theme: Theme,
        action_tx: mpsc::UnboundedSender<Action>,
        editor_command: String,
    ) -> Self {
        Self {
            state,
            theme,
            sidebar: Sidebar::new(),
            conversation: Conversation::new(),
            input: Input::new(),
            focus: Focus::Input,
            action_tx,
            should_quit: false,
            editor_command,
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.state.sessions.len().max(20) as u16.min(40)),
                Constraint::Min(40),
            ])
            .split(frame.area());

        let sidebar_area = chunks[0];
        let right_area = chunks[1];

        // Right side: conversation + input
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(right_area);

        let conversation_area = right_chunks[0];
        let input_area = right_chunks[1];

        self.sidebar
            .render(frame, sidebar_area, &self.state.sessions, &self.theme);
        self.conversation.render(
            frame,
            conversation_area,
            &self.state.messages,
            &self.state.streaming_text,
            &self.state.streaming_thinking,
            self.state.is_streaming,
            &self.state.current_model,
            &self.theme,
        );
        self.input.render(
            frame,
            input_area,
            &self.state.current_model,
            self.state.is_streaming,
            &self.theme,
        );
    }

    pub fn apply_event(&mut self, event: Event) {
        self.state.apply_event(&event);

        // Auto-scroll to bottom on new messages
        match &event {
            Event::StreamDelta { .. }
            | Event::ThinkingDelta { .. }
            | Event::MessagesLoaded { .. } => {
                self.conversation.scroll_to_bottom();
            }
            _ => {}
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        // Global keybindings
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.should_quit = true;
                let _ = self.action_tx.send(Action::Quit);
                return;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                let _ = self.action_tx.send(Action::CreateSession {
                    title: "New Chat".to_string(),
                });
                return;
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focus = match self.focus {
                    Focus::Sidebar => Focus::Conversation,
                    Focus::Conversation => Focus::Input,
                    Focus::Input => Focus::Sidebar,
                };
                self.input.focused = self.focus == Focus::Input;
                return;
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.focus = match self.focus {
                    Focus::Sidebar => Focus::Input,
                    Focus::Conversation => Focus::Sidebar,
                    Focus::Input => Focus::Conversation,
                };
                self.input.focused = self.focus == Focus::Input;
                return;
            }
            _ => {}
        }

        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::Conversation => self.handle_conversation_key(key),
            Focus::Input => self.handle_input_key(key),
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.conversation.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                self.conversation.scroll_down(3);
            }
            _ => {}
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.sidebar.move_down(self.state.sessions.len(), 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sidebar.move_up(1);
            }
            KeyCode::Char('g') => {
                self.sidebar.selected = 0;
            }
            KeyCode::Char('G') => {
                if !self.state.sessions.is_empty() {
                    self.sidebar.selected = self.state.sessions.len() - 1;
                }
            }
            KeyCode::Enter => {
                if let Some(session) = self.state.sessions.get(self.sidebar.selected) {
                    let _ = self.action_tx.send(Action::SelectSession {
                        id: session.id.clone(),
                    });
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(session) = self.state.sessions.get(self.sidebar.selected) {
                    let _ = self.action_tx.send(Action::DeleteSession {
                        id: session.id.clone(),
                    });
                }
            }
            KeyCode::Char('/') => {
                self.sidebar.search_focused = true;
            }
            _ => {}
        }
    }

    fn handle_conversation_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.conversation.scroll_down(1),
            KeyCode::Char('k') | KeyCode::Up => self.conversation.scroll_up(1),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.conversation.scroll_down(10);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.conversation.scroll_up(10);
            }
            KeyCode::Char('g') => self.conversation.scroll_offset = 0,
            KeyCode::Char('G') => self.conversation.scroll_to_bottom(),
            KeyCode::PageUp => self.conversation.scroll_up(20),
            KeyCode::PageDown => self.conversation.scroll_down(20),
            KeyCode::Char('t') => {
                // Toggle thinking for the nearest assistant message
                // Simple: toggle the last assistant message
                let last_assistant = self
                    .state
                    .messages
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, m)| m.role == "assistant")
                    .map(|(i, _)| i);
                if let Some(idx) = last_assistant {
                    self.conversation.toggle_thinking(idx);
                }
            }
            _ => {}
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if !self.input.content.is_empty() && !self.state.is_streaming {
                    let content = self.input.take_content();
                    let _ = self.action_tx.send(Action::SendMessage { content });
                }
            }
            (KeyModifiers::SHIFT, KeyCode::Enter) => {
                self.input.insert_char('\n');
            }
            (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.open_external_editor();
            }
            (_, KeyCode::Char(c)) => self.input.insert_char(c),
            (_, KeyCode::Backspace) => self.input.backspace(),
            (_, KeyCode::Delete) => self.input.delete_char(),
            (_, KeyCode::Left) => self.input.move_left(),
            (_, KeyCode::Right) => self.input.move_right(),
            (_, KeyCode::Home) => self.input.move_home(),
            (_, KeyCode::End) => self.input.move_end(),
            _ => {}
        }
    }

    fn open_external_editor(&mut self) {
        let tmp = std::env::temp_dir().join("tuillem_prompt.md");
        if !self.input.content.is_empty() {
            let _ = std::fs::write(&tmp, &self.input.content);
        } else {
            let _ = std::fs::write(&tmp, "");
        }

        // Suspend terminal and open editor
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);

        let status = std::process::Command::new(&self.editor_command)
            .arg(&tmp)
            .status();

        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen);
        let _ = crossterm::terminal::enable_raw_mode();

        if let Ok(s) = status {
            if s.success() {
                if let Ok(content) = std::fs::read_to_string(&tmp) {
                    let content = content.trim().to_string();
                    if !content.is_empty() {
                        self.input.set_content(content);
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&tmp);
    }
}
```

- [ ] **Step 6: Write lib.rs (TUI run loop)**

`crates/tuillem-tui/src/lib.rs`:
```rust
//! Ratatui TUI layer for tuillem.

pub mod app;
pub mod conversation;
pub mod input;
pub mod sidebar;
pub mod theme;

use app::App;
use crossterm::event::{self, Event as CEvent, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tuillem_core::{Action, Event};

pub async fn run(
    mut app: App,
    mut event_rx: mpsc::UnboundedReceiver<Event>,
    mouse_enabled: bool,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    if mouse_enabled {
        crossterm::execute!(stdout, crossterm::event::EnableMouseCapture)?;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        // Check for core events (non-blocking)
        while let Ok(event) = event_rx.try_recv() {
            app.apply_event(event);
        }

        // Check for terminal events
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CEvent::Key(key) => app.handle_key_event(key),
                CEvent::Mouse(mouse) => app.handle_mouse_event(mouse),
                CEvent::Resize(_, _) => {} // Terminal auto-handles
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    if mouse_enabled {
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::event::DisableMouseCapture
        )?;
    }
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

- [ ] **Step 7: Run TUI tests**

Run: `cargo test -p tuillem-tui`
Expected: all tests pass (theme + input)

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(tui): add TUI layer with sidebar, conversation, input, and theme support"
```

---

## Task 9: Binary Entry Point & Integration

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Wire everything together in main.rs**

`src/main.rs`:
```rust
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to file (not terminal, since we own the screen)
    let log_dir = directories::ProjectDirs::from("", "", "tuillem")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&log_dir)?;
    let log_file = std::fs::File::create(log_dir.join("tuillem.log"))?;
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    info!("tuillem v{} starting", tuillem_config::version());

    // Load config
    let config_path = tuillem_config::Config::default_path();
    let config = if config_path.exists() {
        tuillem_config::Config::from_file(&config_path)?
    } else {
        eprintln!(
            "No config file found at {}. Using defaults.",
            config_path.display()
        );
        eprintln!("Copy config.example.yaml to {} to get started.", config_path.display());
        tuillem_config::Config::from_yaml("{}")?
    };

    // Expand db path
    let db_path = shellexpand::tilde(&config.database.path).to_string();
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open database
    let db = tuillem_db::Db::open(&db_path)?;

    // Initialize providers
    let mut providers: HashMap<String, Box<dyn tuillem_provider::Provider>> = HashMap::new();
    for provider_config in &config.providers {
        match tuillem_provider::create_provider(provider_config) {
            Ok(provider) => {
                providers.insert(provider_config.name.clone(), provider);
            }
            Err(e) => {
                eprintln!("Warning: failed to initialize provider '{}': {}", provider_config.name, e);
            }
        }
    }

    // Initialize plugin host
    let plugin_host = tuillem_plugin::PluginHost::new(config.tools.clone());

    // Determine defaults
    let default_provider = config
        .defaults
        .provider
        .clone()
        .unwrap_or_else(|| config.providers.first().map(|p| p.name.clone()).unwrap_or_default());
    let default_model = config
        .defaults
        .model
        .clone()
        .unwrap_or_else(|| {
            config
                .providers
                .first()
                .and_then(|p| p.default_model.clone().or_else(|| p.models.first().cloned()))
                .unwrap_or_default()
        });

    // Create channels
    let (action_tx, action_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Build theme
    let theme = tuillem_tui::theme::Theme::from_config(&config.theme, &config.themes);

    // Build app state
    let state = tuillem_core::AppState::new(default_provider.clone(), default_model.clone());

    // Build TUI app
    let app = tuillem_tui::app::App::new(state, theme, action_tx.clone(), config.editor.clone());

    // Start coordinator in background
    let coordinator = tuillem_core::Coordinator::new(
        db,
        providers,
        plugin_host,
        default_provider,
        default_model,
        config.defaults.system_prompt.clone(),
    );
    tokio::spawn(async move {
        coordinator.run(action_rx, event_tx).await;
    });

    // Run TUI (blocks until quit)
    tuillem_tui::run(app, event_rx, config.ui.mouse).await?;

    info!("tuillem shutting down");
    Ok(())
}
```

- [ ] **Step 2: Add shellexpand dependency**

Add to workspace `Cargo.toml` under `[dependencies]`:
```toml
shellexpand = "3"
```

- [ ] **Step 3: Verify full build**

Run: `cargo build`
Expected: successful build with no errors

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: wire all crates together in main binary entry point"
```

---

## Task 10: Verify and Polish

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: all tests pass across all crates

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -W clippy::all`
Expected: no errors (warnings acceptable for initial pass)

- [ ] **Step 3: Format code**

Run: `cargo fmt --all`

- [ ] **Step 4: Fix any clippy warnings or test failures**

Address issues found in steps 1-3.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: fix clippy warnings and format code"
```
