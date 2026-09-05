# Run all steps of CI
ci: db-prepare run-version cli-help-dump db-schema-dump format lint test build doc security-scan

# Format all files
[group('ci')]
format: format-rust format-rest format-sql

# Format the code with rustfmt
[group('ci')]
format-rust:
  cargo fmt --all

# Format all other files with dprint
[group('ci')]
format-rest:
  dprint fmt

# Format SQL migration files with SQLFluff
[group('ci')]
format-sql:
  uvx sqlfluff fix migrations

# Check SQL migration formatting with SQLFluff
[group('ci')]
format-sql-check:
  uvx sqlfluff lint migrations

# Run clippy linter
[group('ci')]
lint:
  cargo clippy --all-targets --all-features -- -D warnings

# Run tests
[group('ci')]
test:
  cargo nextest run

# Run tests with coverage
[group('ci')]
test-coverage:
  cargo tarpaulin --out Html && open tarpaulin-report.html

# Open the coverage report in the browser
[group('ci')]
test-coverage-open:
  open tarpaulin-report.html

# Review test snapshots
[group('ci')]
test-snapshot-review:
  cargo insta review

# Generate documentation
[group('ci')]
doc:
  cargo doc --no-deps --all-features

# Generate documentation and open it in the browser
[group('ci')]
doc-open:
  just doc && open target/doc/git_local_review/index.html

# Build the application
[group('ci')]
build:
  cargo build

# Dump the CLI help to a file
[group('ci')]
cli-help-dump:
  cargo run -- --help > cli_help.txt

# Run the security scan using Trivy
[group('ci')]
security-scan:
  trivy fs --config .trivy.yaml Cargo.lock
