# Install the database migrations tool
[group('db')]
db-cli-install:
  cargo install sqlx-cli

# Create a new database migration
[group('db')]
db-migration-generate NAME:
  sqlx migrate add {{NAME}}

# Run database migrations
[group('db')]
db-migrate:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx migrate run

# Revert the last database migration
[group('db')]
db-revert:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx migrate revert

# Create the database
[group('db')]
db-create:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx database create

# Drop the database
[group('db')]
db-drop:
  DATABASE_URL="sqlite:./tmp/reviews.db" sqlx database drop -y

# Write sqlx query data to the .sqlx folder
[group('db')]
db-prepare:
  DATABASE_URL="sqlite:./tmp/reviews.db" cargo sqlx prepare

# Reset the database by dropping, creating, and migrating
[group('db')]
db-reset:
  just db-drop
  just db-create
  just db-migrate

# Setup the database by creating and migrating it
[group('db')]
db-setup:
  just db-create
  just db-migrate

# Dump the current database schema to a file
[group('db')]
db-schema-dump:
  sqlite3 tmp/reviews.db .schema > schema.sql
