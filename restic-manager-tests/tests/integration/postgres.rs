//! PostgreSQL integration tests
//!
//! These tests require Docker and verify PostgreSQL backup/restore workflows.
//! Run with: `cargo test -p restic-manager-tests --test integration -- --ignored`

use anyhow::Result;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to check if Docker is available
fn is_docker_available() -> bool {
    Command::new("docker")
        .args(&["ps"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Helper to start a PostgreSQL container
fn start_postgres_container(name: &str) -> Result<()> {
    Command::new("docker")
        .args(&[
            "run",
            "-d",
            "--name",
            name,
            "-e",
            "POSTGRES_PASSWORD=testpass",
            "-e",
            "POSTGRES_DB=testdb",
            "postgres:15-alpine",
        ])
        .output()?;

    // Wait for PostgreSQL to be ready
    thread::sleep(Duration::from_secs(5));

    // Check if ready
    for _ in 0..30 {
        let result = Command::new("docker")
            .args(&[
                "exec",
                name,
                "pg_isready",
                "-U",
                "postgres",
            ])
            .output();

        if result.map(|o| o.status.success()).unwrap_or(false) {
            return Ok(());
        }

        thread::sleep(Duration::from_secs(1));
    }

    Err(anyhow::anyhow!("PostgreSQL failed to become ready"))
}

/// Helper to stop and remove container
fn cleanup_container(name: &str) {
    let _ = Command::new("docker").args(&["stop", name]).output();
    let _ = Command::new("docker").args(&["rm", name]).output();
}

/// Helper to execute SQL in container
fn exec_sql(container: &str, sql: &str) -> Result<String> {
    let output = Command::new("docker")
        .args(&[
            "exec",
            container,
            "psql",
            "-U",
            "postgres",
            "-d",
            "testdb",
            "-t",
            "-c",
            sql,
        ])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Helper to create test data
fn create_test_data(container: &str) -> Result<()> {
    exec_sql(container, "CREATE TABLE test_table (id SERIAL PRIMARY KEY, data TEXT)")?;
    exec_sql(container, "INSERT INTO test_table (data) VALUES ('test1')")?;
    exec_sql(container, "INSERT INTO test_table (data) VALUES ('test2')")?;
    exec_sql(container, "INSERT INTO test_table (data) VALUES ('test3')")?;
    Ok(())
}

/// Helper to verify test data
fn verify_test_data(container: &str) -> Result<i32> {
    let result = exec_sql(container, "SELECT COUNT(*) FROM test_table")?;
    result.trim().parse::<i32>().map_err(|e| anyhow::anyhow!("Failed to parse count: {}", e))
}

/// Helper to dump database
fn dump_database(container: &str, output_path: &str) -> Result<()> {
    let output = Command::new("docker")
        .args(&[
            "exec",
            container,
            "pg_dump",
            "-U",
            "postgres",
            "testdb",
        ])
        .output()?;

    std::fs::write(output_path, &output.stdout)?;
    Ok(())
}

/// Test PostgreSQL backup with Docker
#[test]
#[ignore] // Requires Docker
fn test_postgres_backup_with_docker() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let container_name = "restic-test-postgres-backup";
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let dump_path = temp_dir.path().join("dump.sql");

    // Start PostgreSQL
    start_postgres_container(container_name).expect("Failed to start PostgreSQL");

    // Create test data
    create_test_data(container_name).expect("Failed to create test data");

    // Dump database
    dump_database(container_name, dump_path.to_str().unwrap())
        .expect("Failed to dump database");

    // Verify dump file exists and has content
    assert!(dump_path.exists(), "Dump file should exist");
    let dump_content = std::fs::read_to_string(&dump_path).expect("Failed to read dump");
    assert!(dump_content.contains("test_table"), "Dump should contain table");
    assert!(dump_content.contains("test1"), "Dump should contain data");

    // Cleanup
    cleanup_container(container_name);
}

/// Test PostgreSQL backup and restore cycle
#[test]
#[ignore] // Requires Docker
fn test_postgres_backup_restore_cycle() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let container_name = "restic-test-postgres-restore";
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let dump_path = temp_dir.path().join("dump.sql");

    // Start PostgreSQL
    start_postgres_container(container_name).expect("Failed to start PostgreSQL");

    // Create test data
    create_test_data(container_name).expect("Failed to create test data");
    let original_count = verify_test_data(container_name).expect("Failed to verify data");
    assert_eq!(original_count, 3, "Should have 3 rows");

    // Dump database
    dump_database(container_name, dump_path.to_str().unwrap())
        .expect("Failed to dump database");

    // Drop the table to simulate data loss
    exec_sql(container_name, "DROP TABLE test_table").expect("Failed to drop table");

    // Verify table is gone
    let result = exec_sql(container_name, "SELECT COUNT(*) FROM test_table");
    assert!(result.is_err() || !result.unwrap().contains("3"), "Table should be dropped");

    // Restore from dump
    let dump_content = std::fs::read_to_string(&dump_path).expect("Failed to read dump");
    let output = Command::new("docker")
        .args(&[
            "exec",
            "-i",
            container_name,
            "psql",
            "-U",
            "postgres",
            "-d",
            "testdb",
        ])
        .arg("-c")
        .arg(&dump_content)
        .output()
        .expect("Failed to restore database");

    assert!(output.status.success(), "Restore should succeed");

    // Verify data is restored
    let restored_count = verify_test_data(container_name).expect("Failed to verify restored data");
    assert_eq!(restored_count, 3, "Should have 3 rows after restore");

    // Cleanup
    cleanup_container(container_name);
}

/// Test PostgreSQL incremental backup
#[test]
#[ignore] // Requires Docker
fn test_postgres_incremental_backup() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let container_name = "restic-test-postgres-incremental";
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Start PostgreSQL
    start_postgres_container(container_name).expect("Failed to start PostgreSQL");

    // Create initial data
    create_test_data(container_name).expect("Failed to create test data");

    // First backup
    let dump1_path = temp_dir.path().join("dump1.sql");
    dump_database(container_name, dump1_path.to_str().unwrap())
        .expect("Failed to dump database");

    // Add more data
    exec_sql(container_name, "INSERT INTO test_table (data) VALUES ('test4')")
        .expect("Failed to insert data");
    exec_sql(container_name, "INSERT INTO test_table (data) VALUES ('test5')")
        .expect("Failed to insert data");

    // Second backup
    let dump2_path = temp_dir.path().join("dump2.sql");
    dump_database(container_name, dump2_path.to_str().unwrap())
        .expect("Failed to dump database");

    // Verify both dumps exist
    assert!(dump1_path.exists(), "First dump should exist");
    assert!(dump2_path.exists(), "Second dump should exist");

    // Verify second dump has more data
    let dump1_content = std::fs::read_to_string(&dump1_path).expect("Failed to read dump1");
    let dump2_content = std::fs::read_to_string(&dump2_path).expect("Failed to read dump2");

    assert!(dump1_content.contains("test3"), "First dump should have test3");
    assert!(!dump1_content.contains("test4"), "First dump should not have test4");
    assert!(dump2_content.contains("test4"), "Second dump should have test4");
    assert!(dump2_content.contains("test5"), "Second dump should have test5");

    // Cleanup
    cleanup_container(container_name);
}
