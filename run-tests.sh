#!/bin/bash

# Comprehensive test runner for restic-manager

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "============================================"
echo "Restic Manager - Comprehensive Test Suite"
echo "============================================"
echo ""

# Parse command line arguments
RUN_UNIT=true
RUN_INTEGRATION=false
RUN_CONTAINER=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --all)
      RUN_UNIT=true
      RUN_INTEGRATION=true
      RUN_CONTAINER=true
      shift
      ;;
    --integration)
      RUN_INTEGRATION=true
      shift
      ;;
    --container)
      RUN_CONTAINER=true
      shift
      ;;
    --unit-only)
      RUN_UNIT=true
      RUN_INTEGRATION=false
      RUN_CONTAINER=false
      shift
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: $0 [--all | --integration | --container | --unit-only]"
      exit 1
      ;;
  esac
done

# Function to print section header
print_section() {
    echo ""
    echo -e "${YELLOW}=== $1 ===${NC}"
    echo ""
}

# Function to print success
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Step 1: Unit Tests
if [ "$RUN_UNIT" = true ]; then
    print_section "Running Unit Tests"

    echo "Building project..."
    if cargo build; then
        print_success "Build successful"
    else
        print_error "Build failed"
        exit 1
    fi

    echo ""
    echo "Running unit tests..."
    if cargo test --lib; then
        print_success "Unit tests passed"
    else
        print_error "Unit tests failed"
        exit 1
    fi

    echo ""
    echo "Running config tests..."
    if cargo test --test config_tests; then
        print_success "Config tests passed"
    else
        print_error "Config tests failed"
        exit 1
    fi
fi

# Step 2: Integration Tests
if [ "$RUN_INTEGRATION" = true ]; then
    print_section "Running Integration Tests"

    echo "Building release binary..."
    if cargo build --release; then
        print_success "Release build successful"
    else
        print_error "Release build failed"
        exit 1
    fi

    echo ""
    echo "Setting up integration test environment..."
    cd tests/integration
    if ./setup.sh; then
        print_success "Integration test setup complete"
    else
        print_error "Integration test setup failed"
        cd ../..
        exit 1
    fi
    cd ../..

    echo ""
    echo "Running integration backup test..."
    if cargo run --release -- --config tests/integration/config.toml run --service postgres-backup; then
        print_success "Integration backup test passed"
    else
        print_error "Integration backup test failed"
        cd tests/integration && ./cleanup.sh && cd ../..
        exit 1
    fi

    echo ""
    echo "Verifying backup artifacts..."
    if [ -d "tests/integration/test-data" ] && [ -d "tests/integration/test-backup-repo" ]; then
        print_success "Backup artifacts verified"
    else
        print_error "Backup artifacts not found"
        cd tests/integration && ./cleanup.sh && cd ../..
        exit 1
    fi

    echo ""
    echo "Cleaning up integration test..."
    cd tests/integration
    ./cleanup.sh
    cd ../..
    print_success "Integration test cleanup complete"
fi

# Step 3: Container Tests
if [ "$RUN_CONTAINER" = true ]; then
    print_section "Running Container Deployment Tests"

    echo "Ensuring release binary exists..."
    if [ ! -f "target/release/restic-manager" ]; then
        echo "Building release binary..."
        if cargo build --release; then
            print_success "Release build successful"
        else
            print_error "Release build failed"
            exit 1
        fi
    fi

    echo ""
    echo "Setting up container test environment..."
    cd tests/container
    if ./setup.sh; then
        print_success "Container test setup complete"
    else
        print_error "Container test setup failed"
        cd ../..
        exit 1
    fi
    cd ../..

    echo ""
    echo "Setting up restic in container..."
    if docker exec restic-manager-test /app/restic-manager setup-restic; then
        print_success "Restic setup in container complete"
    else
        print_error "Restic setup in container failed"
        cd tests/container && ./cleanup.sh && cd ../..
        exit 1
    fi

    echo ""
    echo "Creating backup directory in container..."
    if docker exec restic-manager-test mkdir -p /backup-data/dumps; then
        print_success "Backup directory created"
    else
        print_error "Failed to create backup directory"
        cd tests/container && ./cleanup.sh && cd ../..
        exit 1
    fi

    echo ""
    echo "Running backup in container..."
    if docker exec restic-manager-test /app/restic-manager --config /app/config.toml run --service postgres; then
        print_success "Container backup test passed"
    else
        print_error "Container backup test failed"
        cd tests/container && ./cleanup.sh && cd ../..
        exit 1
    fi

    echo ""
    echo "Verifying backup in container..."
    if docker exec restic-manager-test ls -la /backup-repo >/dev/null 2>&1; then
        print_success "Container backup verified"
    else
        print_error "Container backup verification failed"
        cd tests/container && ./cleanup.sh && cd ../..
        exit 1
    fi

    echo ""
    echo "Cleaning up container test..."
    cd tests/container
    ./cleanup.sh
    cd ../..
    print_success "Container test cleanup complete"
fi

# Summary
print_section "Test Summary"
echo -e "${GREEN}All tests completed successfully!${NC}"
echo ""

if [ "$RUN_UNIT" = true ]; then
    echo "✓ Unit tests: PASSED"
fi
if [ "$RUN_INTEGRATION" = true ]; then
    echo "✓ Integration tests: PASSED"
fi
if [ "$RUN_CONTAINER" = true ]; then
    echo "✓ Container tests: PASSED"
fi

echo ""
echo "============================================"
