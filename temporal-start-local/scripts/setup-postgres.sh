#!/bin/sh
set -eu

: "${POSTGRES_SEEDS:?POSTGRES_SEEDS is required}"
: "${POSTGRES_USER:?POSTGRES_USER is required}"
: "${SQL_PASSWORD:?SQL_PASSWORD is required}"

# PostgreSQL belongs to another Compose project, so wait for its port here.
attempt=1
until nc -z -w 2 "$POSTGRES_SEEDS" "${DB_PORT:-5432}"; do
  if [ "$attempt" -ge 30 ]; then
    echo "Existing PostgreSQL is unavailable." >&2
    exit 1
  fi
  attempt=$((attempt + 1))
  sleep 2
done

sql_tool() {
  temporal-sql-tool --plugin postgres12 --ep "$POSTGRES_SEEDS" \
    -p "${DB_PORT:-5432}" -u "$POSTGRES_USER" "$@"
}

# Prefer existing databases so their owner need not have CREATEDB privileges.
# On a fresh setup, an account with CREATEDB can also create them here.
for database in temporal temporal_visibility; do
  echo "Preparing database: $database"
  if ! sql_tool --db "$database" setup-schema -v 0.0; then
    sql_tool --db "$database" create
    sql_tool --db "$database" setup-schema -v 0.0
  fi
done

sql_tool --db temporal update-schema \
  -d /etc/temporal/schema/postgresql/v12/temporal/versioned
sql_tool --db temporal_visibility update-schema \
  -d /etc/temporal/schema/postgresql/v12/visibility/versioned

echo "Temporal PostgreSQL schemas are ready."
