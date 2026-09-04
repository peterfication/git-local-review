import "Justfile.ci.just"
import "Justfile.db.just"

default:
  just --list

# Run the application
run:
  cargo run

# Run the application and print the version
run-version:
  cargo run -- --version

# Run the repo setup
setup: git-hooks-install run-version

# Install git hooks using Lefthook
git-hooks-install:
  lefthook install
