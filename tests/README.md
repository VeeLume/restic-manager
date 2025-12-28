# Restic Manager Tests

Two comprehensive test suites for validating restic-manager functionality.

## Test Suites

### 1. Integration Test (`integration/`)
Tests restic-manager with a real PostgreSQL container on the **host system**.

**What it tests:**
- Docker volume backup
- Pre/post backup hooks
- Database dumps
- Restic operations
- Cross-platform compatibility

**Quick start:**
```bash
cd tests/integration
./setup.sh           # Linux/macOS
setup.bat            # Windows
```

[Read more →](integration/README.md)

---

### 2. Container Test (`container/`)
Tests restic-manager in a **fully isolated Ubuntu container** simulating real deployment.

**What it tests:**
- Complete Ubuntu deployment
- Cron job setup and execution
- Docker-in-Docker support
- Scheduled automated backups
- Log management
- Real-world deployment scenario

**Quick start:**
```bash
cd tests/container
./setup.sh
```

[Read more →](container/README.md)

---

## Which Test Should I Run?

### Run Integration Test if you want to:
- ✅ Quickly validate basic functionality
- ✅ Test on your development machine
- ✅ Test Docker volume operations
- ✅ Verify hooks work correctly

### Run Container Test if you want to:
- ✅ Validate deployment on Ubuntu/Linux
- ✅ Test cron job configuration
- ✅ Simulate production environment
- ✅ Test automated scheduled backups
- ✅ Verify complete deployment workflow

### Run Both Tests to:
- ✅ Ensure complete coverage
- ✅ Validate cross-platform compatibility
- ✅ Test both manual and automated scenarios

---

## Test Matrix

| Feature | Integration Test | Container Test |
|---------|-----------------|----------------|
| PostgreSQL backup | ✅ | ✅ |
| Docker volumes | ✅ | Limited |
| Pre/post hooks | ✅ | ✅ |
| Manual execution | ✅ | ✅ |
| Cron automation | ❌ | ✅ |
| Ubuntu deployment | ❌ | ✅ |
| Cross-platform | ✅ (Win/Lin/Mac) | ✅ (Linux) |
| Isolation | Shared host | Full container |
| Setup time | ~1 minute | ~3 minutes |

---

## Running All Tests

### Sequential
```bash
# Test 1: Integration
cd tests/integration
./setup.sh
cd ../..
cargo run --release -- --config tests/integration/config.toml run
cd tests/integration
./cleanup.sh

# Test 2: Container
cd ../container
./setup.sh
docker exec -it restic-manager-test /app/restic-manager setup-restic
docker exec -it restic-manager-test mkdir -p /backup-data/dumps
docker exec -it restic-manager-test /app/restic-manager --config /app/config.toml run
./cleanup.sh
```

### Automated Test Script
Create a `run-all-tests.sh`:
```bash
#!/bin/bash
set -e

echo "Running all restic-manager tests..."

# Integration test
echo "=== Integration Test ==="
cd tests/integration
./setup.sh
cd ../..
cargo run --release -- --config tests/integration/config.toml run --service postgres-backup
cd tests/integration
./cleanup.sh
cd ../..

# Container test
echo "=== Container Test ==="
cd tests/container
./setup.sh
docker exec restic-manager-test /app/restic-manager setup-restic
docker exec restic-manager-test mkdir -p /backup-data/dumps
docker exec restic-manager-test /app/restic-manager --config /app/config.toml run --service postgres
./cleanup.sh
cd ../..

echo "All tests passed!"
```

---

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Tests
on: [push, pull_request]

jobs:
  integration-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release
      - run: cd tests/integration && ./setup.sh
      - run: cargo run --release -- --config tests/integration/config.toml run
      - run: cd tests/integration && ./cleanup.sh

  container-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cd tests/container && ./setup.sh
      - run: docker exec restic-manager-test /app/restic-manager setup-restic
      - run: docker exec restic-manager-test mkdir -p /backup-data/dumps
      - run: docker exec restic-manager-test /app/restic-manager --config /app/config.toml run
      - run: cd tests/container && ./cleanup.sh
```

---

## Troubleshooting

### Integration Test Issues

**Docker not running:**
```bash
docker ps
# If fails, start Docker Desktop
```

**Permission denied:**
```bash
chmod +x tests/integration/*.sh
```

### Container Test Issues

**Binary not found:**
```bash
# Build first
cargo build --release
ls -l target/release/restic-manager
```

**Cron not executing:**
```bash
docker exec restic-manager-test service cron status
docker exec restic-manager-test crontab -l
```

---

## Adding New Tests

### Adding to Integration Test
1. Edit `tests/integration/config.toml`
2. Add new service configuration
3. Update setup scripts if needed
4. Document in README.md

### Adding to Container Test
1. Edit `tests/container/config.toml`
2. Update Dockerfile if new dependencies needed
3. Modify entrypoint.sh if needed
4. Document in README.md

---

## Cleanup

### Clean Integration Test
```bash
cd tests/integration
./cleanup.sh
```

### Clean Container Test
```bash
cd tests/container
./cleanup.sh
```

### Clean Everything
```bash
cd tests/integration && ./cleanup.sh && cd ../container && ./cleanup.sh && cd ../..
```

---

For detailed documentation on each test, see:
- [Integration Test Guide](integration/README.md)
- [Container Test Guide](container/README.md)
- [Main Test Guide](../TEST-GUIDE.md)
