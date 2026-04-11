# tuillem

A 3-pane terminal AI chat client with easy connectivity to local and remote LLM endpoints. Switch between providers and models mid-conversation. Full markdown rendering with tables and syntax highlighting. SQLite-backed conversation history with full-text search. Configurable themes, slash commands, and a plugin framework for extensibility.

## Features

- **Multi-provider support** -- Anthropic, OpenAI, OpenRouter, Ollama, LM Studio, and any OpenAI-compatible endpoint
- **Switch models mid-conversation** -- jump between providers and models, with per-session memory of last-used model
- **Terminal markdown rendering** -- headings, bold/italic, code blocks with syntax highlighting, tables with column reflow, lists with wrapped indentation, blockquotes
- **SQLite storage** -- all conversations, thinking blocks, tool calls, and metadata persisted locally with FTS5 full-text search
- **10 built-in themes** -- dark, light, Dracula, Nord, Gruvbox, Tokyo Night, Solarized (dark + light), GitHub Light, Rose Pine Dawn. Custom themes via YAML
- **Colour mode auto-detection** -- graceful degradation from truecolor to 256-colour to basic 16-colour terminals
- **Slash commands** -- `/set model`, `/tag`, `/rename`, `/new <prompt>`, `/export`, `/stats`, `/help` and more
- **Command palette** (Ctrl+K) -- quick access to common actions
- **Settings panel** (Ctrl+S) -- edit all config in-app, changes apply instantly. Open raw YAML in your editor with validation
- **Collapsible sidebar** (Ctrl+L) -- date-grouped conversations with full-text search and live preview
- **External editor** (Ctrl+E) -- compose long prompts in your preferred editor
- **Clipboard copy** (Ctrl+Y / Ctrl+B) -- copy responses or individual code blocks
- **Plugin system** -- external process tools via stdin/stdout JSON protocol
- **Stats for nerds** -- token counts, tokens/sec, context usage percentage
- **First-run wizard** -- interactive setup creates your config on first launch
- **Environment variable expansion** -- `${VAR}` and `${VAR:-default}` in YAML config

## Installation

### Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/seesee/tuillem/releases):

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | `tuillem-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 (aarch64) | `tuillem-aarch64-unknown-linux-gnu.tar.gz` |
| Linux | ARMv7 (Raspberry Pi) | `tuillem-armv7-unknown-linux-gnueabihf.tar.gz` |
| macOS | Intel (x86_64) | `tuillem-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon (M1/M2/M3) | `tuillem-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `tuillem-x86_64-pc-windows-msvc.zip` |

Extract and place the binary in your `$PATH`:

```bash
# Linux / macOS
tar xzf tuillem-*.tar.gz
sudo mv tuillem /usr/local/bin/

# Or without sudo
mkdir -p ~/.local/bin
mv tuillem ~/.local/bin/
```

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

Or create a config file manually:

- **Linux**: `~/.config/tuillem/config.yaml`
- **macOS**: `~/Library/Application Support/com.tuillem.tuillem/config.yaml`

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

### Sidebar

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate sessions (previews conversation) |
| `Enter` | Select session and focus input |
| `/` | Search conversations (title, tags, and content) |
| `d` | Delete session (y/n confirm) |
| `r` | Rename session |
| `PgUp` / `PgDn` | Scroll conversation while browsing |

### Slash commands

Type in the message box:

| Command | Action |
|---------|--------|
| `/help` | Show all commands |
| `/new [prompt]` | New conversation, optionally with an initial message |
| `/set think` / `/set nothink` | Toggle thinking mode |
| `/set model <name>` | Switch model (fuzzy match) |
| `/set provider <name>` | Switch provider |
| `/set system <prompt>` | Set system prompt |
| `/tag <name>` | Tag conversation |
| `/rename <title>` | Rename conversation |
| `/export` | Save transcript to file |
| `/model` | Show current model |
| `/stats` | Show session stats |
| `/clear` | Clear conversation (with confirmation) |

## Configuration

Config location:
- **Linux**: `~/.config/tuillem/config.yaml`
- **macOS**: `~/Library/Application Support/com.tuillem.tuillem/config.yaml`

Database location:
- **Linux**: `~/.local/share/tuillem/tuillem.db`
- **macOS**: `~/Library/Application Support/com.tuillem.tuillem/tuillem.db`

Environment variables are expanded in config values: `${VAR}` or `${VAR:-default}`.

### Colour support

tuillem auto-detects your terminal's colour capability. If colours look wrong (common on some SSH sessions or Raspberry Pi terminals), set the mode explicitly:

```yaml
ui:
  color_mode: "256"    # Options: auto, truecolor, 256, basic
```

See `config.example.yaml` for all available options including themes, layout, scroll settings, and more.

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

- A terminal with 256-colour or truecolor support (most modern terminals)
- A Nerd Font is recommended for rounded message bubbles but not required -- set `ui.nerd_fonts: false` in config
- Rust 1.85+ if building from source

## License

MIT
