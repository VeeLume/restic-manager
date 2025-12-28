//! Docker operations abstraction for testability
//!
//! This module provides a trait-based abstraction for Docker operations,
//! enabling dependency injection and mocking for tests.

#![allow(dead_code)]

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

/// Abstraction for Docker operations, enabling mocking in tests
pub trait DockerOperations: Send + Sync {
    /// List all Docker volumes
    fn list_volumes(&self, timeout: Duration) -> Result<Vec<String>>;

    /// Check if a Docker volume exists (exact match)
    fn volume_exists(&self, volume_name: &str, timeout: Duration) -> Result<bool>;

    /// Archive a Docker volume to a tar.gz file
    fn archive_volume(
        &self,
        volume_name: &str,
        output_path: &Path,
        timeout: Duration,
    ) -> Result<()>;

    /// Restore a Docker volume from a tar.gz file
    fn restore_volume(
        &self,
        volume_name: &str,
        archive_path: &Path,
        timeout: Duration,
    ) -> Result<()>;

    /// Get the size of a Docker volume in bytes
    fn get_volume_size(&self, volume_name: &str, timeout: Duration) -> Result<u64>;
}

/// Default implementation using real Docker CLI calls
#[derive(Debug, Clone, Default)]
pub struct RealDockerOps;

impl RealDockerOps {
    pub fn new() -> Self {
        Self
    }
}

impl DockerOperations for RealDockerOps {
    fn list_volumes(&self, timeout: Duration) -> Result<Vec<String>> {
        super::docker::list_volumes(timeout)
    }

    fn volume_exists(&self, volume_name: &str, timeout: Duration) -> Result<bool> {
        super::docker::volume_exists(volume_name, timeout)
    }

    fn archive_volume(
        &self,
        volume_name: &str,
        output_path: &Path,
        timeout: Duration,
    ) -> Result<()> {
        super::docker::archive_volume(volume_name, output_path, timeout)
    }

    fn restore_volume(
        &self,
        volume_name: &str,
        archive_path: &Path,
        timeout: Duration,
    ) -> Result<()> {
        super::docker::restore_volume(volume_name, archive_path, timeout)
    }

    fn get_volume_size(&self, volume_name: &str, timeout: Duration) -> Result<u64> {
        super::docker::get_volume_size(volume_name, timeout)
    }
}

