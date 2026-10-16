//! Human-readable sizes.

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

#[cfg(test)]
mod tests {
    use super::bytes;

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
