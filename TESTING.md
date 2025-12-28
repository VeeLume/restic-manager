# Testing Guide

Comprehensive testing documentation for restic-manager.

## Quick Start

### Run Unit Tests Only
```bash
cargo test
# or
make test
```

### Run All Tests (Unit + Integration + Container)
```bash
./run-tests.sh --all
# or
make test-all
```

### Run Specific Test Suites
```bash
# Unit tests only
./run-tests.sh --unit-only

# Integration tests only
./run-tests.sh --integration

# Container tests only
./run-tests.sh --container
```

## Test Structure

### 1. Unit Tests
Located in the source code with `#[cfg(test)]` modules.

**What they test:**
- Configuration parsing and validation
- Profile inheritance
- URL building for restic repositories
- Path expansion and exclusion merging

**Run with:**
```bash
cargo test --lib
cargo test --test config_tests
```

**Examples:**
- `src/config/mod.rs` - Tests for configuration merging
- `src/utils/restic.rs` - Tests for repository URL building
- `tests/config_tests.rs` - Integration tests for config loading

### 2. Integration Tests
Located in `tests/integration/`.

**What they test:**
- Full backup workflow with real PostgreSQL container
- Docker volume backup
- Pre/post backup hooks
- Database dumps
- Restic operations

**Run with:**
```bash
make test-integration
# or
cd tests/integration && ./setup.sh
cargo run --release -- --config tests/integration/config.toml run
cd tests/integration && ./cleanup.sh
```

### 3. Container Deployment Tests
Located in `tests/container/`.

**What they test:**
- Ubuntu container deployment
- Cron job setup
- Automated backups
- Real-world deployment scenario

**Run with:**
```bash
make test-container
# or
cd tests/container && ./setup.sh
docker exec restic-manager-test /app/restic-manager setup-restic
docker exec restic-manager-test /app/restic-manager --config /app/config.toml run
cd tests/container && ./cleanup.sh
```

## Automated Testing

### Using the Test Runner Script

The `run-tests.sh` script provides a convenient way to run all tests:

```bash
# Make it executable
chmod +x run-tests.sh

# Run all tests
./run-tests.sh --all

# Run specific test suites
./run-tests.sh --unit-only
./run-tests.sh --integration
./run-tests.sh --container
```

### Using Makefile

The Makefile provides convenient targets:

```bash
# See all available targets
make help

# Run unit tests
make test

# Run integration tests
make test-integration

# Run container tests
make test-container

# Run everything
make test-all

# Clean up
make clean-tests
```

## Continuous Integration

GitHub Actions automatically runs tests on every push and pull request.

**Workflow:** `.github/workflows/ci.yml`

**Jobs:**
1. **Unit Tests** - Runs on Linux, macOS, and Windows
2. **Integration Tests** - Runs on Linux with Docker
3. **Container Tests** - Runs on Linux with Docker
4. **Security Audit** - Runs `cargo audit` for vulnerabilities

**View CI results:**
- Go to the Actions tab on GitHub
- Check the status of your commits

## Writing New Tests

### Adding Unit Tests

Add tests to the relevant module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_feature() {
        // Test code here
        assert_eq!(2 + 2, 4);
    }
}
```

### Adding Integration Tests

Create a new file in `tests/`:

```rust
// tests/my_integration_test.rs

#[test]
fn test_my_integration() {
    // Test code using the library
    let config = restic_manager::config::load_config("config.toml").unwrap();
    assert!(config.services.len() > 0);
}
```

### Adding to Test Suites

To add to integration or container tests:

1. Edit `tests/integration/config.toml` or `tests/container/config.toml`
2. Update setup scripts if needed
3. Document in the respective README.md

## Test Coverage

To generate test coverage reports:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage

# Open coverage report
open coverage/index.html
```

## Troubleshooting

### Integration Tests Fail

```bash
# Check Docker is running
docker ps

# Manually run setup
cd tests/integration
./setup.sh

# Check container logs
docker logs restic-test-postgres

# Clean up and retry
./cleanup.sh
```

### Container Tests Fail

```bash
# Check binary exists
ls -l target/release/restic-manager

# Rebuild
cargo build --release

# Check container status
docker ps -a | grep restic-manager-test

# Clean up
cd tests/container && ./cleanup.sh
```

### Unit Tests Fail

```bash
# Update dependencies
cargo update

# Clean build
cargo clean
cargo build

# Run specific test
cargo test test_name -- --nocapture
```

## Best Practices

1. **Run unit tests frequently** during development
2. **Run integration tests** before committing major changes
3. **Run all tests** before creating a pull request
4. **Keep tests isolated** - each test should clean up after itself
5. **Use descriptive test names** that explain what is being tested
6. **Test edge cases** and error conditions, not just happy paths

## Performance

Test suite timing (approximate):

- Unit tests: ~5 seconds
- Integration tests: ~30 seconds
- Container tests: ~60 seconds
- **Total:** ~95 seconds

To speed up development:
- Use `cargo test --lib` for quick feedback
- Only run integration/container tests when needed
- Use `cargo watch` for continuous testing during development

## Additional Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Integration Test Guide](tests/README.md)
- [Container Test Guide](tests/container/README.md)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
