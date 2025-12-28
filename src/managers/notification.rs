//! Discord webhook notification manager
//!
//! Sends notifications to Discord via webhooks for backup events.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info};

use crate::config::{NotificationConfig, NotifyEvent};

/// Notification manager for sending Discord webhooks
pub struct NotificationManager {
    config: NotificationConfig,
    cache_path: PathBuf,
}

/// Discord embed color codes (decimal)
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum NotificationColor {
    /// Red - for failures
    Failure = 15158332,    // #E74C3C
    /// Orange - for warnings
    Warning = 15105570,    // #E67E22
    /// Yellow - for long-running operations
    LongRunning = 16776960, // #FFFF00
    /// Green - for success
    Success = 3066993,     // #2ECC71
    /// Blue - for info
    Info = 3447003,        // #3498DB
}

impl NotificationColor {
    fn as_decimal(&self) -> u32 {
        *self as u32
    }
}

/// Notification payload to send
#[derive(Debug, Clone)]
pub struct Notification {
    pub event_type: NotifyEvent,
    pub service_name: String,
    pub destination: Option<String>,
    pub message: String,
    pub error: Option<String>,
    pub duration_secs: Option<u64>,
}

/// Discord webhook payload
#[derive(Debug, Serialize)]
struct DiscordPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    embeds: Vec<DiscordEmbed>,
}

#[derive(Debug, Serialize)]
struct DiscordEmbed {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    color: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<DiscordField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    footer: Option<DiscordFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
struct DiscordField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Debug, Serialize)]
struct DiscordFooter {
    text: String,
}

/// Rate limit cache entry
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// Unix timestamp of last notification
    last_sent: u64,
    /// Count of notifications sent in current window
    count: u32,
}

/// Rate limit cache
#[derive(Debug, Serialize, Deserialize, Default)]
struct NotificationCache {
    /// Map of cache key to entry
    entries: HashMap<String, CacheEntry>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(config: NotificationConfig) -> Self {
        let cache_path = Self::get_cache_path();
        Self { config, cache_path }
    }

    /// Get the cache file path
    fn get_cache_path() -> PathBuf {
        if let Some(cache_dir) = dirs::cache_dir() {
            cache_dir.join("restic-manager-notifications.json")
        } else {
            PathBuf::from("/tmp/restic-manager-notifications.json")
        }
    }

    /// Check if notifications are enabled for an event type
    pub fn is_enabled(&self, event: &NotifyEvent) -> bool {
        if self.config.discord_webhook_url.is_empty() {
            return false;
        }
        self.config.notify_on.contains(event)
    }

    /// Send a notification if enabled and not rate-limited
    pub fn send(&self, notification: Notification) -> Result<()> {
        // Check if this event type is enabled
        if !self.is_enabled(&notification.event_type) {
            debug!(
                "Notification type {:?} not enabled, skipping",
                notification.event_type
            );
            return Ok(());
        }

        // Check rate limit
        let cache_key = format!(
            "{}:{}:{:?}",
            notification.service_name,
            notification.destination.as_deref().unwrap_or("all"),
            notification.event_type
        );

        if self.is_rate_limited(&cache_key)? {
            debug!("Notification rate-limited for key: {}", cache_key);
            return Ok(());
        }

        // Build and send the webhook
        let payload = self.build_payload(&notification);
        self.send_webhook(&payload)?;

        // Update rate limit cache
        self.update_cache(&cache_key)?;

        info!(
            "Sent {:?} notification for service '{}'",
            notification.event_type, notification.service_name
        );

        Ok(())
    }

    /// Send a failure notification
    pub fn send_failure(
        &self,
        service_name: &str,
        destination: Option<&str>,
        error: &str,
        duration_secs: Option<u64>,
    ) -> Result<()> {
        self.send(Notification {
            event_type: NotifyEvent::Failure,
            service_name: service_name.to_string(),
            destination: destination.map(String::from),
            message: format!("Backup failed for service '{}'", service_name),
            error: Some(error.to_string()),
            duration_secs,
        })
    }

    /// Send a warning notification
    #[allow(dead_code)]
    pub fn send_warning(
        &self,
        service_name: &str,
        destination: Option<&str>,
        message: &str,
    ) -> Result<()> {
        self.send(Notification {
            event_type: NotifyEvent::Warning,
            service_name: service_name.to_string(),
            destination: destination.map(String::from),
            message: message.to_string(),
            error: None,
            duration_secs: None,
        })
    }

