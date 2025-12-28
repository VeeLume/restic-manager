//! File-based locking to prevent concurrent backups

use anyhow::{Context, Result};
use fd_lock::RwLock;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Lock guard for a service backup
pub struct BackupLock {
    // Store the lock and file together
    _lock: Box<(RwLock<File>, Option<fd_lock::RwLockWriteGuard<'static, File>>)>,
    lock_path: PathBuf,
}

impl BackupLock {
    /// Acquire an exclusive lock for a service
    /// Returns error if the service is already being backed up
    pub fn acquire(service_name: &str) -> Result<Self> {
        let lock_path = Self::lock_path(service_name);

        debug!("Attempting to acquire lock: {:?}", lock_path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create lock directory")?;
        }

        // Open or create the lock file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)
            .context(format!("Failed to open lock file: {:?}", lock_path))?;

        // Create boxed lock
        let mut boxed_lock = Box::new((RwLock::new(file), None));

        // SAFETY: We're creating a self-referential structure here.
        // The lock guard references the RwLock, which is stored in the same Box.
        // This is safe because:
        // 1. The Box won't move once created
        // 2. The guard and RwLock will be dropped together
        // 3. The guard is dropped before the RwLock in the tuple drop order
        let lock_ptr = &mut boxed_lock.0 as *mut RwLock<File>;
        let guard = unsafe { (*lock_ptr).try_write() }
            .context(format!(
                "Service '{}' is already being backed up (lock held)",
                service_name
            ))?;

        // Store the guard - casting to 'static is safe because we control the lifetime
        let static_guard: fd_lock::RwLockWriteGuard<'static, File> = unsafe { std::mem::transmute(guard) };
        boxed_lock.1 = Some(static_guard);

        info!("Acquired backup lock for service: {}", service_name);

        Ok(Self {
            _lock: boxed_lock,
            lock_path,
        })
    }

    /// Get the lock file path for a service
    fn lock_path(service_name: &str) -> PathBuf {
        #[cfg(unix)]
        let base = Path::new("/tmp");

        #[cfg(windows)]
        let base = std::env::temp_dir();

        base.join(format!("restic-manager-{}.lock", service_name))
    }

    /// Get the lock file path (for cleanup or inspection)
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for BackupLock {
    fn drop(&mut self) {
        info!("Released backup lock: {:?}", self.lock_path);

        // Try to remove the lock file (best effort)
        if let Err(e) = std::fs::remove_file(&self.lock_path) {
            debug!("Failed to remove lock file: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_acquire_and_release() {
        let service = "test-service";

        // Acquire lock
        let lock = BackupLock::acquire(service).expect("Failed to acquire lock");
        assert!(lock.path().exists());

        // Try to acquire again (should fail)
        let result = BackupLock::acquire(service);
        assert!(result.is_err());

        // Drop lock
        drop(lock);

        // Should be able to acquire again
        let lock2 = BackupLock::acquire(service).expect("Failed to acquire lock after release");
        drop(lock2);
    }
}
