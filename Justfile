default:
  just --list

# Run all steps of CI
ci: db-prepare run-version cli-help-dump db-schema-dump format lint test build doc

# Run the application
run:
  cargo run

# Run the application and print the version
run-version:
  cargo run -- --version

# Dump the CLI help to a file
cli-help-dump:
  cargo run -- --help > cli_help.txt

# Format all files
format: format-rust format-rest

# Format the code with rustfmt
format-rust:
  cargo fmt --all

# Format all other files with dprint
format-rest:
  dprint fmt

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

# Generate documentation and open it in the browser
doc-open:
  just doc && open target/doc/git_local_review/index.html

# Build the application
build:
  cargo build

# Install git hooks using Lefthook
git-hooks-install:
  lefthook install

# Install the database migrations tool
db-cli-install:
  cargo install sqlx-cli

# Create a new database migration
db-migration-generate NAME:
  sqlx migrate add {{NAME}}

# Run database migrations
db-migrate:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx migrate run

# Revert the last database migration
db-revert:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx migrate revert

# Create the database
db-create:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx database create

# Drop the database
db-drop:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx database drop -y

# Write sqlx query data to the .sqlx folder
db-prepare:
  DATABASE_URL="sqlite:./tmp/reviews.db" cargo sqlx prepare

# Reset the database by dropping, creating, and migrating
db-reset:
  just db-drop
  just db-create
  just db-migrate

# Setup the database by creating and migrating it
db-setup:
  just db-create
  just db-migrate

# Dump the current database schema to a file
db-schema-dump:
  sqlite3 tmp/reviews.db .schema > schema.sql
