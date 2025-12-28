//! Command execution abstraction for testability
//!
//! This module provides a trait-based abstraction for command execution,
//! enabling dependency injection and mocking for tests.

#![allow(dead_code)]

use anyhow::Result;
use std::path::Path;
use std::process::Output;
use std::time::Duration;

/// Abstraction for command execution, enabling mocking in tests
pub trait CommandExecutor: Send + Sync {
    /// Run a command with optional timeout
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        timeout: Option<Duration>,
    ) -> Result<Output>;

    /// Run a command and return stdout as string
    fn run_command_stdout(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        timeout: Option<Duration>,
    ) -> Result<String>;

    /// Run a shell command (useful for hooks)
    fn run_shell_command(
        &self,
        command: &str,
        working_dir: Option<&Path>,
        timeout: Option<Duration>,
    ) -> Result<Output>;
}

/// Default implementation using real subprocess calls
#[derive(Debug, Clone, Default)]
pub struct RealExecutor;

impl RealExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl CommandExecutor for RealExecutor {
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        timeout: Option<Duration>,
    ) -> Result<Output> {
        super::command::run_command(program, args, working_dir, timeout)
    }

    fn run_command_stdout(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        timeout: Option<Duration>,
    ) -> Result<String> {
        super::command::run_command_stdout(program, args, working_dir, timeout)
    }

    fn run_shell_command(
        &self,
        command: &str,
        working_dir: Option<&Path>,
        timeout: Option<Duration>,
    ) -> Result<Output> {
        super::command::run_shell_command(command, working_dir, timeout)
    }
}

/// A mock executor for testing that records calls and returns configured responses
/// Available for use in external test crates
#[allow(dead_code)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Recorded command invocation
    #[derive(Clone, Debug)]
    pub struct CommandCall {
        pub program: String,
        pub args: Vec<String>,
        pub working_dir: Option<String>,
    }

    /// Response configuration for mock
    #[derive(Clone, Debug)]
    pub enum MockResponse {
        Success { stdout: String, stderr: String },
        Failure { stderr: String, exit_code: i32 },
        Timeout,
    }

    impl Default for MockResponse {
        fn default() -> Self {
            MockResponse::Success {
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    /// Mock executor for testing
    #[derive(Clone, Default)]
    pub struct MockExecutor {
        /// Recorded command invocations
        pub calls: Arc<Mutex<Vec<CommandCall>>>,
        /// Pre-configured responses: program name -> response
        responses: Arc<Mutex<HashMap<String, MockResponse>>>,
        /// Default response when no specific response is configured
        default_response: Arc<Mutex<MockResponse>>,
    }

    impl MockExecutor {
        pub fn new() -> Self {
            Self::default()
        }

        /// Configure a response for a specific program
        pub fn expect(self, program: &str, response: MockResponse) -> Self {
            self.responses
                .lock()
                .unwrap()
                .insert(program.to_string(), response);
            self
        }

        /// Set the default response for unconfigured programs
        pub fn with_default_response(self, response: MockResponse) -> Self {
            *self.default_response.lock().unwrap() = response;
            self
        }

        /// Get all recorded calls
        pub fn get_calls(&self) -> Vec<CommandCall> {
            self.calls.lock().unwrap().clone()
        }

        /// Check if a program was called
        pub fn was_called(&self, program: &str) -> bool {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| c.program == program)
        }

        /// Get number of calls to a specific program
        pub fn call_count(&self, program: &str) -> usize {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.program == program)
                .count()
        }

        fn record_call(&self, program: &str, args: &[&str], working_dir: Option<&Path>) {
            self.calls.lock().unwrap().push(CommandCall {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                working_dir: working_dir.map(|p| p.display().to_string()),
            });
        }

        fn get_response(&self, program: &str) -> MockResponse {
            self.responses
                .lock()
                .unwrap()
                .get(program)
                .cloned()
                .unwrap_or_else(|| self.default_response.lock().unwrap().clone())
        }

        fn execute_response(&self, response: MockResponse) -> Result<Output> {
            match response {
                MockResponse::Success { stdout, stderr } => Ok(Output {
                    status: std::process::ExitStatus::default(),
                    stdout: stdout.into_bytes(),
                    stderr: stderr.into_bytes(),
                }),
                MockResponse::Failure { stderr, exit_code } => {
                    anyhow::bail!("Command failed with exit code {:?}: {}", exit_code, stderr)
                }
                MockResponse::Timeout => {
                    anyhow::bail!("Command timed out")
                }
            }
        }
    }

    impl CommandExecutor for MockExecutor {
        fn run_command(
            &self,
            program: &str,
            args: &[&str],
            working_dir: Option<&Path>,
            _timeout: Option<Duration>,
        ) -> Result<Output> {
            self.record_call(program, args, working_dir);
            let response = self.get_response(program);
            self.execute_response(response)
        }

        fn run_command_stdout(
            &self,
            program: &str,
            args: &[&str],
            working_dir: Option<&Path>,
            timeout: Option<Duration>,
        ) -> Result<String> {
            let output = self.run_command(program, args, working_dir, timeout)?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }

        fn run_shell_command(
            &self,
            command: &str,
            working_dir: Option<&Path>,
            timeout: Option<Duration>,
        ) -> Result<Output> {
            #[cfg(unix)]
            let (shell, flag) = ("sh", "-c");
            #[cfg(windows)]
            let (shell, flag) = ("cmd", "/C");

            self.run_command(shell, &[flag, command], working_dir, timeout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_executor_creation() {
        let executor = RealExecutor::new();
        // Just verify it can be created
        let _ = executor;
    }

    #[test]
    fn test_mock_executor_records_calls() {
        use mock::*;

        let executor = MockExecutor::new().with_default_response(MockResponse::Success {
            stdout: "output".to_string(),
            stderr: String::new(),
        });

        let _ = executor.run_command("test-program", &["arg1", "arg2"], None, None);

        assert!(executor.was_called("test-program"));
        assert_eq!(executor.call_count("test-program"), 1);

        let calls = executor.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].program, "test-program");
        assert_eq!(calls[0].args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_mock_executor_configured_response() {
        use mock::*;

        let executor = MockExecutor::new().expect(
            "my-program",
            MockResponse::Success {
                stdout: "expected output".to_string(),
                stderr: String::new(),
            },
        );

        let result = executor.run_command_stdout("my-program", &[], None, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "expected output");
    }

    #[test]
    fn test_mock_executor_failure_response() {
        use mock::*;

        let executor = MockExecutor::new().expect(
            "failing-program",
            MockResponse::Failure {
                stderr: "error message".to_string(),
                exit_code: 1,
            },
        );

        let result = executor.run_command("failing-program", &[], None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("error message"));
    }
}
