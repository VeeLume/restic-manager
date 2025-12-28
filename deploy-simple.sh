#!/bin/bash
# Simplified deployment script that avoids sudo heredoc issues

set -e

echo "=========================================="
echo "Restic Manager - Simple Deployment"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Get server details
read -p "Enter your server hostname or IP: " SERVER_HOST
read -p "Enter SSH username [valerie]: " SERVER_USER
SERVER_USER=${SERVER_USER:-valerie}

echo ""
echo "Server: $SERVER_USER@$SERVER_HOST"
read -p "Is this correct? (y/n): " CONFIRM
if [[ $CONFIRM != "y" ]]; then
    echo "Aborted."
    exit 1
fi

# Test SSH
echo ""
echo -e "${GREEN}Testing SSH connection...${NC}"
if ! ssh $SERVER_USER@$SERVER_HOST "echo 'Connected'"; then
    echo "SSH connection failed"
    exit 1
fi

# Check binary
BINARY_PATH="target/release/restic-manager"
if [[ ! -f $BINARY_PATH ]]; then
    echo "Binary not found. Building..."
    cargo build --release
fi

# Check config
if [[ ! -f production-config.toml ]]; then
    echo "Configuration file not found: production-config.toml"
    exit 1
fi

# Get password
echo ""
echo -e "${GREEN}Restic Password Setup${NC}"
read -sp "Enter restic password: " RESTIC_PASSWORD
echo ""
read -sp "Confirm password: " RESTIC_PASSWORD_CONFIRM
echo ""

if [[ $RESTIC_PASSWORD != $RESTIC_PASSWORD_CONFIRM ]]; then
    echo "Passwords don't match"
    exit 1
fi

# Copy files
echo ""
echo -e "${GREEN}Copying files to server...${NC}"
scp $BINARY_PATH $SERVER_USER@$SERVER_HOST:/tmp/restic-manager
scp production-config.toml $SERVER_USER@$SERVER_HOST:/tmp/config.toml

# Create setup script on server
echo ""
echo -e "${GREEN}Creating setup script on server...${NC}"
ssh $SERVER_USER@$SERVER_HOST "cat > /tmp/setup-restic-manager.sh" << 'EOF'
#!/bin/bash
set -e

echo "Installing restic-manager..."

# Install binary
sudo mv /tmp/restic-manager /usr/local/bin/
sudo chmod +x /usr/local/bin/restic-manager

# Create config directory
sudo mkdir -p /etc/restic-manager
sudo mv /tmp/config.toml /etc/restic-manager/

# Create log directory
sudo mkdir -p /var/log/restic-manager
sudo chown $USER:$USER /var/log/restic-manager

# Create backup directory (adjust path if needed)
sudo mkdir -p /backup/repos
sudo chown $USER:$USER /backup/repos

echo "Files installed successfully"
echo ""
echo "Next steps:"
echo "1. Create password file: echo 'your-password' > /home/$USER/restic_password && chmod 600 /home/$USER/restic_password"
echo "2. Download restic: restic-manager setup-restic"
echo "3. Validate config: restic-manager --config /etc/restic-manager/config.toml validate"
echo "4. Run setup: restic-manager --config /etc/restic-manager/config.toml setup"
EOF

ssh $SERVER_USER@$SERVER_HOST "chmod +x /tmp/setup-restic-manager.sh"

# Create password file
echo ""
echo -e "${GREEN}Creating password file...${NC}"
ssh $SERVER_USER@$SERVER_HOST "echo '$RESTIC_PASSWORD' > /home/$SERVER_USER/restic_password && chmod 600 /home/$SERVER_USER/restic_password"

echo ""
echo "=========================================="
echo -e "${GREEN}Files Copied Successfully!${NC}"
echo "=========================================="
echo ""
echo "Now run these commands on your server:"
echo ""
echo "  ssh $SERVER_USER@$SERVER_HOST"
echo "  /tmp/setup-restic-manager.sh"
echo ""
echo "This will install the binary and config (requires sudo password)."
echo ""
echo "After that, run:"
echo "  restic-manager setup-restic"
echo "  restic-manager --config /etc/restic-manager/config.toml validate"
echo "  restic-manager --config /etc/restic-manager/config.toml setup"
echo ""
echo "Or run the automated setup:"
echo ""
read -p "Would you like to connect to the server now? (y/n): " CONNECT

if [[ $CONNECT == "y" ]]; then
    echo ""
    echo "Connecting to server..."
    echo "Run: /tmp/setup-restic-manager.sh"
    echo ""
    ssh -t $SERVER_USER@$SERVER_HOST
fi
