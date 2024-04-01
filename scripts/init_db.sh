#!/usr/bin/env bash
set -x
set -eo pipefail

# Checks so we don't accidently run script and leave system half broken
if ! [ -x "$(command -v psql)" ]; then
  echo >&2 "Error: psql is not installed."
  exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed"
  echo >&2 "Try: cargo install sqlx-cli --no-default-features --features rustls,postgres"
  exit 1
fi

# Check if custom user is set or default to 'postgres'
DB_USER="${POSTGRES_USER:=postgres}"
# Check if custom password is set or default to 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_PORT="${POSTGRES_PORT:=5432}"
DB_HOST="${POSTGRES_HOST:=localhost}"

# Launch postgres using Docker
docker run \
  -e POSTGRES_USER=${DB_USER} \
  -e POSTGRES_PASSWORD=${DB_PASSWORD} \
  -e POSTGRES_DB=${DB_NAME} \
  -p "${DB_PORT}":5432 \
  -d postgres \
  postgres -N 1000
  # ^ Increase max number of connections for testing purposes

# Do not proceed until database is up (Race Conditions)
# Ping Postgress untils it is ready to accept commands
export PGPASSWORD="${DB_PASSWORD}"
until psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
  >&2 echo "Postgres is not yet available - sleeping"
  sleep 1
done

>&2 echo "Postgress is up and running on port ${DB_PORT}!"

DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL
# SQLX relies on this `DATABASE_URL` env variable.
sqlx database create
