# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Documentation

- **[`README.md`](README.md)**: User-facing documentation with installation, usage and development instructions
- **`CLAUDE.md**: This file - technical architecture and development guidance for Claude Code

Make sure to keep the documentation files up-to-date when making changes.

## Project Overview

This is a Rust-based Terminal User Interface (TUI) application for reviewing Git changes with local SQLite state storage. It's built using the Ratatui framework with a async event-driven architecture featuring clean separation of concerns, comprehensive testing, and modern Rust patterns.

## Commands

All commands are available via `just` (task runner). See README.md for user-focused quick start instructions.

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
- **app.rs**: Contains the main `App` struct with view stack management and core application state
- **event.rs**: Event system with async event handling using tokio channels
  - `Event` enum for Tick (30 FPS), Crossterm (terminal input), and App (custom events)
  - `EventHandler` for managing event channels with test-friendly extensions
  - `EventTask` for background event processing
  - `AppEvent` enum with parameterized events (e.g., `ReviewCreateSubmit(ReviewCreateData)`)
- **event_handler.rs**: **Event processing logic extracted from App** - handles all event routing and business logic
- **ui.rs**: Ratatui widget implementation for rendering the TUI
- **database.rs**: SQLite database connection and management with connection pooling
- **logging.rs**: Logging setup using tui-logger with file output to `tmp/app.log`

### View System Architecture

**Modern trait-based view management with stack system:**

- **views/mod.rs**: Core view system with `ViewHandler` trait and `ViewType` enum for type-safe view management
- **views/main.rs**: Main review listing view with key bindings (n=create, q/Esc/Ctrl+C=quit)
- **views/review_create.rs**: Modal review creation dialog with text input state management

**View Stack Management:**
- `App.view_stack: Vec<Box<dyn ViewHandler>>` - Dynamic view stack using trait objects
- Only the top view receives key events (proper modal behavior)
- Views handle their own state (e.g., `ReviewCreateView.title_input`)
- Type-safe view identification with `ViewType` enum (Main, ReviewCreate)

### Service Layer Architecture

**Business logic separated from UI concerns:**

- **services/review_service.rs**: Business logic for review operations (create, list, validation)
- **models/review.rs**: Review entity with SQLite persistence, migrations, and CRUD operations
- Clean separation: Views → Events → EventHandler → Services → Models → Database

### Event-Driven Architecture

**Sophisticated async event system with complete separation of concerns:**

1. **Event Flow**: Views send events → EventHandler processes → Services handle business logic
2. **Event Types**:
   - `Tick`: 30 FPS rendering updates
   - `Crossterm`: Terminal input (keys, mouse, etc.)
   - `App`: Custom application events with data payloads
3. **App Events**:
   - `Quit`: Application shutdown
   - `ReviewCreateOpen`: Open review creation modal
   - `ReviewCreateClose`: Close review creation modal
   - `ReviewCreateSubmit(ReviewCreateData)`: Submit review with data payload

### Database Schema

SQLite database with embedded migrations:

```sql
CREATE TABLE reviews (
    id TEXT PRIMARY KEY,      -- UUID v4
    title TEXT NOT NULL,      -- User-provided title
    created_at TEXT NOT NULL  -- ISO 8601 timestamp
);
```

### Dependencies

See [Cargo.toml](Cargo.toml)

### Testing Strategy

**Comprehensive test suite with 42+ tests covering all components:**

#### Test Categories:
- **Unit Tests**: Models, services, and business logic (isolated)
- **Integration Tests**: Event processing and view interactions
- **UI Tests**: View behavior with event verification and state inspection
- **Database Tests**: CRUD operations with in-memory SQLite

#### Key Testing Features:
- **Event Verification**: Tests can inspect published events using `EventHandler.try_recv()`
- **View State Inspection**: Test-only `debug_state()` method for checking view internals
- **Modal Behavior Testing**: Verification that only top view receives key events
- **Database Isolation**: All tests use in-memory SQLite (`"sqlite::memory:"`)
- **Mock Event Handlers**: `EventHandler::new_for_test()` for controlled testing

#### Test Examples:
- Event publishing verification (e.g., key 'q' sends `AppEvent::Quit`)
- View type assertions (e.g., `view.view_type() == ViewType::ReviewCreate`)
- State mutation testing (e.g., `debug_state()` shows `"title_input: \"test\""`)
- Modal routing validation (only top view processes keys)

### Architecture Patterns

- **Trait Objects**: `Box<dyn ViewHandler>` for dynamic view management
- **Event Sourcing**: All state changes flow through events
- **Service Layer**: Business logic separated from UI concerns
- **Repository Pattern**: Database operations abstracted through models
- **Modal View Stack**: Proper modal behavior with view hierarchy
- **Dependency Injection**: Database and services cleanly abstracted
- **Test-Driven Development**: Comprehensive test coverage with mocking strategies

### Development Notes

- **Tests**: All major components have comprehensive test coverage
- **Type Safety**: `ViewType` enum prevents view type errors at compile time
- **Event Testing**: Full event flow can be verified in tests
- **Clean Architecture**: Clear separation between UI, business logic, and data layers
- **Modal System**: Proper view stack management for overlays and dialogs
- **Async Throughout**: Tokio runtime used consistently for all I/O operations
- **Error Handling**: `color-eyre` provides detailed error context and backtraces
- **Database Migrations**: Schema changes handled automatically on startup
- **Logging**: Structured logging to `tmp/app.log` with TUI integration
- **Uses Rust 2024 edition** with modern language features

### Code Quality Standards

- **Clippy Clean**: All code passes `clippy` with warnings denied
- **Formatted**: All code formatted with `rustfmt`
- **Tested**: Minimum 95% test coverage on core business logic
- **Documented**: Public APIs have comprehensive documentation
- **Type Safe**: Extensive use of strong types and enums for correctness

After a changeset, run `just ci` to verify the changes and fix occurring errors so that the code quality standards are upheld.
