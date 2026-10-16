//! Human-readable sizes, durations and paths.

use std::path::Path;
use std::time::SystemTime;

/// Bytes as Finder shows them (decimal units): `1.2 GB`, `340 MB`, `12 KB`.
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];
    if n < 1000 {
        return format!("{n} B");
    }
    let mut value = n as f64 / 1000.;
    let mut unit = 0;
    while value >= 999.95 && unit < UNITS.len() - 1 {
        value /= 1000.;
        unit += 1;
    }
    if value < 10. {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

/// Days elapsed since `then` (0 if in the future).
pub fn days_since(then: SystemTime, now: SystemTime) -> u64 {
    now.duration_since(then).map_or(0, |d| d.as_secs() / 86_400)
}

/// `path` with the home folder shown as `~`.
pub fn display_path(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn shortens_home_and_counts_days() {
        let home = Path::new("/Users/al");
        assert_eq!(
            display_path(Path::new("/Users/al/dev/x"), Some(home)),
            "~/dev/x"
        );
        assert_eq!(display_path(home, Some(home)), "~");
        assert_eq!(display_path(Path::new("/opt/x"), Some(home)), "/opt/x");
        let t = SystemTime::UNIX_EPOCH;
        assert_eq!(days_since(t, t + Duration::from_secs(3 * 86_400 + 5)), 3);
        assert_eq!(days_since(t + Duration::from_secs(10), t), 0);
    }

    #[test]
    fn formats_like_finder() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(999), "999 B");
        assert_eq!(bytes(1_000), "1.0 KB");
        assert_eq!(bytes(12_345), "12 KB");
        assert_eq!(bytes(340_000_000), "340 MB");
        assert_eq!(bytes(1_234_000_000), "1.2 GB");
        assert_eq!(bytes(999_999_999), "1.0 GB");
        assert_eq!(bytes(u64::MAX), "18447 PB");
    }
}
