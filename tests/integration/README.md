# Integration Test

Tests restic-manager with a real PostgreSQL Docker container, including:
- Pre-backup hooks (database dump)
- Docker volume backup
- Post-backup hooks (verification)
- Full backup workflow

## Quick Start

### Windows
```cmd
cd tests\integration
setup.bat
cd ..\..
cargo run --release -- --config tests/integration/config.toml run --service postgres-backup
cd tests\integration
cleanup.bat
```

### Linux/macOS
```bash
cd tests/integration
chmod +x setup.sh cleanup.sh
./setup.sh
cd ../..
cargo run --release -- --config tests/integration/config.toml run --service postgres-backup
cd tests/integration
./cleanup.sh
```

## What It Tests

- ✅ Docker volume archiving
- ✅ Pre-backup hooks (database dumps)
- ✅ Post-backup hooks (verification)
- ✅ Restic repository operations
- ✅ Retention policy application
- ✅ Cross-platform compatibility

See the main [TEST-GUIDE.md](../../TEST-GUIDE.md) for detailed documentation.
