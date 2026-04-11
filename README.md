# tuillem

A 3-pane terminal AI chat client with easy connectivity to local and remote LLM endpoints. Switch between providers and models mid-conversation. Full markdown rendering with tables and syntax highlighting. SQLite-backed conversation history with full-text search. Configurable themes, adaptive scroll, slash commands, and a plugin framework for extensibility.

## Features

- **Multi-provider support** -- Anthropic, OpenAI, OpenRouter, Ollama, LM Studio, and any OpenAI-compatible endpoint
- **Switch models mid-conversation** -- jump between providers and models, with per-session memory of last-used model
- **Terminal markdown rendering** -- headings, bold/italic, code blocks with syntax highlighting, tables with column reflow, lists with wrapped indentation, blockquotes
- **SQLite storage** -- all conversations, thinking blocks, tool calls, and metadata persisted locally with FTS5 full-text search
- **10 built-in themes** -- dark, light, Dracula, Nord, Gruvbox, Tokyo Night, Solarized (dark + light), GitHub Light, Rose Pine Dawn. Custom themes via YAML
- **Slash commands** -- `/set model`, `/tag`, `/rename`, `/export`, `/stats`, `/help` and more
- **Reading scroll** -- viewport freezes after one screenful of response; Enter advances with line highlighting
- **Command palette** (Ctrl+K) -- quick access to common actions
- **Settings panel** (Ctrl+S) -- edit all config in-app, changes apply instantly
- **Collapsible sidebar** (Ctrl+L) -- date-grouped conversations with search and preview
- **External editor** (Ctrl+E) -- compose long prompts in your preferred editor
- **Clipboard copy** (Ctrl+Y / Ctrl+B) -- copy responses or individual code blocks
- **Plugin system** -- external process tools via stdin/stdout JSON protocol
- **Stats for nerds** -- token counts, tokens/sec, context usage
- **First-run wizard** -- interactive setup creates your config on first launch
- **Environment variable expansion** -- `${VAR}` and `${VAR:-default}` in YAML config

## Installation

### From crates.io

```bash
cargo install tuillem
```

### From source

```bash
git clone https://github.com/seesee/tuillem.git
cd tuillem
cargo install --path .
```

## Quick start

On first run with no config file, tuillem launches an interactive setup wizard:

```
Welcome to tuillem! Let's set up your configuration.

Step 1/5: Provider Setup
  1. Anthropic (Claude)
  2. OpenAI
  3. OpenRouter
  4. Ollama (local)
  5. LM Studio / OpenAI-compatible (local)
Choose [1-5]:
```

Or create `~/.config/tuillem/config.yaml` manually (macOS: `~/Library/Application Support/com.tuillem.tuillem/config.yaml`):

```yaml
providers:
  - name: anthropic
    provider_type: anthropic
    api_key: "${ANTHROPIC_API_KEY}"
    default_model: claude-sonnet-4-20250514
    models:
      - claude-sonnet-4-20250514
      - claude-opus-4-0520

  - name: ollama
    provider_type: ollama
    base_url: http://localhost:11434
    models:
      - llama3
      - mistral

  - name: lmstudio
    provider_type: openai
    api_key: "lm-studio"
    base_url: http://localhost:1234/v1
    models:
      - my-local-model

defaults:
  provider: anthropic
  model: claude-sonnet-4-20250514
```

Then run:

```bash
tuillem
```

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle focus: input / sidebar / conversation |
| `Ctrl+N` | New conversation |
| `Ctrl+O` | Switch model |
| `Ctrl+T` | Switch provider |
| `Ctrl+K` | Command palette |
| `Ctrl+S` | Settings |
| `Ctrl+H` | Help overlay |
| `Ctrl+L` | Toggle sidebar |
| `Ctrl+E` | External editor |
| `Ctrl+Y` | Copy last response |
| `Ctrl+B` | Copy code blocks |
| `Ctrl+R` | Regenerate response |
| `Ctrl+C` | Quit |
| `Enter` (empty) | Scroll conversation forward |
| `PgUp` / `PgDn` | Scroll conversation (works from any pane) |
| `Esc` | Cancel streaming / close overlay / return to input |

### Sidebar (when focused)

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate sessions (previews conversation) |
| `Enter` | Select session and focus input |
| `/` | Search conversations (title, tags, and content) |
| `d` | Delete session (y/n confirm) |
| `r` | Rename session |

### Slash commands

Type in the message box:

| Command | Action |
|---------|--------|
| `/help` | Show all commands |
| `/set think` / `/set nothink` | Toggle thinking mode |
| `/set model <name>` | Switch model (fuzzy match) |
| `/set provider <name>` | Switch provider |
| `/set system <prompt>` | Set system prompt |
| `/tag <name>` | Tag conversation |
| `/rename <title>` | Rename conversation |
| `/new` | New conversation |
| `/export` | Save transcript to file |
| `/model` | Show current model |
| `/stats` | Show session stats |

## Configuration

Config location:
- **Linux**: `~/.config/tuillem/config.yaml`
- **macOS**: `~/Library/Application Support/com.tuillem.tuillem/config.yaml`

Database location:
- **Linux**: `~/.local/share/tuillem/tuillem.db`
- **macOS**: `~/Library/Application Support/com.tuillem.tuillem/tuillem.db`

Environment variables are expanded in config values: `${VAR}` or `${VAR:-default}`.

See `config.example.yaml` for all available options.

## Architecture

tuillem is a Cargo workspace with 7 focused crates:

| Crate | Purpose |
|-------|---------|
| `tuillem-config` | YAML config parsing with validation |
| `tuillem-db` | SQLite storage with FTS5 search |
| `tuillem-markdown` | Terminal markdown rendering (comrak + syntect) |
| `tuillem-provider` | LLM provider abstraction with SSE streaming |
| `tuillem-plugin` | External process tool execution |
| `tuillem-core` | Async coordinator, state management, action routing |
| `tuillem-tui` | Ratatui UI with themes, overlays, and widgets |

Communication between layers uses typed `tokio::mpsc` channels. The coordinator runs on a dedicated thread (SQLite is `!Sync`).

## Requirements

- Rust 2024 edition (1.85+)
- A terminal with 256-color or truecolor support
- A Nerd Font is recommended (for rounded message bubbles) but not required -- set `ui.nerd_fonts: false` in config

## License

MIT
