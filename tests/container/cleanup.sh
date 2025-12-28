#!/bin/bash

echo "============================================"
echo "Container Deployment Test - Cleanup"
echo "============================================"
echo ""

cd "$(dirname "$0")"

echo "[1/3] Stopping containers..."
docker-compose down -v
echo "Done."
echo ""

echo "[2/3] Removing logs..."
rm -rf logs
echo "Done."
echo ""

echo "[3/3] Cleanup complete!"
echo "============================================"
