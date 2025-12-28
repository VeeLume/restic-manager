//! Integration tests for restic-manager
//!
//! These tests require Docker and test the full backup/restore workflow.
//! Run with: `cargo test -p restic-manager-tests --test integration -- --ignored`

mod postgres;
mod docker_volumes;
