#!/bin/bash
set -e

echo "============================================"
echo "Container Deployment Test - Setup"
echo "============================================"
echo ""

# Build Rust binary in release mode
echo "[1/5] Building restic-manager..."
cd "$(dirname "$0")/../.."
cargo build --release
echo "Done."
echo ""

# Build Docker image
echo "[2/5] Building Docker image..."
cd tests/container
docker-compose build
echo "Done."
echo ""

# Start containers
echo "[3/5] Starting containers..."
docker-compose up -d
echo "Done."
echo ""

# Wait for PostgreSQL
echo "[4/5] Waiting for PostgreSQL..."
sleep 10
echo "Done."
echo ""

# Create test data in PostgreSQL
echo "[5/5] Creating test data..."
docker exec restic-test-db psql -U testuser -d testdb -c "
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100),
    email VARCHAR(100),
    created_at TIMESTAMP DEFAULT NOW()
);
"

docker exec restic-test-db psql -U testuser -d testdb -c "
INSERT INTO users (name, email) VALUES
    ('Alice', 'alice@example.com'),
    ('Bob', 'bob@example.com'),
    ('Charlie', 'charlie@example.com');
"

docker exec restic-test-db psql -U testuser -d testdb -c "
SELECT COUNT(*) as user_count FROM users;
"

echo "Done."
echo ""

echo "============================================"
echo "Container Test Environment Ready"
echo "============================================"
echo ""
echo "The restic-manager container is running."
echo ""
echo "Enter the container:"
echo "  docker exec -it restic-manager-test bash"
echo ""
echo "Inside the container, you can:"
echo "  # Setup restic"
echo "  /app/restic-manager setup-restic"
echo ""
echo "  # Create backup data directory"
echo "  mkdir -p /backup-data/dumps"
echo ""
echo "  # Run a test backup"
echo "  /app/restic-manager --config /app/config.toml run --service postgres"
echo ""
echo "  # Setup cron job"
echo "  echo '*/5 * * * * /app/restic-manager --config /app/config.toml run --service postgres >> /var/log/restic-manager/cron.log 2>&1' | crontab -"
echo ""
echo "  # View cron jobs"
echo "  crontab -l"
echo ""
echo "  # View logs"
echo "  tail -f /var/log/restic-manager/cron.log"
echo ""
echo "View logs from outside:"
echo "  tail -f tests/container/logs/cron.log"
echo ""
echo "Cleanup:"
echo "  ./tests/container/cleanup.sh"
echo ""
