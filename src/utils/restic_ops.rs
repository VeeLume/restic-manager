//! Restic operations abstraction for testability
//!
//! This module provides a trait-based abstraction for restic operations,
//! enabling dependency injection and mocking for tests.

#![allow(dead_code)]

use crate::config::RetentionPolicy;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

// Re-export types from restic module
pub use super::restic::{ResticEnv, Snapshot};

/// Abstraction for restic operations, enabling mocking in tests
pub trait ResticOperations: Send + Sync {
    /// Initialize a restic repository if it doesn't exist
    fn init_repository(&self, env: &ResticEnv, timeout: Duration) -> Result<()>;

    /// Backup files to restic repository
    fn backup(
        &self,
        env: &ResticEnv,
        paths: &[PathBuf],
        excludes: &[String],
        timeout: Duration,
    ) -> Result<()>;

    /// List snapshots in a repository
    fn list_snapshots(&self, env: &ResticEnv, timeout: Duration) -> Result<Vec<Snapshot>>;

    /// Restore from a snapshot
    fn restore_snapshot(
        &self,
        env: &ResticEnv,
        snapshot_id: &str,
        target_dir: Option<&str>,
        include_paths: &[String],
        timeout: Duration,
    ) -> Result<()>;

    /// Apply retention policy to repository
    fn apply_retention(
        &self,
        env: &ResticEnv,
        retention: &RetentionPolicy,
        timeout: Duration,
    ) -> Result<()>;

    /// Check repository integrity
    fn check_repository(
        &self,
        env: &ResticEnv,
        read_data: bool,
        timeout: Duration,
    ) -> Result<String>;

    /// Unlock repository (useful after failures)
    fn unlock_repository(&self, env: &ResticEnv, timeout: Duration) -> Result<()>;

    /// Get repository stats
    fn get_stats(&self, env: &ResticEnv, timeout: Duration) -> Result<String>;

    /// Count snapshots in a repository
    fn count_snapshots(&self, env: &ResticEnv, timeout: Duration) -> Result<usize>;

    /// Get the latest snapshot for a repository
    fn get_latest_snapshot(
        &self,
        env: &ResticEnv,
        timeout: Duration,
    ) -> Result<Option<Snapshot>>;

    /// List files in a snapshot
    fn list_snapshot_files(
        &self,
        env: &ResticEnv,
        snapshot_id: &str,
        timeout: Duration,
    ) -> Result<Vec<String>>;
}

/// Default implementation using real restic calls
#[derive(Debug, Clone, Default)]
pub struct RealResticOps;

impl RealResticOps {
    pub fn new() -> Self {
        Self
    }
}

impl ResticOperations for RealResticOps {
    fn init_repository(&self, env: &ResticEnv, timeout: Duration) -> Result<()> {
        super::restic::init_repository(env, timeout)
    }

    fn backup(
        &self,
        env: &ResticEnv,
        paths: &[PathBuf],
        excludes: &[String],
        timeout: Duration,
    ) -> Result<()> {
        super::restic::backup(env, paths, excludes, timeout)
    }

    fn list_snapshots(&self, env: &ResticEnv, timeout: Duration) -> Result<Vec<Snapshot>> {
        super::restic::list_snapshots(env, timeout)
    }

    fn restore_snapshot(
        &self,
        env: &ResticEnv,
        snapshot_id: &str,
        target_dir: Option<&str>,
        include_paths: &[String],
        timeout: Duration,
    ) -> Result<()> {
        super::restic::restore_snapshot(env, snapshot_id, target_dir, include_paths, timeout)
    }

    fn apply_retention(
        &self,
        env: &ResticEnv,
        retention: &RetentionPolicy,
        timeout: Duration,
    ) -> Result<()> {
        super::restic::apply_retention(env, retention, timeout)
    }

    fn check_repository(
        &self,
        env: &ResticEnv,
        read_data: bool,
        timeout: Duration,
    ) -> Result<String> {
        super::restic::check_repository(env, read_data, timeout)
    }

    fn unlock_repository(&self, env: &ResticEnv, timeout: Duration) -> Result<()> {
        super::restic::unlock_repository(env, timeout)
    }

    fn get_stats(&self, env: &ResticEnv, timeout: Duration) -> Result<String> {
        super::restic::get_stats(env, timeout)
    }

    fn count_snapshots(&self, env: &ResticEnv, timeout: Duration) -> Result<usize> {
        super::restic::count_snapshots(env, timeout)
    }

    fn get_latest_snapshot(
        &self,
        env: &ResticEnv,
        timeout: Duration,
    ) -> Result<Option<Snapshot>> {
        super::restic::get_latest_snapshot(env, timeout)
    }

    fn list_snapshot_files(
        &self,
        env: &ResticEnv,
        snapshot_id: &str,
        timeout: Duration,
    ) -> Result<Vec<String>> {
        super::restic::list_snapshot_files(env, snapshot_id, timeout)
    }
}

