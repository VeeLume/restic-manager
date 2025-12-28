mod config;
mod managers;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use managers::backup::BackupManager;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "restic-manager")]
#[command(about = "Backup orchestration tool wrapping restic", long_about = None)]
#[command(version)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "/home/valerie/backup-config.toml")]
    config: PathBuf,

    /// Use system restic from PATH instead of managed binary
    #[arg(long)]
    use_system_restic: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run backups for all enabled services or a specific service
    Run {
        /// Specific service to backup (defaults to all enabled services)
        #[arg(short, long)]
        service: Option<String>,
    },

    /// Restore a service from backup
    Restore {
        /// Service to restore
        #[arg(short, long)]
        service: String,

        /// Specific snapshot ID to restore (interactive if not specified)
        #[arg(long)]
        snapshot: Option<String>,

        /// Destination to restore from
        #[arg(short, long)]
        destination: Option<String>,

        /// Target directory for restoration (defaults to service's original paths)
        #[arg(short, long)]
        target: Option<String>,

        /// Restore specific paths only (can be used multiple times)
        #[arg(long)]
        path: Vec<String>,
    },

    /// Show status and health of all services
    Status {
        /// Specific service to check
        #[arg(short, long)]
        service: Option<String>,
    },

    /// List all configured services
    List,

    /// Show available snapshots for a service
    Snapshots {
        /// Service name
        #[arg(short, long)]
        service: String,

        /// Optional destination filter
        #[arg(short, long)]
        destination: Option<String>,
    },

    /// Verify repository integrity
    Verify {
        /// Specific service to verify
        #[arg(short, long)]
        service: Option<String>,

        /// Perform deep verification (reads all data - slower)
        #[arg(long)]
        read_data: bool,
    },

    /// Initialize directories and setup cron jobs
    Setup {
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,

        /// Only setup cron jobs, skip directory initialization
        #[arg(long)]
        cron_only: bool,

        /// Only initialize directories, skip cron setup
        #[arg(long)]
        dirs_only: bool,
    },

    /// Validate configuration file
    Validate,

    /// Setup restic binary (download if needed)
    SetupRestic,

    /// Update restic binary to latest version
    UpdateRestic,

    /// Show restic version
    ResticVersion,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Commands that don't require a config file - use simple console logging
    match &cli.command {
        Some(Commands::SetupRestic) => {
            managers::logging::init_console_logging();
            return handle_setup_restic();
        }
        Some(Commands::UpdateRestic) => {
            managers::logging::init_console_logging();
            return handle_update_restic(cli.use_system_restic);
        }
        Some(Commands::ResticVersion) => {
            managers::logging::init_console_logging();
            return handle_restic_version(cli.use_system_restic);
        }
        _ => {
            // All other commands require config and full logging
        }
    }

    // Load and validate configuration (needed for use_system_restic setting)
    let config = config::load_config(&cli.config)?;
    let resolved_services = config::resolve_all_services(&config)?;

    // Setup logging with file rotation (must keep guard alive)
    let logging_config = managers::logging::LoggingConfig::from_config(
        &config.global.log_directory,
        &config.global.log_level,
        config.global.log_max_files,
        config.global.log_max_size_mb,
    );
    let _log_guard = managers::logging::init_logging(&logging_config)?;

    // Determine if we should use system restic (CLI arg overrides config)
    let use_system_restic = cli.use_system_restic || config.global.use_system_restic;

    // Set global flag for restic operations
    utils::restic::set_use_system_restic(use_system_restic);

    // Ensure restic is available (except for validate command)
    match cli.command {
        Some(Commands::Validate) => {
            // Skip restic check for validate
        }
        _ => {
            // For all other commands, ensure restic is available
            if !utils::restic_installer::restic_exists(use_system_restic) {
                if use_system_restic {
                    eprintln!("⚠️  System restic not found in PATH!");
                    eprintln!();
                    eprintln!("You have use_system_restic enabled, but restic is not installed.");
                    eprintln!("Either:");
                    eprintln!("  1. Install restic system-wide, or");
                    eprintln!("  2. Run: restic-manager setup-restic");
                    eprintln!("     and set use_system_restic = false");
                    eprintln!();
                } else {
                    eprintln!("⚠️  Restic binary not found!");
                    eprintln!();
                    eprintln!("Restic is required for backup operations.");
                    eprintln!("Run the following command to download restic:");
                    eprintln!();
                    eprintln!("  restic-manager setup-restic");
                    eprintln!();
                    eprintln!("Or set use_system_restic = true in config to use system restic.");
                    eprintln!();
                }
                std::process::exit(1);
            }
        }
    }

    // Create backup manager
    let backup_manager = BackupManager::new(config.clone(), resolved_services.clone());

    // If no command specified, show status overview
    let command = cli.command.unwrap_or(Commands::Status { service: None });

    match command {
        Commands::Run { service } => {
            if let Some(service_name) = service {
                println!("Running backup for service: {}", service_name);
                backup_manager.backup_service(&service_name)?;
                println!("✓ Backup completed successfully");
            } else {
                println!("Running backups for all enabled services...");
                backup_manager.backup_all()?;
                println!("✓ All backups completed successfully");
            }
        }

        Commands::Restore { service, snapshot, destination, target, path } => {
            use dialoguer::{Confirm, Select};

            // Get the service configuration
            let service_config = resolved_services.get(&service).ok_or_else(|| {
                anyhow::anyhow!("Service '{}' not found in configuration", service)
            })?;

            println!("=== Restore Service: {} ===\n", service);

            // Determine which destination to use
            let dest_name = if let Some(ref d) = destination {
                if !service_config.targets.contains(d) {
                    eprintln!("Error: Service '{}' does not use destination '{}'", service, d);
                    eprintln!("Available destinations: {}", service_config.targets.join(", "));
                    std::process::exit(1);
                }
                d.clone()
            } else if service_config.targets.len() == 1 {
                service_config.targets[0].clone()
            } else {
                // Interactive destination selection
                println!("Multiple destinations available. Select one:");
                let selection = Select::new()
                    .items(&service_config.targets)
                    .default(0)
                    .interact()?;
                service_config.targets[selection].clone()
            };

            let dest = config.destinations.get(&dest_name).ok_or_else(|| {
                anyhow::anyhow!("Destination '{}' not found", dest_name)
            })?;

            println!("Using destination: {} ({})\n", dest_name, dest.url);

            // Build repository URL and environment
            let repo_url = utils::restic::build_repository_url(dest, &service, None);
            let password_file = &config.global.restic_password_file;
            let env = utils::restic::ResticEnv::new(password_file, &repo_url);

            // Get snapshots
            let snapshots = utils::restic::list_snapshots(&env, std::time::Duration::from_secs(60))?;

            if snapshots.is_empty() {
                eprintln!("No snapshots found for service '{}'", service);
                std::process::exit(1);
            }

            // Determine which snapshot to restore
            let snapshot_id = if let Some(ref snap_id) = snapshot {
                // Verify snapshot exists
                if !snapshots.iter().any(|s| s.id.starts_with(snap_id) || s.short_id == *snap_id) {
                    eprintln!("Snapshot '{}' not found", snap_id);
                    std::process::exit(1);
                }
                snap_id.clone()
            } else {
                // Interactive snapshot selection
                println!("Available snapshots:");
                let items: Vec<String> = snapshots.iter().map(|s| {
                    let date = if let Some(date_part) = s.time.split('T').next() {
                        let time_part = s.time.split('T').nth(1)
                            .and_then(|t| t.split('.').next())
                            .unwrap_or("");
                        format!("{} {}", date_part, time_part)
                    } else {
                        s.time.clone()
                    };
                    format!("{} - {} ({})", s.short_id, date, s.hostname)
                }).collect();

                let selection = Select::new()
                    .with_prompt("Select snapshot to restore")
                    .items(&items)
                    .default(items.len() - 1) // Default to most recent
                    .interact()?;

                snapshots[selection].short_id.clone()
            };

            println!("\nSelected snapshot: {}\n", snapshot_id);

            // Show what will be restored
            println!("Preview of snapshot contents:");
            match utils::restic::list_snapshot_files(&env, &snapshot_id, std::time::Duration::from_secs(30)) {
                Ok(files) => {
                    let preview_count = 10;
                    for file in files.iter().take(preview_count) {
                        println!("  {}", file);
                    }
                    if files.len() > preview_count {
                        println!("  ... and {} more files", files.len() - preview_count);
                    }
                    println!("\nTotal: {} items", files.len());
                }
                Err(e) => {
                    eprintln!("Warning: Could not list snapshot contents: {}", e);
                }
            }

            // Determine target directory
            let target_dir = if let Some(ref t) = target {
                Some(t.as_str())
            } else {
                println!("\nNo target directory specified.");
                println!("Restore will overwrite original locations!");
                None
            };

            if let Some(t) = target_dir {
                println!("\nRestore target: {}", t);
            } else {
                println!("\nRestore target: Original locations (IN-PLACE)");
            }

            if !path.is_empty() {
                println!("Restoring specific paths:");
                for p in &path {
                    println!("  {}", p);
                }
            }

            println!();

            // Confirmation
            let confirm = Confirm::new()
                .with_prompt("Do you want to proceed with the restore?")
                .default(false)
                .interact()?;

            if !confirm {
                println!("Restore cancelled.");
                std::process::exit(0);
            }

            println!("\nStarting restore...\n");

            // Perform restore with longer timeout (30 minutes)
            let restore_timeout = std::time::Duration::from_secs(1800);

            match utils::restic::restore_snapshot(
                &env,
                &snapshot_id,
                target_dir,
                &path,
                restore_timeout,
            ) {
                Ok(()) => {
                    println!("\n✓ Restore completed successfully!");
                    if let Some(t) = target_dir {
                        println!("Files restored to: {}", t);
                    } else {
                        println!("Files restored to original locations");
                    }
                }
                Err(e) => {
                    eprintln!("\n✗ Restore failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Status { service } => {
            if let Some(service_name) = service {
                // Get the service configuration
                let service_config = resolved_services.get(&service_name).ok_or_else(|| {
                    anyhow::anyhow!("Service '{}' not found in configuration", service_name)
                })?;

                println!("=== Status for service: {} ===\n", service_name);
                println!("Description: {}", service_config.description);
                println!("Enabled: {}", if service_config.enabled { "Yes" } else { "No" });
                println!("Schedule: {}", service_config.schedule);
                println!("Timeout: {} seconds", service_config.timeout_seconds);
                println!("Targets: {}", service_config.targets.join(", "));
                println!();

                // Show status for each destination
                for target_name in &service_config.targets {
                    let destination = config.destinations.get(target_name).ok_or_else(|| {
                        anyhow::anyhow!("Destination '{}' not found", target_name)
                    })?;

                    println!("Destination: {}", target_name);
                    println!("  Repository: {}", destination.url);

                    // Build repository URL
                    let repo_url = utils::restic::build_repository_url(destination, &service_name, None);
                    let password_file = &config.global.restic_password_file;
                    let env = utils::restic::ResticEnv::new(password_file, &repo_url);

                    // Get snapshot count
                    match utils::restic::count_snapshots(&env, std::time::Duration::from_secs(30)) {
                        Ok(count) => {
                            println!("  Snapshots: {}", count);

                            if count > 0 {
                                // Get latest snapshot
                                if let Ok(Some(latest)) = utils::restic::get_latest_snapshot(&env, std::time::Duration::from_secs(30)) {
                                    let date_str = if let Some(date_part) = latest.time.split('T').next() {
                                        let time_part = latest.time.split('T').nth(1)
                                            .and_then(|t| t.split('.').next())
                                            .unwrap_or("");
                                        format!("{} {}", date_part, time_part)
                                    } else {
                                        latest.time.clone()
                                    };

                                    println!("  Last Backup: {}", date_str);

                                    // Calculate age and health
                                    if let Ok(snapshot_time) = chrono::DateTime::parse_from_rfc3339(&latest.time) {
                                        let now = chrono::Utc::now();
                                        let age = now.signed_duration_since(snapshot_time);
                                        let hours = age.num_hours();

                                        println!("  Age: {} hours ago", hours);

                                        // Health indicator based on age
                                        let health = if hours < 24 {
                                            "✓ Healthy (recent backup)"
                                        } else if hours < 48 {
                                            "⚠ Warning (backup is 1-2 days old)"
                                        } else {
                                            "✗ Critical (backup is over 2 days old)"
                                        };
                                        println!("  Health: {}", health);
                                    }
                                }

                                // Get repository size
                                if let Ok(size) = utils::restic::get_stats(&env, std::time::Duration::from_secs(30)) {
                                    println!("  Repository Size: {}", size);
                                }
                            } else {
                                println!("  Health: ✗ No backups found");
                            }
                        }
                        Err(e) => {
                            eprintln!("  ✗ Failed to get status: {}", e);
                        }
                    }

                    println!();
                }
            } else {
                println!("=== Backup Status Overview ===\n");
                println!("Services configured: {}", resolved_services.len());
                println!("Destinations: {}", config.destinations.len());
                println!("\nServices:");
                for (name, svc) in &resolved_services {
                    let status = if svc.enabled { "enabled" } else { "disabled" };
                    println!(
                        "  {} - {} ({})",
                        name,
                        svc.description,
                        status
                    );
                }
            }
        }

        Commands::List => {
            println!("Configured services:");
            for (name, svc) in &resolved_services {
                println!("  {}", name);
                println!("    Description: {}", svc.description);
                println!("    Enabled: {}", svc.enabled);
                println!("    Schedule: {}", svc.schedule);
                println!("    Targets: {}", svc.targets.join(", "));
                println!();
            }
        }

        Commands::Snapshots { service, destination } => {
            // Get the service configuration
            let service_config = resolved_services.get(&service).ok_or_else(|| {
                anyhow::anyhow!("Service '{}' not found in configuration", service)
            })?;

            println!("=== Snapshots for service: {} ===\n", service);

            // Filter targets if destination is specified
            let targets: Vec<String> = if let Some(ref dest) = destination {
                if service_config.targets.contains(dest) {
                    vec![dest.clone()]
                } else {
                    eprintln!("Error: Service '{}' does not use destination '{}'", service, dest);
                    eprintln!("Available destinations: {}", service_config.targets.join(", "));
                    std::process::exit(1);
                }
            } else {
                service_config.targets.clone()
            };

            // List snapshots for each destination
            for target_name in &targets {
                let destination = config.destinations.get(target_name).ok_or_else(|| {
                    anyhow::anyhow!("Destination '{}' not found", target_name)
                })?;

                println!("Destination: {}", target_name);
                println!("Repository: {}\n", destination.url);

                // Build repository URL
                let repo_url = utils::restic::build_repository_url(destination, &service, None);

                // Get password file (destination-specific or global)
                let password_file = destination.url.contains("sftp://")
                    .then(|| config.global.restic_password_file.clone())
                    .unwrap_or_else(|| config.global.restic_password_file.clone());

                // Create restic environment
                let env = utils::restic::ResticEnv::new(&password_file, &repo_url);

                // List snapshots
                match utils::restic::list_snapshots(&env, std::time::Duration::from_secs(60)) {
                    Ok(snapshots) => {
                        if snapshots.is_empty() {
                            println!("  No snapshots found.\n");
                        } else {
                            // Print table header
                            println!("  {:<10} {:<20} {:<15}", "ID", "Date", "Hostname");
                            println!("  {}", "-".repeat(50));

                            // Print snapshots
                            for snapshot in &snapshots {
                                // Parse and format the timestamp
                                let date_str = if let Some(date_part) = snapshot.time.split('T').next() {
                                    // Extract time part too
                                    let time_part = snapshot.time.split('T').nth(1)
                                        .and_then(|t| t.split('.').next())
                                        .unwrap_or("");
                                    format!("{} {}", date_part, time_part)
                                } else {
                                    snapshot.time.clone()
                                };

                                println!(
                                    "  {:<10} {:<20} {:<15}",
                                    &snapshot.short_id,
                                    date_str,
                                    &snapshot.hostname
                                );
                            }

                            println!("\n  Total: {} snapshots", snapshots.len());

                            // Get repository stats
                            if let Ok(size) = utils::restic::get_stats(&env, std::time::Duration::from_secs(30)) {
                                println!("  Repository size: {}", size);
                            }

                            println!();
                        }
                    }
                    Err(e) => {
                        eprintln!("  ✗ Failed to list snapshots: {}\n", e);
                    }
                }
            }
        }

        Commands::Verify { service, read_data } => {
            println!("=== Verifying Repositories ===\n");

            if read_data {
                println!("⚠ Deep verification enabled (this will take longer)\n");
            }

            // Determine which services to verify
            let services_to_verify: Vec<_> = if let Some(ref service_name) = service {
                let service_config = resolved_services.get(service_name).ok_or_else(|| {
                    anyhow::anyhow!("Service '{}' not found in configuration", service_name)
                })?;
                vec![(service_name.as_str(), service_config)]
            } else {
                resolved_services.iter().map(|(name, config)| (name.as_str(), config)).collect()
            };

            let mut total_checks = 0;
            let mut passed_checks = 0;
            let mut failed_checks = 0;

            for (service_name, service_config) in services_to_verify {
                if !service_config.enabled && service.is_none() {
                    // Skip disabled services when verifying all
                    continue;
                }

                println!("Service: {}", service_name);

                for target_name in &service_config.targets {
                    let destination = config.destinations.get(target_name).ok_or_else(|| {
                        anyhow::anyhow!("Destination '{}' not found", target_name)
                    })?;

                    println!("  Destination: {} ({})", target_name, destination.url);

                    // Build repository URL
                    let repo_url = utils::restic::build_repository_url(destination, service_name, None);
                    let password_file = &config.global.restic_password_file;
                    let env = utils::restic::ResticEnv::new(password_file, &repo_url);

                    total_checks += 1;

                    // Timeout: 5 minutes for normal check, 30 minutes for deep check
                    let timeout = if read_data {
                        std::time::Duration::from_secs(1800)
                    } else {
                        std::time::Duration::from_secs(300)
                    };

                    match utils::restic::check_repository(&env, read_data, timeout) {
                        Ok(output) => {
                            // Check if output contains any errors
                            if output.to_lowercase().contains("error") || output.to_lowercase().contains("fatal") {
                                println!("    ✗ Check completed with warnings/errors");
                                println!("    Output: {}", output);
                                failed_checks += 1;
                            } else {
                                println!("    ✓ Repository structure is OK");
                                println!("    ✓ No errors found");
                                passed_checks += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("    ✗ Check failed: {}", e);
                            failed_checks += 1;
                        }
                    }

                    println!();
                }
            }

            // Summary
            println!("=== Verification Summary ===");
            println!("Total checks: {}", total_checks);
            println!("Passed: {}", passed_checks);
            println!("Failed: {}", failed_checks);

            if failed_checks == 0 {
                println!("\n✓ All checks passed!");
            } else {
                println!("\n✗ Some checks failed. Please review the errors above.");
                std::process::exit(1);
            }
        }

        Commands::Setup { dry_run, cron_only, dirs_only } => {
            println!("=== Setting up restic-manager ===\n");

            if dry_run {
                println!("DRY RUN MODE - No changes will be made\n");
            }

            let mut _total_steps = 0;
            let mut _completed_steps = 0;

            // Step 1: Create directories (unless cron-only)
            if !cron_only {
                _total_steps += 1;
                println!("[1/4] Creating directories...");

                // Create log directory
                let log_dir = &config.global.log_directory;
                if dry_run {
                    println!("  [DRY RUN] Would create: {}", log_dir.display());
                } else {
                    match std::fs::create_dir_all(log_dir) {
                        Ok(_) => {
                            println!("  ✓ Created {}", log_dir.display());
                            _completed_steps += 1;
                        }
                        Err(e) => eprintln!("  ✗ Failed to create {}: {}", log_dir.display(), e),
                    }
                }

                // Create docker base directory if it doesn't exist
                let docker_base = &config.global.docker_base;
                if !docker_base.exists() {
                    if dry_run {
                        println!("  [DRY RUN] Would create: {}", docker_base.display());
                    } else {
                        match std::fs::create_dir_all(docker_base) {
                            Ok(_) => println!("  ✓ Created {}", docker_base.display()),
                            Err(e) => eprintln!("  ✗ Failed to create {}: {}", docker_base.display(), e),
                        }
                    }
                } else {
                    println!("  ✓ {} already exists", docker_base.display());
                }

                println!();
            }

            // Step 2: Initialize restic repositories (unless cron-only)
            if !cron_only {
                _total_steps += 1;
                println!("[2/4] Initializing restic repositories...");

                for (service_name, service_config) in &resolved_services {
                    if !service_config.enabled {
                        continue;
                    }

                    for target_name in &service_config.targets {
                        let destination = match config.destinations.get(target_name) {
                            Some(d) => d,
                            None => {
                                eprintln!("  ✗ Destination '{}' not found", target_name);
                                continue;
                            }
                        };

                        let repo_url = utils::restic::build_repository_url(destination, service_name, None);

                        if dry_run {
                            println!("  [DRY RUN] Would initialize: {} -> {}", service_name, repo_url);
                        } else {
                            let password_file = &config.global.restic_password_file;
                            let env = utils::restic::ResticEnv::new(password_file, &repo_url);

                            match utils::restic::init_repository(&env, std::time::Duration::from_secs(300)) {
                                Ok(_) => {
                                    println!("  ✓ Initialized {} at {} ({})", service_name, target_name, destination.url);
                                    _completed_steps += 1;
                                }
                                Err(e) => eprintln!("  ✗ Failed to initialize {} at {}: {}", service_name, target_name, e),
                            }
                        }
                    }
                }

                println!();
            }

            // Step 3: Install cron jobs (unless dirs-only)
            if !dirs_only {
                _total_steps += 1;
                println!("[3/4] Installing cron jobs...");

                #[cfg(unix)]
                {
                    let config_path = cli.config.clone();

                    for (service_name, service_config) in &resolved_services {
                        if !service_config.enabled {
                            println!("  - Skipping {} (disabled)", service_name);
                            continue;
                        }

                        // Validate cron schedule
                        if !utils::cron::validate_cron_schedule(&service_config.schedule) {
                            eprintln!("  ✗ Invalid cron schedule for {}: {}", service_name, service_config.schedule);
                            continue;
                        }

                        match utils::cron::add_cron_job(
                            service_name,
                            &service_config.schedule,
                            &config_path,
                            dry_run,
                        ) {
                            Ok(_) => {
                                println!("  ✓ Added job for '{}' ({})", service_name, service_config.schedule);
                                _completed_steps += 1;
                            }
                            Err(e) => eprintln!("  ✗ Failed to add job for {}: {}", service_name, e),
                        }
                    }
                }

                #[cfg(windows)]
                {
                    eprintln!("  ✗ Cron job setup is not supported on Windows");
                    eprintln!("    Use Task Scheduler instead");
                }

                println!();
            }

            // Step 4: Verify setup
            _total_steps += 1;
            println!("[4/4] Verifying setup...");

            if !dirs_only {
                #[cfg(unix)]
                {
                    if dry_run {
                        println!("  [DRY RUN] Would verify cron jobs");
                    } else {
                        match utils::cron::list_cron_jobs() {
                            Ok(jobs) => {
                                if jobs.is_empty() {
                                    eprintln!("  ⚠ No cron jobs found");
                                } else {
                                    println!("  ✓ {} cron job(s) installed", jobs.len());
                                }
                            }
                            Err(e) => eprintln!("  ✗ Failed to list cron jobs: {}", e),
                        }
                    }
                }
            }

            if !cron_only {
                if config.global.log_directory.exists() {
                    println!("  ✓ Log directory accessible");
                } else {
                    eprintln!("  ✗ Log directory not found");
                }

                if config.global.docker_base.exists() {
                    println!("  ✓ Docker base directory accessible");
                } else {
                    eprintln!("  ✗ Docker base directory not found");
                }
            }

            println!();

            // Summary
            if dry_run {
                println!("=== Dry Run Complete ===");
                println!("No changes were made. Run without --dry-run to apply changes.");
            } else {
                println!("=== Setup Complete ===");
                println!();
                println!("Next steps:");
                println!("  1. View scheduled jobs: crontab -l");
                println!("  2. Test a backup manually:");
                println!("     restic-manager run --service <SERVICE_NAME>");
                println!("  3. Check logs in: {}", config.global.log_directory.display());
            }
        }

        Commands::Validate => {
            println!("Configuration is valid!");
            println!("Services: {}", resolved_services.len());
            println!("Destinations: {}", config.destinations.len());
            println!("Profiles: {}", config.profiles.len());
        }

        // SetupRestic, UpdateRestic, and ResticVersion are handled at the start of main()
        Commands::SetupRestic | Commands::UpdateRestic | Commands::ResticVersion => {
            unreachable!("These commands are handled before config loading")
        }
    }

    Ok(())
}

/// Handle setup-restic command (doesn't require config)
fn handle_setup_restic() -> Result<()> {
    println!("Setting up managed restic binary...");

    if utils::restic_installer::local_restic_exists() {
        println!("✓ Managed restic is already installed");
        let version = utils::restic_installer::get_restic_version(false)?;
        println!("  Version: {}", version);
        println!("  Binary: {}", utils::restic_installer::get_restic_bin_path().display());
    } else {
        println!("Downloading restic from GitHub...");
        utils::restic_installer::download_restic()?;
        let version = utils::restic_installer::get_restic_version(false)?;
        println!("✓ Restic installed successfully");
        println!("  Version: {}", version);
        println!("  Binary: {}", utils::restic_installer::get_restic_bin_path().display());
        println!();
        println!("To use this binary, ensure use_system_restic = false in your config (default).");
    }

    Ok(())
}

/// Handle update-restic command (doesn't require config)
fn handle_update_restic(use_system_restic: bool) -> Result<()> {
    println!("Updating restic...");

    if !utils::restic_installer::restic_exists(use_system_restic) {
        if use_system_restic {
            println!("System restic not found in PATH.");
        } else {
            println!("Managed restic not found. Run 'restic-manager setup-restic' first.");
        }
        std::process::exit(1);
    }

    let old_version = utils::restic_installer::get_restic_version(use_system_restic)?;
    println!("Current version: {}", old_version);

    utils::restic_installer::update_restic(use_system_restic)?;

    let new_version = utils::restic_installer::get_restic_version(use_system_restic)?;
    println!("✓ Updated to: {}", new_version);

    Ok(())
}

/// Handle restic-version command (doesn't require config)
fn handle_restic_version(use_system_restic: bool) -> Result<()> {
    if !utils::restic_installer::restic_exists(use_system_restic) {
        if use_system_restic {
            println!("System restic not found in PATH.");
        } else {
            println!("Managed restic not found. Run 'restic-manager setup-restic' first.");
        }
        std::process::exit(1);
    }

    let version = utils::restic_installer::get_restic_version(use_system_restic)?;
    println!("Restic version: {}", version);
    println!("Binary location: {}", utils::restic_installer::get_restic_command(use_system_restic));

    if use_system_restic {
        println!("Source: System PATH (use_system_restic = true)");
    } else {
        println!("Source: Managed binary (use_system_restic = false)");
    }

    Ok(())
}

