# git-local-review

A Terminal User Interface (TUI) application for reviewing Git changes with local SQLite state storage.

## Features

- **Review Management**: Create and list local Git reviews with persistent storage
- **Modern TUI**
- **Local Storage**: SQLite database for managing review state

## Key Bindings

| View                   | Key                       | Action                                                 |
| ---------------------- | ------------------------- | ------------------------------------------------------ |
| **Global**             | `?`                       | Show help modal for current view                       |
| **Main**               | `n`                       | Create new review                                      |
| **Main**               | `Up` / `Down` / `k` / `j` | Change review selection                                |
| **Main**               | `o` / `Space` / `Enter`   | Open selected review                                   |
| **Main**               | `d`                       | Delete selected review                                 |
| **Main**               | `q` / `Ctrl+C`            | Quit application                                       |
| **Review create**      | `Up` / `Down` / `k` / `j` | Change branch selection                                |
| **Review create**      | `Tab`                     | Switch between target and base branch selection        |
| **Review create**      | `Enter`                   | Submit review                                          |
| **Review create**      | `Esc`                     | Cancel and close popup                                 |
| **Review details**     | `Up` / `Down` / `k` / `j` | Change file or line selection                          |
| **Review details**     | `Enter`                   | Switch between files lists and content box             |
| **Review details**     | `Space`                   | When in files list, toggle file viewed                 |
| **Review details**     | `c`                       | Open comments view for currently selected file or line |
| **Review details**     | `Esc`                     | Close review details / go back to main view            |
| **ConfirmationDialog** | `y` / `Y` / `Enter`       | Confirm                                                |
| **ConfirmationDialog** | `n` / `N` / `Esc`         | Cancel                                                 |
| **Help Modal**         | `Up` / `Down` / `k` / `j` | Navigate keybindings                                   |
| **Help Modal**         | `Enter`                   | Execute selected action                                |
| **Help Modal**         | `Esc`                     | Close help modal                                       |

## Installation

```bash
cargo install --git https://github.com/peterfication/git-local-review
git-local-review
```

## Architecture

- **View Stack System**
- **Event-Driven Design**: Async event processing

### Core Components

- **`src/main.rs`:** Application entry point with terminal initialization.
- **`src/app.rs`:** Main application state and view stack management.
- **`src/event.rs`:** Event system with async handling (Tick, Crossterm, App events). Event names are defined here.
- **`src/event_handler.rs`:** Event processing logic.
- **`src/ui.rs`:** Ratatui rendering implementation.

### Views & UI

- **`src/views/mod.rs`:** View system with `ViewHandler` trait and `ViewType` enum.
- **`src/views/main.rs`:** Main review listing view.
- **`src/views/review_create.rs`:** Modal review creation dialog.

### Data & Services

- **`src/database.rs`:** Database connection and management (SQLite)
- **`src/models`**: Entities
- **`src/services`**: Business logic for

## Development

### Prerequisites

- Rust version >= `1.88.0`
- [just](https://github.com/casey/just) task runner

### Quick Start

```bash
# Clone the repository
git clone git@github.com:peterfication/git-local-review.git
cd git-local-review

# Run the steps from the CI pipeline
just ci

# Run the application
just run

# Or use cargo directly
cargo run
```

### Available Commands

All development tasks are managed via `just`:

```bash
just run      # Run the application
just test     # Run all tests
just lint     # Run clippy linting
just format   # Format code with rustfmt
just build    # Build the project
just doc      # Generate documentation
just ci       # Run full CI pipeline (format, lint, test, build, doc)
```

### Disclaimer regarding AI / Coding LLMs

I'm still pretty new to Rust. This project is made possible through the usage of coding LLMs. However, the coding LLMs are tightly managed and directed and all generated code is reviewed thoroughly and adapted where needed.

## License

This project is licensed under the MIT license ([LICENSE](LICENSE) or [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))
