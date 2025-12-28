# Restic Manager Tests

**Note:** Tests have been moved to the `restic-manager-tests/` workspace crate.

## Quick Start

```bash
# Run all fast tests
cargo test -p restic-manager-tests

# Run Docker integration tests (requires Docker)
cargo test -p restic-manager-tests -- --ignored

# Run everything
cargo test -p restic-manager-tests -- --include-ignored
```

## Test Location

All tests are now in the `restic-manager-tests/` crate:

```
restic-manager-tests/
├── Cargo.toml
├── src/lib.rs              # Test utilities (ConfigBuilder, mocks, fixtures)
├── tests/
│   ├── unit/               # Unit tests (config, restic, docker)
│   ├── commands/           # Command tests (run, restore, status, etc.)
│   └── integration/        # Docker integration tests (ignored by default)
└── README.md               # Detailed test documentation
```

See `restic-manager-tests/README.md` for complete documentation.
