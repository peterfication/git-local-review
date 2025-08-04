# Architecture

- **View Stack System**
- **Event-Driven Design**: Async event processing

## Core Components

- **`src/main.rs`:** Application entry point with terminal initialization.
- **`src/app.rs`:** Main application state and view stack management.
- **`src/event.rs`:** Event system with async handling (Tick, Crossterm, App events). Event names are defined here.
- **`src/event_handler.rs`:** Event processing logic.
- **`src/ui.rs`:** Ratatui rendering implementation.

## Views & UI

- **`src/views/mod.rs`:** View system with `ViewHandler` trait and `ViewType` enum.
- **`src/views/main.rs`:** Main review listing view.
- **`src/views/review_create.rs`:** Modal review creation dialog.
- ...

## Data & Services

- **`src/database.rs`:** Database connection and management (SQLite).
- **`src/models`**: Entities.
- **`src/services`**: Business logic for the application.
- **`src/services/mod.rs`**: ServiceHandler for services to handle events.