    /// Send a long-running notification
    pub fn send_long_running(
        &self,
        service_name: &str,
        destination: Option<&str>,
        duration_secs: u64,
        threshold_minutes: u64,
    ) -> Result<()> {
        self.send(Notification {
            event_type: NotifyEvent::LongRunning,
            service_name: service_name.to_string(),
            destination: destination.map(String::from),
            message: format!(
                "Backup is taking longer than expected (>{} minutes)",
                threshold_minutes
            ),
            error: None,
            duration_secs: Some(duration_secs),
        })
    }

    /// Send a success notification
    pub fn send_success(
        &self,
        service_name: &str,
        destination: Option<&str>,
        duration_secs: u64,
    ) -> Result<()> {
        self.send(Notification {
            event_type: NotifyEvent::Success,
            service_name: service_name.to_string(),
            destination: destination.map(String::from),
            message: format!("Backup completed successfully for service '{}'", service_name),
            error: None,
            duration_secs: Some(duration_secs),
        })
    }

    /// Build Discord webhook payload
    fn build_payload(&self, notification: &Notification) -> DiscordPayload {
        let (color, emoji) = match notification.event_type {
            NotifyEvent::Failure => (NotificationColor::Failure, "\u{274C}"), // Red X
            NotifyEvent::Warning => (NotificationColor::Warning, "\u{26A0}\u{FE0F}"), // Warning
            NotifyEvent::LongRunning => (NotificationColor::LongRunning, "\u{23F0}"), // Alarm clock
            NotifyEvent::Success => (NotificationColor::Success, "\u{2705}"), // Green check
        };

        let title = format!(
            "{} Restic Manager: {:?}",
            emoji,
            notification.event_type
        );

        let mut fields = vec![
            DiscordField {
                name: "Service".to_string(),
                value: notification.service_name.clone(),
                inline: true,
            },
        ];

        if let Some(ref dest) = notification.destination {
            fields.push(DiscordField {
                name: "Destination".to_string(),
                value: dest.clone(),
                inline: true,
            });
        }

        if let Some(duration) = notification.duration_secs {
            fields.push(DiscordField {
                name: "Duration".to_string(),
                value: format_duration(duration),
                inline: true,
            });
        }

        if let Some(ref error) = notification.error {
            // Truncate error message if too long
            let error_display = if error.len() > 500 {
                format!("{}...", &error[..497])
            } else {
                error.clone()
            };
            fields.push(DiscordField {
                name: "Error".to_string(),
                value: format!("```\n{}\n```", error_display),
                inline: false,
            });
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| {
                chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            })
            .ok()
            .flatten();

        let embed = DiscordEmbed {
            title,
            description: Some(notification.message.clone()),
            color: color.as_decimal(),
            fields,
            footer: Some(DiscordFooter {
                text: "restic-manager".to_string(),
            }),
            timestamp,
        };

        DiscordPayload {
            username: Some("Restic Manager".to_string()),
            avatar_url: None,
            content: None,
            embeds: vec![embed],
        }
    }

    /// Send webhook to Discord
    fn send_webhook(&self, payload: &DiscordPayload) -> Result<()> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .post(&self.config.discord_webhook_url)
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .context("Failed to send Discord webhook")?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 204 {
            debug!("Discord webhook sent successfully");
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            error!("Discord webhook failed with status {}: {}", status, body);
            anyhow::bail!("Discord webhook failed with status {}: {}", status, body)
        }
    }

    /// Check if a notification is rate-limited
    fn is_rate_limited(&self, cache_key: &str) -> Result<bool> {
        let cache = self.load_cache()?;

        if let Some(entry) = cache.entries.get(cache_key) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let rate_limit_secs = self.config.rate_limit_minutes * 60;

            if now - entry.last_sent < rate_limit_secs {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Update the rate limit cache
    fn update_cache(&self, cache_key: &str) -> Result<()> {
        let mut cache = self.load_cache()?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        cache.entries.insert(
            cache_key.to_string(),
            CacheEntry {
                last_sent: now,
                count: cache.entries.get(cache_key).map_or(1, |e| e.count + 1),
            },
        );

        // Clean up old entries (older than 24 hours)
        let cutoff = now.saturating_sub(86400);
        cache.entries.retain(|_, v| v.last_sent > cutoff);

        self.save_cache(&cache)?;
        Ok(())
    }

    /// Load the notification cache from disk
    fn load_cache(&self) -> Result<NotificationCache> {
        if !self.cache_path.exists() {
            return Ok(NotificationCache::default());
        }

        let content = fs::read_to_string(&self.cache_path)
            .context("Failed to read notification cache")?;

        serde_json::from_str(&content)
            .context("Failed to parse notification cache")
    }

    /// Save the notification cache to disk
    fn save_cache(&self, cache: &NotificationCache) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(cache)
            .context("Failed to serialize notification cache")?;

        fs::write(&self.cache_path, content)
            .context("Failed to write notification cache")?;

        Ok(())
    }
}

