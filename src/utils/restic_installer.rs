//! Restic binary installation and management

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// Get the path where restic binary should be stored
pub fn get_restic_bin_path() -> PathBuf {
    let app_dir = get_app_dir();
    app_dir.join("bin").join(restic_binary_name())
}

/// Get the application directory (~/.restic-manager or %LOCALAPPDATA%/restic-manager)
fn get_app_dir() -> PathBuf {
    #[cfg(unix)]
    {
        if let Some(home) = dirs::home_dir() {
            home.join(".restic-manager")
        } else {
            PathBuf::from(".restic-manager")
        }
    }

    #[cfg(windows)]
    {
        if let Some(app_data) = dirs::data_local_dir() {
            app_data.join("restic-manager")
        } else {
            PathBuf::from("restic-manager")
        }
    }
}

/// Get the platform-specific binary name
fn restic_binary_name() -> &'static str {
    #[cfg(windows)]
    return "restic.exe";

    #[cfg(not(windows))]
    return "restic";
}

/// Check if local managed restic binary exists
pub fn local_restic_exists() -> bool {
    get_restic_bin_path().exists()
}

/// Check if system restic exists in PATH
pub fn system_restic_exists() -> bool {
    which::which("restic").is_ok()
}

/// Check if restic binary exists (local only, unless use_system is true)
pub fn restic_exists(use_system: bool) -> bool {
    if local_restic_exists() {
        return true;
    }

    if use_system {
        return system_restic_exists();
    }

    false
}

/// Get the restic command to use
pub fn get_restic_command(use_system: bool) -> String {
    // If explicitly using system restic, check PATH first
    if use_system && system_restic_exists() {
        return "restic".to_string();
    }

    // Otherwise, prefer local binary
    let local_path = get_restic_bin_path();
    if local_path.exists() {
        local_path.display().to_string()
    } else if use_system && system_restic_exists() {
        "restic".to_string()
    } else {
        // Return local path anyway - will fail with clear error
        local_path.display().to_string()
    }
}

/// Ensure restic is available (download if needed)
pub async fn ensure_restic(use_system: bool) -> Result<PathBuf> {
    let local_path = get_restic_bin_path();

    // If using system restic, check PATH
    if use_system {
        if system_restic_exists() {
            info!("Using system restic from PATH");
            return Ok(PathBuf::from("restic"));
        } else {
            anyhow::bail!("System restic requested but not found in PATH");
        }
    }

    // Use local managed binary
    if local_path.exists() {
        info!("Using local managed restic binary: {:?}", local_path);
        return Ok(local_path);
    }

    // Need to download restic
    info!("Local restic not found, downloading from GitHub...");
    download_restic().await?;

    Ok(local_path)
}

/// Download restic from GitHub releases
pub async fn download_restic() -> Result<()> {
    let download_url = get_download_url()?;
    info!("Downloading restic from: {}", download_url);

    // Create bin directory
    let bin_dir = get_app_dir().join("bin");
    fs::create_dir_all(&bin_dir)
        .context("Failed to create bin directory")?;

    // Download the archive
    let client = reqwest::Client::builder()
        .user_agent("restic-manager/0.1.0")
        .build()?;

    let response = client
        .get(&download_url)
        .send()
        .await
        .context("Failed to download restic")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download restic: HTTP {}", response.status());
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response")?;

    info!("Downloaded {} bytes", bytes.len());

    // Extract binary
    extract_restic(&bytes, &bin_dir)?;

    info!("Successfully installed restic to: {:?}", get_restic_bin_path());

    Ok(())
}

/// Get the download URL for the current platform
fn get_download_url() -> Result<String> {
    let base_url = "https://github.com/restic/restic/releases/latest/download";

    // Detect platform and architecture
    let (os, arch, ext) = if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            ("windows", "amd64", "zip")
        } else if cfg!(target_arch = "aarch64") {
            ("windows", "arm64", "zip")
        } else {
            anyhow::bail!("Unsupported Windows architecture")
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "x86_64") {
            ("linux", "amd64", "bz2")
        } else if cfg!(target_arch = "aarch64") {
            ("linux", "arm64", "bz2")
        } else {
            anyhow::bail!("Unsupported Linux architecture")
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "x86_64") {
            ("darwin", "amd64", "bz2")
        } else if cfg!(target_arch = "aarch64") {
            ("darwin", "arm64", "bz2")
        } else {
            anyhow::bail!("Unsupported macOS architecture")
        }
    } else {
        anyhow::bail!("Unsupported operating system")
    };

    Ok(format!("{}/restic_{}_{}.{}", base_url, os, arch, ext))
}

/// Extract restic binary from archive
fn extract_restic(bytes: &[u8], bin_dir: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        extract_zip(bytes, bin_dir)
    }

    #[cfg(not(windows))]
    {
        extract_bz2(bytes, bin_dir)
    }
}

#[cfg(windows)]
fn extract_zip(bytes: &[u8], bin_dir: &Path) -> Result<()> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .context("Failed to read ZIP archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();

        if name.ends_with("restic.exe") || name == "restic.exe" {
            let output_path = bin_dir.join("restic.exe");
            let mut output = fs::File::create(&output_path)
                .context("Failed to create restic.exe")?;
            std::io::copy(&mut file, &mut output)
                .context("Failed to write restic.exe")?;
            info!("Extracted restic.exe");
            return Ok(());
        }
    }

    anyhow::bail!("restic.exe not found in ZIP archive")
}

#[cfg(not(windows))]
fn extract_bz2(bytes: &[u8], bin_dir: &Path) -> Result<()> {
    use bzip2::read::BzDecoder;
    use std::io::Read;

    let mut decoder = BzDecoder::new(bytes);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .context("Failed to decompress bz2")?;

    let output_path = bin_dir.join("restic");
    fs::write(&output_path, &decompressed)
        .context("Failed to write restic binary")?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&output_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&output_path, perms)
            .context("Failed to set executable permissions")?;
    }

    info!("Extracted restic");
    Ok(())
}

/// Update restic using self-update
pub async fn update_restic(use_system: bool) -> Result<()> {
    let restic_cmd = get_restic_command(use_system);

    info!("Updating restic using self-update...");

    let output = tokio::process::Command::new(&restic_cmd)
        .arg("self-update")
        .output()
        .await
        .context("Failed to run restic self-update")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Restic self-update failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("Update result: {}", stdout.trim());

    Ok(())
}

/// Get restic version
pub async fn get_restic_version(use_system: bool) -> Result<String> {
    let restic_cmd = get_restic_command(use_system);

    let output = tokio::process::Command::new(&restic_cmd)
        .arg("version")
        .output()
        .await
        .context("Failed to get restic version")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get restic version");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}
