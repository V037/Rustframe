use std::fs;

#[derive(Debug, Clone)]
pub struct RamUsage {
    pub total_kb: u64,
    pub available_kb: u64,
    pub used_kb: u64,
}

impl RamUsage {
    /// Read RAM usage from /proc/meminfo (Linux) or return a default value.
    pub fn read() -> Self {
        if let Ok(content) = fs::read_to_string("/proc/meminfo") {
            let total_kb = parse_meminfo_line(&content, "MemTotal");
            let available_kb = parse_meminfo_line(&content, "MemAvailable");

            RamUsage {
                total_kb: total_kb.unwrap_or(0),
                available_kb: available_kb.unwrap_or(0),
                used_kb: total_kb.unwrap_or(0).saturating_sub(available_kb.unwrap_or(0)),
            }
        } else {
            // Fallback for non-Linux environments
            RamUsage {
                total_kb: 0,
                available_kb: 0,
                used_kb: 0,
            }
        }
    }

    /// Get usage percentage (0.0 - 100.0)
    pub fn usage_percent(&self) -> f32 {
        if self.total_kb == 0 {
            return 0.0;
        }
        let percent = self.used_kb as f32 / self.total_kb as f32 * 100.0;
        percent.min(100.0)
    }

    /// Format bytes to human-readable string (e.g., "1.5 GB")
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * 1024;
        const GB: u64 = 1024 * 1024 * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Format percentage to a human-readable string (e.g., "75.3%")
    pub fn format_percent(percent: f32) -> String {
        format!("{:.1}%", percent)
    }
}

fn parse_meminfo_line(content: &str, key: &str) -> Option<u64> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(key) {
            // Format: "MemTotal:       15927680 kB"
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                return parts[2].parse::<u64>().ok();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(RamUsage::format_bytes(1024), "1.0 KB");
        assert_eq!(RamUsage::format_bytes(1048576), "1.0 MB");
        assert_eq!(RamUsage::format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_percent() {
        assert_eq!(RamUsage::format_percent(50.0), "50.0%");
        assert_eq!(RamUsage::format_percent(100.0), "100.0%");
    }

    #[test]
    fn test_read_returns_valid_structure() {
        let ram = RamUsage::read();
        // Just ensure it doesn't panic and has reasonable values
        assert!(ram.total_kb >= 0);
        assert!(ram.used_kb <= ram.total_kb);
    }
}