//! Tests for the 'list' command
//!
//! The list command displays all configured services and their details.

use test_utils::ConfigBuilder;
use restic_manager::config::resolve_all_services;

#[test]
fn test_list_all_services() {
    let config = ConfigBuilder::minimal()
        .add_service("appwrite")
        .add_service("immich")
        .add_service("generic")
        .build();

    let resolved = resolve_all_services(&config).unwrap();

    assert_eq!(resolved.len(), 3);
    assert!(resolved.contains_key("appwrite"));
    assert!(resolved.contains_key("immich"));
    assert!(resolved.contains_key("generic"));
}

#[test]
fn test_list_shows_enabled_status() {
    let config = ConfigBuilder::minimal()
        .add_service("enabled-service")
        .add_disabled_service("disabled-service")
        .build();

    let resolved = resolve_all_services(&config).unwrap();

    assert!(resolved.get("enabled-service").unwrap().enabled);
    assert!(!resolved.get("disabled-service").unwrap().enabled);
}

#[test]
fn test_list_shows_targets() {
    let config = ConfigBuilder::minimal()
        .add_service("single-target")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("single-target").unwrap();

    // Service should have at least one target (local from minimal config)
    assert!(!service.targets.is_empty());
    assert!(service.targets.contains(&"local".to_string()));
}

#[test]
fn test_list_shows_schedule() {
    let config = ConfigBuilder::minimal().add_service("scheduled-service").build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("scheduled-service").unwrap();

    // Default schedule from ConfigBuilder
    assert_eq!(service.schedule, "0 2 * * *");
}

#[test]
fn test_list_empty_config() {
    let config = ConfigBuilder::minimal().build();

    let resolved = resolve_all_services(&config).unwrap();

    // No services configured (only destination from minimal)
    assert!(resolved.is_empty());
}

#[test]
fn test_list_with_paths_and_volumes() {
    let config = ConfigBuilder::minimal()
        .add_service_with_paths("files-service", vec!["data".to_string()])
        .add_service_with_volumes("docker-service", vec!["app_data".to_string()])
        .build();

    let resolved = resolve_all_services(&config).unwrap();

    let files_service = resolved.get("files-service").unwrap();
    let files_config = files_service.config.as_ref().unwrap();
    assert!(!files_config.paths.is_empty());

    let docker_service = resolved.get("docker-service").unwrap();
    let docker_config = docker_service.config.as_ref().unwrap();
    assert!(!docker_config.volumes.is_empty());
}

#[test]
fn test_list_service_descriptions() {
    let config = ConfigBuilder::minimal()
        .add_service("test-service")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("test-service").unwrap();

    // Should have a description
    assert!(!service.description.is_empty());
}

#[test]
fn test_list_service_timeout() {
    let config = ConfigBuilder::minimal()
        .with_timeout(7200)
        .add_service("long-running")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("long-running").unwrap();

    // Timeout should be inherited from global config
    assert_eq!(service.timeout_seconds, 7200);
}