/// Mock implementation for testing
/// Available for use in external test crates
#[allow(dead_code)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Recorded operation call
    #[derive(Clone, Debug)]
    pub enum ResticCall {
        Init,
        Backup { paths: Vec<PathBuf> },
        ListSnapshots,
        Restore { snapshot_id: String },
        ApplyRetention,
        Check { read_data: bool },
        Unlock,
        GetStats,
        CountSnapshots,
        GetLatestSnapshot,
        ListSnapshotFiles { snapshot_id: String },
    }

    /// Mock restic operations for testing
    #[derive(Clone, Default)]
    pub struct MockResticOps {
        /// Recorded operation calls
        pub calls: Arc<Mutex<Vec<ResticCall>>>,
        /// Pre-configured snapshots to return
        pub snapshots: Arc<Mutex<Vec<Snapshot>>>,
        /// Whether backup should fail
        pub should_fail_backup: Arc<Mutex<bool>>,
        /// Whether restore should fail
        pub should_fail_restore: Arc<Mutex<bool>>,
        /// Whether init should fail
        pub should_fail_init: Arc<Mutex<bool>>,
        /// Whether list_snapshots should fail
        pub should_fail_list: Arc<Mutex<bool>>,
        /// Whether check should fail
        pub should_fail_check: Arc<Mutex<bool>>,
        /// Stats to return
        pub stats: Arc<Mutex<String>>,
        /// Check result to return
        pub check_result: Arc<Mutex<String>>,
        /// Snapshot files (snapshot_id -> files)
        pub snapshot_files: Arc<Mutex<std::collections::HashMap<String, Vec<String>>>>,
    }

    impl MockResticOps {
        pub fn new() -> Self {
            Self {
                stats: Arc::new(Mutex::new("1.0 GiB".to_string())),
                check_result: Arc::new(Mutex::new("no errors found".to_string())),
                ..Default::default()
            }
        }

        /// Configure snapshots to return
        pub fn with_snapshots(self, snapshots: Vec<Snapshot>) -> Self {
            *self.snapshots.lock().unwrap() = snapshots;
            self
        }

        /// Configure backup to fail
        pub fn with_failing_backup(self) -> Self {
            *self.should_fail_backup.lock().unwrap() = true;
            self
        }

        /// Configure restore to fail
        pub fn with_failing_restore(self) -> Self {
            *self.should_fail_restore.lock().unwrap() = true;
            self
        }

        /// Configure init to fail
        pub fn with_failing_init(self) -> Self {
            *self.should_fail_init.lock().unwrap() = true;
            self
        }

        /// Configure stats response
        pub fn with_stats(self, stats: &str) -> Self {
            *self.stats.lock().unwrap() = stats.to_string();
            self
        }

        /// Configure check result
        pub fn with_check_result(self, result: &str) -> Self {
            *self.check_result.lock().unwrap() = result.to_string();
            self
        }

        /// Configure list_snapshots to fail
        pub fn with_failing_list(self) -> Self {
            *self.should_fail_list.lock().unwrap() = true;
            self
        }

        /// Configure check to fail
        pub fn with_failing_check(self) -> Self {
            *self.should_fail_check.lock().unwrap() = true;
            self
        }

        /// Configure files for a specific snapshot
        pub fn with_snapshot_files(self, snapshot_id: &str, files: Vec<String>) -> Self {
            self.snapshot_files
                .lock()
                .unwrap()
                .insert(snapshot_id.to_string(), files);
            self
        }

        /// Get all recorded calls
        pub fn get_calls(&self) -> Vec<ResticCall> {
            self.calls.lock().unwrap().clone()
        }

        /// Check if init was called
        pub fn init_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, ResticCall::Init))
        }

        /// Check if backup was called
        pub fn backup_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, ResticCall::Backup { .. }))
        }

        /// Check if restore was called
        pub fn restore_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, ResticCall::Restore { .. }))
        }

        /// Check if check_repository was called
        pub fn check_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, ResticCall::Check { .. }))
        }

        /// Check if unlock was called
        pub fn unlock_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, ResticCall::Unlock))
        }

        fn record_call(&self, call: ResticCall) {
            self.calls.lock().unwrap().push(call);
        }
    }

    impl ResticOperations for MockResticOps {
        fn init_repository(&self, _env: &ResticEnv, _timeout: Duration) -> Result<()> {
            self.record_call(ResticCall::Init);
            if *self.should_fail_init.lock().unwrap() {
                anyhow::bail!("Mock init failure");
            }
            Ok(())
        }

        fn backup(
            &self,
            _env: &ResticEnv,
            paths: &[PathBuf],
            _excludes: &[String],
            _timeout: Duration,
        ) -> Result<()> {
            self.record_call(ResticCall::Backup {
                paths: paths.to_vec(),
            });
            if *self.should_fail_backup.lock().unwrap() {
                anyhow::bail!("Mock backup failure");
            }
            Ok(())
        }

        fn list_snapshots(&self, _env: &ResticEnv, _timeout: Duration) -> Result<Vec<Snapshot>> {
            self.record_call(ResticCall::ListSnapshots);
            if *self.should_fail_list.lock().unwrap() {
                anyhow::bail!("Mock list_snapshots failure");
            }
            Ok(self.snapshots.lock().unwrap().clone())
        }

        fn restore_snapshot(
            &self,
            _env: &ResticEnv,
            snapshot_id: &str,
            _target_dir: Option<&str>,
            _include_paths: &[String],
            _timeout: Duration,
        ) -> Result<()> {
            self.record_call(ResticCall::Restore {
                snapshot_id: snapshot_id.to_string(),
            });
            if *self.should_fail_restore.lock().unwrap() {
                anyhow::bail!("Mock restore failure");
            }
            Ok(())
        }

        fn apply_retention(
            &self,
            _env: &ResticEnv,
            _retention: &RetentionPolicy,
            _timeout: Duration,
        ) -> Result<()> {
            self.record_call(ResticCall::ApplyRetention);
            Ok(())
        }

        fn check_repository(
            &self,
            _env: &ResticEnv,
            read_data: bool,
            _timeout: Duration,
        ) -> Result<String> {
            self.record_call(ResticCall::Check { read_data });
            if *self.should_fail_check.lock().unwrap() {
                anyhow::bail!("Mock check failure");
            }
            Ok(self.check_result.lock().unwrap().clone())
        }

        fn unlock_repository(&self, _env: &ResticEnv, _timeout: Duration) -> Result<()> {
            self.record_call(ResticCall::Unlock);
            Ok(())
        }

        fn get_stats(&self, _env: &ResticEnv, _timeout: Duration) -> Result<String> {
            self.record_call(ResticCall::GetStats);
            Ok(self.stats.lock().unwrap().clone())
        }

        fn count_snapshots(&self, _env: &ResticEnv, _timeout: Duration) -> Result<usize> {
            self.record_call(ResticCall::CountSnapshots);
            Ok(self.snapshots.lock().unwrap().len())
        }

        fn get_latest_snapshot(
            &self,
            _env: &ResticEnv,
            _timeout: Duration,
        ) -> Result<Option<Snapshot>> {
            self.record_call(ResticCall::GetLatestSnapshot);
            Ok(self.snapshots.lock().unwrap().last().cloned())
        }

        fn list_snapshot_files(
            &self,
            _env: &ResticEnv,
            snapshot_id: &str,
            _timeout: Duration,
        ) -> Result<Vec<String>> {
            self.record_call(ResticCall::ListSnapshotFiles {
                snapshot_id: snapshot_id.to_string(),
            });
            // Return configured files if they exist, otherwise default
            let files = self.snapshot_files.lock().unwrap();
            if let Some(configured) = files.get(snapshot_id) {
                Ok(configured.clone())
            } else {
                Ok(vec![
                    "/data/file1.txt".to_string(),
                    "/data/file2.txt".to_string(),
                ])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_restic_ops_creation() {
        let ops = RealResticOps::new();
        let _ = ops;
    }

    #[test]
    fn test_mock_restic_ops_records_calls() {
        use mock::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let password_file = temp_dir.path().join("password");
        std::fs::write(&password_file, "test").unwrap();

        let mock = MockResticOps::new().with_snapshots(vec![Snapshot {
            id: "abc123".to_string(),
            short_id: "abc".to_string(),
            time: "2025-01-01T00:00:00Z".to_string(),
            hostname: "test".to_string(),
            paths: vec!["/data".to_string()],
        }]);

        let env = ResticEnv::new(&password_file, "/tmp/repo");
        let timeout = Duration::from_secs(30);

        mock.init_repository(&env, timeout).unwrap();
        mock.backup(&env, &[PathBuf::from("/data")], &[], timeout)
            .unwrap();
        let snapshots = mock.list_snapshots(&env, timeout).unwrap();

        assert!(mock.init_called());
        assert!(mock.backup_called());
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "abc123");
    }

    #[test]
    fn test_mock_restic_ops_failing_backup() {
        use mock::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let password_file = temp_dir.path().join("password");
        std::fs::write(&password_file, "test").unwrap();

        let mock = MockResticOps::new().with_failing_backup();
        let env = ResticEnv::new(&password_file, "/tmp/repo");
        let timeout = Duration::from_secs(30);

        let result = mock.backup(&env, &[PathBuf::from("/data")], &[], timeout);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mock backup failure"));
    }

    #[test]
    fn test_mock_restic_ops_stats() {
        use mock::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let password_file = temp_dir.path().join("password");
        std::fs::write(&password_file, "test").unwrap();

        let mock = MockResticOps::new().with_stats("2.5 GiB");
        let env = ResticEnv::new(&password_file, "/tmp/repo");
        let timeout = Duration::from_secs(30);

        let stats = mock.get_stats(&env, timeout).unwrap();
        assert_eq!(stats, "2.5 GiB");
    }
}
