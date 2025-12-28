#!/bin/bash
# Deployment script for restic-manager
# Run this from WSL Ubuntu to deploy to your Ubuntu server

set -e  # Exit on error

echo "=========================================="
echo "Restic Manager - Production Deployment"
echo "=========================================="
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running in WSL
if ! grep -qi microsoft /proc/version 2>/dev/null; then
    echo -e "${YELLOW}Warning: Not running in WSL. This script is designed for WSL Ubuntu.${NC}"
fi

# Step 1: Get server details
echo -e "${GREEN}Step 1: Server Configuration${NC}"
read -p "Enter your server hostname or IP: " SERVER_HOST
read -p "Enter SSH username (default: root): " SERVER_USER
SERVER_USER=${SERVER_USER:-root}
read -p "Enter SSH port (default: 22): " SERVER_PORT
SERVER_PORT=${SERVER_PORT:-22}

echo ""
echo "Server: $SERVER_USER@$SERVER_HOST:$SERVER_PORT"
read -p "Is this correct? (y/n): " CONFIRM
if [[ $CONFIRM != "y" ]]; then
    echo "Aborted."
    exit 1
fi

# Step 2: Test SSH connection
echo ""
echo -e "${GREEN}Step 2: Testing SSH connection...${NC}"
if ssh -p $SERVER_PORT -o ConnectTimeout=5 $SERVER_USER@$SERVER_HOST "echo 'Connection successful'"; then
    echo -e "${GREEN}✓ SSH connection successful${NC}"
else
    echo -e "${RED}✗ SSH connection failed${NC}"
    echo "Please ensure:"
    echo "  1. Server is accessible"
    echo "  2. SSH keys are set up"
    echo "  3. Hostname/IP and port are correct"
    exit 1
fi

# Step 3: Check if binary exists
echo ""
echo -e "${GREEN}Step 3: Checking binary...${NC}"
BINARY_PATH="target/release/restic-manager"
if [[ ! -f $BINARY_PATH ]]; then
    echo -e "${RED}✗ Binary not found at $BINARY_PATH${NC}"
    echo "Building binary..."
    cargo build --release
fi

if [[ -f $BINARY_PATH ]]; then
    BINARY_SIZE=$(ls -lh $BINARY_PATH | awk '{print $5}')
    echo -e "${GREEN}✓ Binary found ($BINARY_SIZE)${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

# Step 4: Check configuration
echo ""
echo -e "${GREEN}Step 4: Configuration setup${NC}"
CONFIG_PATH="production-config.toml"

if [[ ! -f $CONFIG_PATH ]]; then
    echo -e "${YELLOW}! Configuration file not found: $CONFIG_PATH${NC}"
    echo "Please edit production-config.toml with your settings first."
    exit 1
fi

echo -e "${GREEN}✓ Configuration file found${NC}"
echo ""
echo "Review your configuration:"
echo "  - Backup destinations"
echo "  - Services to backup"
echo "  - Schedules and retention"
echo ""
read -p "Have you reviewed and edited production-config.toml? (y/n): " CONFIG_READY
if [[ $CONFIG_READY != "y" ]]; then
    echo ""
    echo "Please edit production-config.toml with your settings:"
    echo "  nano production-config.toml"
    echo ""
    echo "Then run this script again."
    exit 1
fi

# Step 5: Password setup
echo ""
echo -e "${GREEN}Step 5: Restic password${NC}"
echo "You need a strong password for restic encryption."
echo "This will be stored in /root/.restic_password on the server."
echo ""
read -sp "Enter restic password (hidden): " RESTIC_PASSWORD
echo ""
read -sp "Confirm password: " RESTIC_PASSWORD_CONFIRM
echo ""

if [[ $RESTIC_PASSWORD != $RESTIC_PASSWORD_CONFIRM ]]; then
    echo -e "${RED}✗ Passwords don't match${NC}"
    exit 1
fi

