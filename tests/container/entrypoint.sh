#!/bin/bash
set -e

echo "==================================="
echo "Restic Manager Container Test"
echo "==================================="
echo ""

# Start cron
echo "Starting cron daemon..."
service cron start

# Setup restic if binary is mounted
if [ -f /app/restic-manager ]; then
    echo "Installing restic binary..."
    /app/restic-manager setup-restic
    echo "Done."
    echo ""
fi

# Show status
if [ -f /app/restic-manager ]; then
    echo "Restic Manager Status:"
    /app/restic-manager restic-version || true
    echo ""
fi

echo "Container ready!"
echo ""
echo "Available commands:"
echo "  /app/restic-manager setup-restic          # Download restic"
echo "  /app/restic-manager --config /app/config.toml list"
echo "  /app/restic-manager --config /app/config.toml run"
echo "  crontab -l                                 # View cron jobs"
echo ""

# Keep container running
exec "$@"
