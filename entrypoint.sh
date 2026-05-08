#!/bin/bash
set -e

# Create a directory for Postgres data
export PGDATA=/app/data/postgres

# Initialize database if not already done
if [ ! -f "$PGDATA/PG_VERSION" ]; then
    echo "Initializing Postgres data directory..."
    initdb -D $PGDATA
fi

# Start Postgres in the background
# We use -k /tmp to ensure the socket is created in a writable directory
echo "Starting Postgres..."
postgres -D $PGDATA -p 5432 -k /tmp &

# Wait for Postgres
until pg_isready -h /tmp; do
  echo "Waiting for database..."
  sleep 2
done

echo "Setting up database..."
# Tell createdb to use the socket in /tmp
createdb -h /tmp mockbank || true

echo "Starting Mock Bank..."
mock-bank &

echo "Starting Gateway..."
gateway &

echo "Starting Nginx..."
# We use -g to override the PID path, but we first need to ensure the main config doesn't conflict
nginx -g "daemon off;"
