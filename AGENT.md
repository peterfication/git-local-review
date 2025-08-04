# AGENT.md

This file provides guidance to coding agents (e.g. Claude Code (claude.ai/code)) when working with code in this repository.

## Documentation

- **[`README.md`](README.md)**: User-facing documentation with installation, usage and development instructions
- **[`ARCHITECTURE.md`](ARCHITECTURE.md)**: Notes about the software architecture of this application
- **[`KEYBINDINGS.md`](KEYBINDINGS.md)**: Documentation about the keybindings of the application

Make sure to keep the documentation files up-to-date when making changes.

## Project Overview

This is a Rust-based Terminal User Interface (TUI) application for reviewing Git changes with local SQLite state storage. Built using Ratatui framework, it features a modern async event-driven architecture with Arc-optimized event sharing, comprehensive testing, and modern Rust 2024 patterns.

## Commands

All commands are available via `just` (task runner). See README.md for user-focused quick start instructions.

Some examples:

- **Run the application**: `just run` or `cargo run`
- **Format code**: `just format` (runs `cargo fmt --all`)
- **Lint code**: `just lint` (runs `cargo clippy --all-targets --all-features -- -D warnings`)
- **Run tests**: `just test` (runs `cargo test --locked`)
- **Build**: `just build` (runs `cargo build`)
- **Generate docs**: `just doc` (runs `cargo doc --no-deps --all-features`)
- **Run full CI pipeline**: `just ci` (format, lint, test, build, doc)

After a changeset, run `just ci` to verify the changes and fix occurring errors so that the code quality standards are upheld.

## Architecture

### Core Components

- **main.rs**: Entry point that initializes logging, terminal, and runs the main app
- **app.rs**: Contains the main `App` struct with view stack management and core application state
- **event.rs**: Arc-optimized event system with async event handling using tokio channels
  - `Event` enum for Tick (30 FPS), Crossterm (terminal input), and App (custom events)
  - `EventHandler` for managing `Arc<Event>` channels with test-friendly extensions
  - `EventTask` for background event processing with efficient event sharing
  - `AppEvent` enum with Arc-wrapped data payloads (e.g., `ReviewCreateSubmit(Arc<ReviewCreateData>)`)
- **event_handler.rs**: **Event processing logic with EventProcessor** - handles all event routing and business logic
- **ui.rs**: Ratatui widget implementation for rendering the TUI
- **database.rs**: SQLite database connection and management with connection pooling
- **logging.rs**: Logging setup using tui-logger with file output to `tmp/app.log`

### Arc-Optimized Event System

**Event architecture with clone free optimizations:**

- **Arc-Wrapped Events**: All events use `Arc<Event>` for efficient sharing without cloning
- **Event Channels**: `mpsc::UnboundedSender<Arc<Event>>` and `mpsc::UnboundedReceiver<Arc<Event>>`
- **Data Payload Optimization**: Event data uses Arc for efficient sharing:
  - `AppEvent::ReviewCreateSubmit(Arc<ReviewCreateData>)`
  - `AppEvent::ReviewDeleteConfirm(Arc<str>)` and `AppEvent::ReviewDelete(Arc<str>)`
  - `AppEvent::ReviewCreatedError(Arc<str>)` and `AppEvent::ReviewDeletedError(Arc<str>)`
- **ReviewsLoadingState**: Uses `Arc<[Review]>` instead of `Vec<Review>` for memory-efficient sharing
- **EventProcessor**: Central event routing with three-phase processing:
  1. Services handle events first (business logic)
  2. Views handle events second (UI updates)
  3. Global handlers last (app-level state changes)

### View System Architecture

**Modern trait-based view management with stack system:**

- **views/mod.rs**: Core view system with `ViewHandler` trait and `ViewType` enum for type-safe view management
- **views/main_view.rs**: Main review listing view with optimized Arc handling for review data
- **views/review_create_view.rs**: Modal review creation dialog with text input state management
- **views/confirmation_dialog.rs**: Reusable confirmation dialog for destructive operations

**View Stack Management:**

- `App.view_stack: Vec<Box<dyn ViewHandler>>` - Dynamic view stack using trait objects
- Only the top view receives key events (proper modal behavior)
- All views receive app events (for state synchronization)
- Views handle their own state (e.g., `ReviewCreateView.title_input`, `MainView.selected_review_index`)
- Type-safe view identification with `ViewType` enum (Main, ReviewCreate, ConfirmationDialog)

### Service Layer Architecture

**Business logic separated from UI concerns:**

- **services/review_service.rs**: Business logic for review operations (create, list, delete, validation)
- **models/review.rs**: Review entity with SQLite persistence, migrations, and CRUD operations
- **ServiceHandler trait**: Async trait for handling app events at the business logic layer
- Clean separation: Views → Events → EventProcessor → Services → Models → Database

### Event-Driven Architecture Flow

**Complete separation of concerns with async event processing:**

1. **Event Flow**: Views send events → EventProcessor routes → Services handle business logic
2. **Event Types**:
   - `Tick`: 30 FPS rendering updates
   - `Crossterm`: Terminal input (keys, mouse, etc.)
   - `App`: Custom application events with Arc-wrapped data payloads
