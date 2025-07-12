# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based Terminal User Interface (TUI) application for reviewing Git changes with local SQLite state storage. It's built using the Ratatui framework with an async event-driven architecture.

## Commands

All commands are available via `just` (task runner):

- **Run the application**: `just run` or `cargo run`
- **Format code**: `just format` (runs `cargo fmt --all`)
- **Lint code**: `just lint` (runs `cargo clippy --all-targets --all-features -- -D warnings`)
- **Run tests**: `just test` (runs `cargo test --locked`)
- **Build**: `just build` (runs `cargo build`)
- **Generate docs**: `just doc` (runs `cargo doc --no-deps --all-features`)
- **Run full CI pipeline**: `just ci` (format, lint, test, build, doc)

## Architecture

### Core Components

- **main.rs**: Entry point that initializes logging, terminal, and runs the main app
- **app.rs**: Contains the main `App` struct with application state and event handling logic
- **event.rs**: Event system with async event handling using tokio channels
  - `Event` enum for Tick, Crossterm, and App events
  - `EventHandler` for managing event channels
  - `EventTask` for background event processing at 30 FPS
- **ui.rs**: Ratatui widget implementation for rendering the TUI
- **logging.rs**: Logging setup using tui-logger with file output to `tmp/app.log`
- **models/**: Data models (currently contains empty `review.rs`)

### Event-Driven Architecture

The application uses an async event-driven pattern:
1. Events are processed through an `EventHandler` with unbounded channels
2. Three event types: Tick (30 FPS), Crossterm (terminal input), App (custom events)
3. Main loop renders UI and processes events asynchronously
4. Key bindings: Esc/q/Ctrl-C to quit, arrow keys for counter increment/decrement

### Dependencies

- **ratatui**: TUI framework for terminal interfaces
- **tokio**: Async runtime with full features
- **crossterm**: Cross-platform terminal manipulation
- **color-eyre**: Error handling and reporting
- **tui-logger**: Logging integration for TUI apps
- **log**: Standard logging facade

### Development Notes

- Application currently implements a basic counter example
- Review functionality appears to be planned but not yet implemented
- Logging outputs to `tmp/app.log` file
- Uses Rust 2024 edition