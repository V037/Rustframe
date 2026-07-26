use sysinfo::{ProcessExt, Pid, System, SystemExt};

#[derive(Debug, Clone)]
pub struct RamUsage {
    pub total_kb: u64,
    pub available_kb: u64,
    pub used_kb: u64,
}

impl RamUsage {
    /// Read RAM usage of the current process using `sysinfo`.
    pub fn read() -> Self {
        let mut system = System::new_all();
        system.refresh_memory();

        let total_kb = system.total_memory();
        // Get memory used by this process (in KB)
        let pid = Pid::from(std::process::id() as usize);
        let used_kb = if let Some(process) = system.process(pid) {
            process.memory() as u64
        } else {
            0
        };

        RamUsage {
            total_kb,
            available_kb: 0,
            used_kb,
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

    /// Format bytes to human‑readable string (e.g., "1.5 GB")
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

    /// Format percentage to a human‑readable string (e.g., "75.3%")
    pub fn format_percent(percent: f32) -> String {
        format!("{:.1}%", percent)
    }
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
