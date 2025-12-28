//! Common utilities for integration tests
//!
//! This module provides cleanup guards and helper functions for integration tests.

use std::process::Command;

/// Guard that ensures Docker container cleanup on drop (even on panic)
pub struct ContainerGuard {
    name: String,
}

impl ContainerGuard {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        cleanup_container(&self.name);
    }
}

/// Helper to stop and remove a Docker container
/// The -v flag also removes anonymous volumes associated with the container
fn cleanup_container(name: &str) {
    let _ = Command::new("docker").args(&["stop", name]).output();
    let _ = Command::new("docker").args(&["rm", "-v", name]).output();
}

/// Guard that ensures Docker volume cleanup on drop (even on panic)
pub struct VolumeGuard {
    name: String,
}

impl VolumeGuard {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Drop for VolumeGuard {
    fn drop(&mut self) {
        cleanup_volume(&self.name);
    }
}

/// Helper to cleanup a Docker volume
fn cleanup_volume(name: &str) {
    let _ = Command::new("docker")
        .args(&["volume", "rm", name])
        .output();
}
