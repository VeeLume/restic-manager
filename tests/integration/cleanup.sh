#!/bin/bash

echo "========================================"
echo "Restic Manager Integration Test Cleanup"
echo "========================================"
echo ""

cd "$(dirname "$0")" || exit 1

echo "[1/4] Stopping Docker container..."
docker-compose down -v
echo "Done."
echo ""

echo "[2/4] Removing test data..."
rm -rf test-data
echo "Done."
echo ""

echo "[3/4] Removing backup repository..."
rm -rf test-backup-repo
echo "Done."
echo ""

echo "[4/4] Removing temporary files..."
rm -f pre-hook-test.txt
echo "Done."
echo ""

echo "========================================"
echo "Cleanup Complete"
echo "========================================"
