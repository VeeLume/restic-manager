//! Test context and harness for integration testing
//!
//! Provides a unified context for setting up and tearing down test environments.

use crate::config_builder::ConfigBuilder;
use anyhow::Result;
use restic_manager::config::Config;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test context that manages test resources and provides common utilities
pub struct TestContext {
    /// Temporary directory for test files
    temp_dir: TempDir,
    /// The test configuration
    config: Option<Config>,
}

impl TestContext {
    /// Create a new test context with a temporary directory
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp dir"),
            config: None,
        }
    }

    /// Create a test context with a minimal configuration
    pub fn with_minimal_config() -> Self {
        let builder = ConfigBuilder::minimal();
        let (config, temp_dir) = builder.persist();

        Self {
            temp_dir,
            config: Some(config),
        }
    }

    /// Create a test context from a ConfigBuilder
    pub fn from_builder(builder: ConfigBuilder) -> Self {
        let (config, temp_dir) = builder.persist();

        Self {
            temp_dir,
            config: Some(config),
        }
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get the configuration
    pub fn config(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    /// Create a subdirectory in the temp dir
    pub fn create_subdir(&self, name: &str) -> PathBuf {
        let path = self.temp_dir.path().join(name);
        std::fs::create_dir_all(&path).expect("Failed to create subdirectory");
        path
    }

    /// Create a file in the temp dir
    pub fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.temp_dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        std::fs::write(&path, content).expect("Failed to write file");
        path
    }

    /// Create a password file with the given content
    pub fn create_password_file(&self, password: &str) -> PathBuf {
        self.create_file("restic-password", password)
    }

    /// Check if a file exists in the temp directory
    pub fn file_exists(&self, name: &str) -> bool {
        self.temp_dir.path().join(name).exists()
    }

    /// Read a file from the temp directory
    pub fn read_file(&self, name: &str) -> Result<String> {
        let path = self.temp_dir.path().join(name);
        Ok(std::fs::read_to_string(path)?)
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for assertion helpers
pub trait ResultAssertions<T> {
    /// Assert that the result is Ok and return the value
    fn assert_ok(self) -> T;

    /// Assert that the result is Ok with a custom message
    fn assert_ok_msg(self, msg: &str) -> T;

    /// Assert that the result is Err
    fn assert_err(self);

    /// Assert that the result is Err and the error message contains the given string
    fn assert_err_contains(self, needle: &str);
}

impl<T: std::fmt::Debug, E: std::fmt::Debug> ResultAssertions<T> for Result<T, E> {
    fn assert_ok(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    }

    fn assert_ok_msg(self, msg: &str) -> T {
        match self {
            Ok(v) => v,
            Err(e) => panic!("{}: {:?}", msg, e),
        }
    }

    fn assert_err(self) {
        if self.is_ok() {
            panic!("Expected Err, got Ok: {:?}", self.unwrap());
        }
    }

    fn assert_err_contains(self, needle: &str) {
        match self {
            Ok(v) => panic!("Expected Err containing '{}', got Ok: {:?}", needle, v),
            Err(e) => {
                let err_msg = format!("{:?}", e);
                assert!(
                    err_msg.contains(needle),
                    "Error '{}' does not contain '{}'",
                    err_msg,
                    needle
                );
            }
        }
    }
}

/// Extension trait for Option assertions
pub trait OptionAssertions<T> {
    /// Assert that the option is Some and return the value
    fn assert_some(self) -> T;

    /// Assert that the option is None
    fn assert_none(self);
}

impl<T: std::fmt::Debug> OptionAssertions<T> for Option<T> {
    fn assert_some(self) -> T {
        match self {
            Some(v) => v,
            None => panic!("Expected Some, got None"),
        }
    }

    fn assert_none(self) {
        if let Some(v) = self {
            panic!("Expected None, got Some: {:?}", v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context() {
        let ctx = TestContext::new();
        assert!(ctx.temp_dir().exists());
    }

    #[test]
    fn test_create_subdir() {
        let ctx = TestContext::new();
        let subdir = ctx.create_subdir("test-subdir");
        assert!(subdir.exists());
        assert!(subdir.is_dir());
    }

    #[test]
    fn test_create_file() {
        let ctx = TestContext::new();
        let file = ctx.create_file("test.txt", "hello world");
        assert!(file.exists());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello world");
    }

    #[test]
    fn test_nested_file() {
        let ctx = TestContext::new();
        let file = ctx.create_file("nested/deep/file.txt", "content");
        assert!(file.exists());
    }

    #[test]
    fn test_result_assertions() {
        let ok_result: Result<i32, &str> = Ok(42);
        assert_eq!(ok_result.assert_ok(), 42);

        let err_result: Result<i32, &str> = Err("error message");
        err_result.assert_err_contains("error");
    }

    #[test]
    fn test_option_assertions() {
        let some_opt = Some(42);
        assert_eq!(some_opt.assert_some(), 42);

        let none_opt: Option<i32> = None;
        none_opt.assert_none();
    }
}