/// Mock implementation for testing
/// Available for use in external test crates
#[allow(dead_code)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Recorded Docker operation call
    #[derive(Clone, Debug)]
    pub enum DockerCall {
        ListVolumes,
        VolumeExists { name: String },
        ArchiveVolume { name: String, path: String },
        RestoreVolume { name: String, path: String },
        GetVolumeSize { name: String },
    }

    /// Mock Docker operations for testing
    #[derive(Clone, Default)]
    pub struct MockDockerOps {
        /// Recorded operation calls
        pub calls: Arc<Mutex<Vec<DockerCall>>>,
        /// Pre-configured volumes
        pub volumes: Arc<Mutex<Vec<String>>>,
        /// Pre-configured volume sizes
        pub volume_sizes: Arc<Mutex<HashMap<String, u64>>>,
        /// Whether archive should fail
        pub should_fail_archive: Arc<Mutex<bool>>,
        /// Whether restore should fail
        pub should_fail_restore: Arc<Mutex<bool>>,
        /// Whether list_volumes should fail
        pub should_fail_list: Arc<Mutex<bool>>,
    }

    impl MockDockerOps {
        pub fn new() -> Self {
            Self::default()
        }

        /// Configure volumes to return
        pub fn with_volumes(self, volumes: Vec<String>) -> Self {
            *self.volumes.lock().unwrap() = volumes;
            self
        }

        /// Configure a volume size
        pub fn with_volume_size(self, name: &str, size: u64) -> Self {
            self.volume_sizes
                .lock()
                .unwrap()
                .insert(name.to_string(), size);
            self
        }

        /// Configure archive to fail
        pub fn with_failing_archive(self) -> Self {
            *self.should_fail_archive.lock().unwrap() = true;
            self
        }

        /// Configure restore to fail
        pub fn with_failing_restore(self) -> Self {
            *self.should_fail_restore.lock().unwrap() = true;
            self
        }

        /// Configure list_volumes to fail
        pub fn with_failing_list(self) -> Self {
            *self.should_fail_list.lock().unwrap() = true;
            self
        }

        /// Get all recorded calls
        pub fn get_calls(&self) -> Vec<DockerCall> {
            self.calls.lock().unwrap().clone()
        }

        /// Check if list_volumes was called
        pub fn list_volumes_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, DockerCall::ListVolumes))
        }

        /// Check if archive_volume was called
        pub fn archive_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, DockerCall::ArchiveVolume { .. }))
        }

        /// Check if restore_volume was called
        pub fn restore_called(&self) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, DockerCall::RestoreVolume { .. }))
        }

        /// Get archive calls with specific volume name
        pub fn archive_calls_for(&self, volume: &str) -> Vec<DockerCall> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| matches!(c, DockerCall::ArchiveVolume { name, .. } if name == volume))
                .cloned()
                .collect()
        }

        fn record_call(&self, call: DockerCall) {
            self.calls.lock().unwrap().push(call);
        }
    }

    impl DockerOperations for MockDockerOps {
        fn list_volumes(&self, _timeout: Duration) -> Result<Vec<String>> {
            self.record_call(DockerCall::ListVolumes);
            if *self.should_fail_list.lock().unwrap() {
                anyhow::bail!("Mock list_volumes failure");
            }
            Ok(self.volumes.lock().unwrap().clone())
        }

        fn volume_exists(&self, volume_name: &str, _timeout: Duration) -> Result<bool> {
            self.record_call(DockerCall::VolumeExists {
                name: volume_name.to_string(),
            });
            if *self.should_fail_list.lock().unwrap() {
                anyhow::bail!("Mock volume_exists failure");
            }
            // Use exact match, not substring
            Ok(self
                .volumes
                .lock()
                .unwrap()
                .iter()
                .any(|v| v == volume_name))
        }

        fn archive_volume(
            &self,
            volume_name: &str,
            output_path: &Path,
            _timeout: Duration,
        ) -> Result<()> {
            self.record_call(DockerCall::ArchiveVolume {
                name: volume_name.to_string(),
                path: output_path.display().to_string(),
            });
            if *self.should_fail_archive.lock().unwrap() {
                anyhow::bail!("Mock archive failure for volume {}", volume_name);
            }
            Ok(())
        }

        fn restore_volume(
            &self,
            volume_name: &str,
            archive_path: &Path,
            _timeout: Duration,
        ) -> Result<()> {
            self.record_call(DockerCall::RestoreVolume {
                name: volume_name.to_string(),
                path: archive_path.display().to_string(),
            });
            if *self.should_fail_restore.lock().unwrap() {
                anyhow::bail!("Mock restore failure for volume {}", volume_name);
            }
            Ok(())
        }

        fn get_volume_size(&self, volume_name: &str, _timeout: Duration) -> Result<u64> {
            self.record_call(DockerCall::GetVolumeSize {
                name: volume_name.to_string(),
            });
            Ok(*self
                .volume_sizes
                .lock()
                .unwrap()
                .get(volume_name)
                .unwrap_or(&1024))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_docker_ops_creation() {
        let ops = RealDockerOps::new();
        let _ = ops;
    }

    #[test]
    fn test_mock_docker_ops_list_volumes() {
        use mock::*;

        let mock = MockDockerOps::new().with_volumes(vec![
            "volume1".to_string(),
            "volume2".to_string(),
            "appwrite_appwrite-data".to_string(),
        ]);

        let timeout = Duration::from_secs(10);
        let volumes = mock.list_volumes(timeout).unwrap();

        assert!(mock.list_volumes_called());
        assert_eq!(volumes.len(), 3);
        assert!(volumes.contains(&"volume1".to_string()));
    }

    #[test]
    fn test_mock_docker_ops_volume_exists_exact_match() {
        use mock::*;

        let mock = MockDockerOps::new().with_volumes(vec![
            "appwrite_appwrite-data".to_string(),
            "other-volume".to_string(),
        ]);

        let timeout = Duration::from_secs(10);

        // Exact match should work
        assert!(mock.volume_exists("appwrite_appwrite-data", timeout).unwrap());
        assert!(mock.volume_exists("other-volume", timeout).unwrap());

        // Substring should NOT match (this is important for Appwrite!)
        assert!(!mock.volume_exists("appwrite", timeout).unwrap());
        assert!(!mock.volume_exists("data", timeout).unwrap());
    }

    #[test]
    fn test_mock_docker_ops_archive() {
        use mock::*;
        use std::path::PathBuf;

        let mock = MockDockerOps::new();
        let timeout = Duration::from_secs(60);
        let path = PathBuf::from("/tmp/archive.tar.gz");

        mock.archive_volume("my-volume", &path, timeout).unwrap();

        assert!(mock.archive_called());
        let calls = mock.archive_calls_for("my-volume");
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn test_mock_docker_ops_failing_archive() {
        use mock::*;
        use std::path::PathBuf;

        let mock = MockDockerOps::new().with_failing_archive();
        let timeout = Duration::from_secs(60);
        let path = PathBuf::from("/tmp/archive.tar.gz");

        let result = mock.archive_volume("my-volume", &path, timeout);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mock archive failure"));
    }

    #[test]
    fn test_mock_docker_ops_volume_size() {
        use mock::*;

        let mock = MockDockerOps::new()
            .with_volumes(vec!["large-volume".to_string()])
            .with_volume_size("large-volume", 1024 * 1024 * 100); // 100 MiB

        let timeout = Duration::from_secs(10);
        let size = mock.get_volume_size("large-volume", timeout).unwrap();

        assert_eq!(size, 104857600);
    }

    #[test]
    fn test_mock_docker_ops_restore() {
        use mock::*;
        use std::path::PathBuf;

        let mock = MockDockerOps::new();
        let timeout = Duration::from_secs(60);
        let path = PathBuf::from("/tmp/archive.tar.gz");

        mock.restore_volume("my-volume", &path, timeout).unwrap();

        assert!(mock.restore_called());
    }
}
