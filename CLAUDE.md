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
