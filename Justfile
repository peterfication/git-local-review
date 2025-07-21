default:
  just --list

# Run all steps of CI
ci: format lint test build doc

# Run the application
run:
  cargo run

# Format the code with rustfmt
format:
  cargo fmt --all

# Run clippy linter
lint:
  cargo clippy --all-targets --all-features -- -D warnings

# Run tests
test:
  cargo nextest run

# Run tests with coverage
test-coverage:
  cargo tarpaulin --out Html && open tarpaulin-report.html

# Open the coverage report in the browser
test-coverage-open:
  open tarpaulin-report.html

# Review test snapshots
test-snapshot-review:
  cargo insta review

# Generate documentation
doc:
  cargo doc --no-deps --all-features

# Build the application
build:
  cargo build

# Install git hooks using Lefthook
git-hooks-install:
  lefthook install
