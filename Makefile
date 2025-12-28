.PHONY: help build test test-unit test-integration test-container test-all clean lint format check

help:
	@echo "Restic Manager - Make Targets"
	@echo ""
	@echo "Development:"
	@echo "  build              - Build debug binary"
	@echo "  build-release      - Build release binary"
	@echo "  format             - Format code with rustfmt"
	@echo "  lint               - Run clippy linter"
	@echo "  check              - Run cargo check"
	@echo ""
	@echo "Testing:"
	@echo "  test               - Run unit tests only"
	@echo "  test-unit          - Run unit tests only"
	@echo "  test-integration   - Run integration tests with Docker"
	@echo "  test-container     - Run container deployment tests"
	@echo "  test-all           - Run all tests (unit, integration, container)"
	@echo ""
	@echo "Cleanup:"
	@echo "  clean              - Remove build artifacts"
	@echo "  clean-tests        - Clean up test environments"
	@echo ""

build:
	cargo build

build-release:
	cargo build --release

format:
	cargo fmt

lint:
	cargo clippy -- -D warnings

check:
	cargo check

test: test-unit

test-unit:
	@echo "Running unit tests..."
	cargo test --lib
	cargo test --test config_tests

test-integration: build-release
	@echo "Running integration tests..."
	cd tests/integration && ./setup.sh
	cargo run --release -- --config tests/integration/config.toml run --service postgres-backup
	cd tests/integration && ./cleanup.sh

test-container: build-release
	@echo "Running container deployment tests..."
	cd tests/container && ./setup.sh
	docker exec restic-manager-test /app/restic-manager setup-restic
	docker exec restic-manager-test mkdir -p /backup-data/dumps
	docker exec restic-manager-test /app/restic-manager --config /app/config.toml run --service postgres
	cd tests/container && ./cleanup.sh

test-all:
	@chmod +x run-tests.sh
	./run-tests.sh --all

clean:
	cargo clean

clean-tests:
	@echo "Cleaning integration tests..."
	@cd tests/integration && ./cleanup.sh 2>/dev/null || true
	@echo "Cleaning container tests..."
	@cd tests/container && ./cleanup.sh 2>/dev/null || true
	@echo "Test cleanup complete"

# CI simulation
ci: format lint test test-integration test-container
	@echo "All CI checks passed!"
