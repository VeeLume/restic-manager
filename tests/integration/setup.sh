#!/bin/bash
set -e

echo "========================================"
echo "Restic Manager Integration Test Setup"
echo "========================================"
echo ""

# Create necessary directories
echo "[1/6] Creating test directories..."
mkdir -p test-data/dumps
mkdir -p test-backup-repo
echo "Done."
echo ""

# Start Docker container
echo "[2/6] Starting PostgreSQL test container..."
docker-compose up -d
echo "Done."
echo ""

# Wait for PostgreSQL to be ready
echo "[3/6] Waiting for PostgreSQL to be ready..."
sleep 10
echo "Done."
echo ""

# Create test data
echo "[4/6] Creating test data in PostgreSQL..."
docker exec restic-test-postgres psql -U testuser -d testdb -c "CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name VARCHAR(100), email VARCHAR(100), created_at TIMESTAMP DEFAULT NOW());"
docker exec restic-test-postgres psql -U testuser -d testdb -c "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com'), ('Bob', 'bob@example.com'), ('Charlie', 'charlie@example.com');"
docker exec restic-test-postgres psql -U testuser -d testdb -c "CREATE TABLE IF NOT EXISTS posts (id SERIAL PRIMARY KEY, user_id INTEGER REFERENCES users(id), title VARCHAR(200), content TEXT, created_at TIMESTAMP DEFAULT NOW());"
docker exec restic-test-postgres psql -U testuser -d testdb -c "INSERT INTO posts (user_id, title, content) VALUES (1, 'First Post', 'Hello World!'), (2, 'Second Post', 'Testing backups');"
echo "Done."
echo ""

# Verify data
echo "[5/6] Verifying test data..."
docker exec restic-test-postgres psql -U testuser -d testdb -c "SELECT COUNT(*) as user_count FROM users;"
docker exec restic-test-postgres psql -U testuser -d testdb -c "SELECT COUNT(*) as post_count FROM posts;"
echo "Done."
echo ""

echo "[6/6] Setup complete!"
echo ""
echo "========================================"
echo "Test Environment Ready"
echo "========================================"
echo ""
echo "You can now run the backup test:"
echo "  cargo run --release -- --config tests/integration/config.toml run --service postgres-backup"
echo ""
echo "To view container logs:"
echo "  docker logs restic-test-postgres"
echo ""
echo "To connect to database:"
echo "  docker exec -it restic-test-postgres psql -U testuser -d testdb"
echo ""
echo "To cleanup:"
echo "  ./tests/integration/cleanup.sh"
echo ""