if [[ ${#RESTIC_PASSWORD} -lt 16 ]]; then
    echo -e "${YELLOW}Warning: Password is less than 16 characters${NC}"
    read -p "Continue anyway? (y/n): " WEAK_PW
    if [[ $WEAK_PW != "y" ]]; then
        exit 1
    fi
fi

# Step 6: Deploy
echo ""
echo -e "${GREEN}Step 6: Deploying to server...${NC}"

echo "  → Copying binary..."
scp -P $SERVER_PORT $BINARY_PATH $SERVER_USER@$SERVER_HOST:/tmp/restic-manager

echo "  → Copying configuration..."
scp -P $SERVER_PORT $CONFIG_PATH $SERVER_USER@$SERVER_HOST:/tmp/config.toml

echo "  → Installing on server..."
ssh -p $SERVER_PORT -t $SERVER_USER@$SERVER_HOST << 'ENDSSH'
set -e

# Move binary
sudo mv /tmp/restic-manager /usr/local/bin/
sudo chmod +x /usr/local/bin/restic-manager

# Create config directory
sudo mkdir -p /etc/restic-manager
sudo mv /tmp/config.toml /etc/restic-manager/

# Create log directory
sudo mkdir -p /var/log/restic-manager

# Create backup directory
sudo mkdir -p /backup/repos

echo "✓ Files installed"
exit
ENDSSH

# Step 7: Create password file
echo "  → Creating password file..."
ssh -p $SERVER_PORT -t $SERVER_USER@$SERVER_HOST "echo '$RESTIC_PASSWORD' | sudo -S tee /root/.restic_password > /dev/null && sudo chmod 600 /root/.restic_password"

# Step 8: Setup restic
echo ""
echo -e "${GREEN}Step 7: Installing restic binary...${NC}"
ssh -p $SERVER_PORT -t $SERVER_USER@$SERVER_HOST << 'ENDSSH'
set -e
echo "Downloading restic..."
sudo restic-manager setup-restic
echo "✓ Restic installed"
exit
ENDSSH

# Step 9: Validate configuration
echo ""
echo -e "${GREEN}Step 8: Validating configuration...${NC}"
ssh -p $SERVER_PORT -t $SERVER_USER@$SERVER_HOST << 'ENDSSH'
set -e
echo "Validating config..."
sudo restic-manager --config /etc/restic-manager/config.toml validate
echo "✓ Configuration valid"
exit
ENDSSH

# Step 10: Initialize
echo ""
echo -e "${GREEN}Step 9: Initializing setup...${NC}"
echo "This will:"
echo "  - Create directories"
echo "  - Initialize restic repositories"
echo "  - Install cron jobs"
echo ""
read -p "Run setup now? (y/n): " RUN_SETUP

if [[ $RUN_SETUP == "y" ]]; then
    ssh -p $SERVER_PORT -t $SERVER_USER@$SERVER_HOST << 'ENDSSH'
set -e
echo "Running setup..."
sudo restic-manager --config /etc/restic-manager/config.toml setup
echo "✓ Setup complete"
exit
ENDSSH
else
    echo "Skipping setup. You can run it manually:"
    echo "  ssh $SERVER_USER@$SERVER_HOST"
    echo "  sudo restic-manager --config /etc/restic-manager/config.toml setup"
fi

# Step 11: Test backup
echo ""
echo -e "${GREEN}Step 10: Test backup${NC}"
read -p "Run a test backup now? (y/n): " RUN_TEST

if [[ $RUN_TEST == "y" ]]; then
    echo "Enter service name to test (from your config.toml):"
    read -p "Service name: " SERVICE_NAME

    ssh -p $SERVER_PORT -t $SERVER_USER@$SERVER_HOST << ENDSSH
set -e
echo "Running test backup for service: $SERVICE_NAME"
sudo restic-manager --config /etc/restic-manager/config.toml run --service $SERVICE_NAME
echo ""
echo "Checking status..."
sudo restic-manager --config /etc/restic-manager/config.toml status --service $SERVICE_NAME
echo ""
echo "Listing snapshots..."
sudo restic-manager --config /etc/restic-manager/config.toml snapshots --service $SERVICE_NAME
exit
ENDSSH
fi

# Summary
echo ""
echo "=========================================="
echo -e "${GREEN}Deployment Complete!${NC}"
echo "=========================================="
echo ""
echo "Server: $SERVER_USER@$SERVER_HOST:$SERVER_PORT"
echo ""
echo "Installed files:"
echo "  Binary: /usr/local/bin/restic-manager"
echo "  Config: /etc/restic-manager/config.toml"
echo "  Password: /root/.restic_password"
echo "  Logs: /var/log/restic-manager/"
echo ""
echo "Next steps:"
echo "  1. Verify cron jobs: ssh $SERVER_USER@$SERVER_HOST 'sudo crontab -l'"
echo "  2. Check status: ssh $SERVER_USER@$SERVER_HOST 'sudo restic-manager status'"
echo "  3. Monitor logs: ssh $SERVER_USER@$SERVER_HOST 'sudo tail -f /var/log/restic-manager/*.log'"
echo ""
echo "Useful commands on server:"
echo "  restic-manager status                    # View backup status"
echo "  restic-manager snapshots --service NAME  # List snapshots"
echo "  restic-manager verify                    # Verify repositories"
echo "  restic-manager restore --service NAME    # Restore (interactive)"
echo ""
echo -e "${YELLOW}Important: Keep your restic password safe!${NC}"
echo "Password is stored in /root/.restic_password on server"
echo "You should also backup this password securely elsewhere"
echo ""