/// Format duration in human-readable form
fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        if secs == 0 {
            format!("{}m", minutes)
        } else {
            format!("{}m {}s", minutes, secs)
        }
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        if minutes == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, minutes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(45), "45s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(120), "2m");
        assert_eq!(format_duration(125), "2m 5s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(3720), "1h 2m");
        assert_eq!(format_duration(7320), "2h 2m");
    }

    #[test]
    fn test_notification_color_values() {
        assert_eq!(NotificationColor::Failure.as_decimal(), 15158332);
        assert_eq!(NotificationColor::Warning.as_decimal(), 15105570);
        assert_eq!(NotificationColor::Success.as_decimal(), 3066993);
    }

    #[test]
    fn test_notification_manager_disabled_when_no_url() {
        let config = NotificationConfig {
            discord_webhook_url: String::new(),
            notify_on: vec![NotifyEvent::Failure],
            rate_limit_minutes: 60,
            cache_file: std::path::PathBuf::from("/tmp/test-cache.json"),
        };
        let manager = NotificationManager::new(config);
        assert!(!manager.is_enabled(&NotifyEvent::Failure));
    }

    #[test]
    fn test_notification_manager_disabled_for_unregistered_events() {
        let config = NotificationConfig {
            discord_webhook_url: "https://discord.com/api/webhooks/test".to_string(),
            notify_on: vec![NotifyEvent::Failure],
            rate_limit_minutes: 60,
            cache_file: std::path::PathBuf::from("/tmp/test-cache.json"),
        };
        let manager = NotificationManager::new(config);
        assert!(manager.is_enabled(&NotifyEvent::Failure));
        assert!(!manager.is_enabled(&NotifyEvent::Warning));
        assert!(!manager.is_enabled(&NotifyEvent::Success));
    }

    #[test]
    fn test_build_failure_payload() {
        let config = NotificationConfig {
            discord_webhook_url: "https://discord.com/api/webhooks/test".to_string(),
            notify_on: vec![NotifyEvent::Failure],
            rate_limit_minutes: 60,
            cache_file: std::path::PathBuf::from("/tmp/test-cache.json"),
        };
        let manager = NotificationManager::new(config);

        let notification = Notification {
            event_type: NotifyEvent::Failure,
            service_name: "postgres".to_string(),
            destination: Some("local".to_string()),
            message: "Backup failed".to_string(),
            error: Some("Connection refused".to_string()),
            duration_secs: Some(120),
        };

        let payload = manager.build_payload(&notification);

        assert_eq!(payload.embeds.len(), 1);
        assert!(payload.embeds[0].title.contains("Failure"));
        assert_eq!(payload.embeds[0].color, NotificationColor::Failure.as_decimal());
        assert!(payload.embeds[0].fields.iter().any(|f| f.name == "Service" && f.value == "postgres"));
        assert!(payload.embeds[0].fields.iter().any(|f| f.name == "Destination" && f.value == "local"));
        assert!(payload.embeds[0].fields.iter().any(|f| f.name == "Duration" && f.value == "2m"));
        assert!(payload.embeds[0].fields.iter().any(|f| f.name == "Error"));
    }

    #[test]
    fn test_cache_path_creation() {
        let path = NotificationManager::get_cache_path();
        assert!(path.to_string_lossy().contains("restic-manager-notifications"));
    }
}