3. **Key App Events**:
   - `Quit`: Application shutdown
   - `ViewClose`: Close current view modal
   - `ReviewCreateOpen`: Open review creation modal
   - `ReviewCreateSubmit(Arc<ReviewCreateData>)`: Submit review with data payload
   - `ReviewsLoadingState(ReviewsLoadingState)`: Propagate review loading state changes
   - `ReviewDeleteConfirm(Arc<str>)`: Open delete confirmation dialog
   - `ReviewDelete(Arc<str>)`: Execute review deletion

### Database Schema (SQLite)

See [schema.sql](schema.sql).

### Performance Optimizations

**Arc-based clone free optimizations throughout:**

- **ReviewsLoadingState::Loaded(Arc<[Review]>)**: Enables multiple views to share review data without cloning
- **MainView Arc handling**: `self.reviews = Arc::clone(reviews)` for O(1) state updates
- **Event sharing**: `Arc<Event>` eliminates expensive event cloning across the event system
- **String optimization**: Uses `Arc<str>` for error messages and IDs to avoid string cloning
- **Efficient pattern matching**: Views only clone data when they actually handle specific events

### Dependencies

See [Cargo.toml](Cargo.toml)

Key dependencies:

- **ratatui**: TUI framework for terminal interfaces
- **tokio**: Async runtime for event handling and I/O
- **sqlx**: Async SQLite database operations
- **color-eyre**: Enhanced error handling and backtraces
- **uuid**: UUID generation for review IDs
- **serde**: Serialization support

### Testing Strategy

**Comprehensive test suite with 42+ tests covering all components:**

#### Test Categories:

- **Unit Tests**: Models, services, and business logic (isolated)
- **Integration Tests**: Event processing and view interactions
- **UI Tests**: View behavior with event verification and state inspection
- **Database Tests**: CRUD operations with in-memory SQLite
- **Arc Optimization Tests**: Verification of efficient memory usage patterns

#### Key Testing Features:

- **Event Verification**: Tests can inspect published events using `EventHandler.try_recv()`
- **View State Inspection**: Test-only `debug_state()` method for checking view internals
- **Modal Behavior Testing**: Verification that only top view receives key events
- **Database Isolation**: All tests use in-memory SQLite (`"sqlite::memory:"`)
- **Mock Event Handlers**: `EventHandler::new_for_test()` for controlled testing
- **Arc Testing**: Verification of Arc clone patterns and memory efficiency
- **Snapshot Testing**: Uses `insta` crate for UI rendering regression tests

#### Test Examples:

- Event publishing verification (e.g., key 'q' sends `AppEvent::Quit`)
- View type assertions (e.g., `view.view_type() == ViewType::ReviewCreate`)
- State mutation testing (e.g., `debug_state()` shows `"title_input: \"test\""`)
- Modal routing validation (only top view processes keys)
- Arc efficiency testing (minimal cloning in event handlers)

### Architecture Patterns

- **Arc-Optimized Event Driven Architecture**: All state changes flow through Arc-wrapped events
- **Trait Objects**: `Box<dyn ViewHandler>` for dynamic view management
- **Service Layer Pattern**: Business logic separated from UI concerns
- **Repository Pattern**: Database operations abstracted through models
- **Modal View Stack**: Proper modal behavior with view hierarchy
- **Dependency Injection**: Database and services cleanly abstracted
- **Test-Driven Development**: Comprehensive test coverage with mocking strategies
- **Time Provider Pattern**: Dependency injection for testable timestamps
- **Event Broadcasting**: All views receive app events for state synchronization

### Memory Management & Performance

**Modern Rust patterns with clone free optimizations:**

- **Arc for Shared State**: Efficient sharing without unsafe code or expensive cloning
- **Move Semantics**: Careful ownership transfer in event system
- **Clone-Free Event Data Sharing**: Arc eliminates unnecessary event clones
- **Async Channels**: Non-blocking event communication with efficient buffering
- **Pattern-Based Optimization**: Views only clone data when actually handling events
- **String Optimization**: `Arc<str>` for shared string data (error messages, IDs)

### Development Notes

- **Arc Optimizations**: Recent performance improvements eliminate unnecessary cloning
- **Type Safety**: `ViewType` enum prevents view type errors at compile time
- **Event Testing**: Full event flow can be verified in tests
- **Clean Architecture**: Clear separation between UI, business logic, and data layers
- **Modal System**: Proper view stack management for overlays and dialogs
- **Async Throughout**: Tokio runtime used consistently for all I/O operations
- **Error Handling**: `color-eyre` provides detailed error context and backtraces
- **Database Migrations**: Schema changes handled automatically on startup
- **Logging**: Structured logging to `tmp/app.log` with TUI integration
- **Uses Rust 2024 edition** with modern language features
- **Memory Efficiency**: Arc-based optimizations minimize allocation and improve performance

### Code Quality Standards

- **Clippy Clean**: All code passes `clippy` with warnings denied
- **Formatted**: All code formatted with `rustfmt`
- **Tested**: 95%+ test coverage on core business logic with Arc optimization verification
- **Documented**: Public APIs have comprehensive documentation
- **Type Safe**: Extensive use of strong types and enums for correctness
- **Performance Focused**: Arc optimizations for efficient memory usage
