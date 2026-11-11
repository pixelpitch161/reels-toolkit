//! The app's version, and the commit and date it was built from (written by `build.rs`).

/// The version from the workspace's `Cargo.toml`, such as "0.6.0" or "0.6.0-rc.1".
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The commit built from, 7 hex digits, or "unknown" when git was not there.
pub const COMMIT: &str = env!("BF_COMMIT");
/// The commit's own date, such as "2026-11-12", or "unknown".
pub const DATE: &str = env!("BF_COMMIT_DATE");

/// The version as Settings shows it: "0.6.0 (a842e11, 12 Nov 2026)".
pub fn line() -> String {
    format!("{VERSION} ({COMMIT}, {})", date_words(DATE))
}

/// The version with the app's name, as `--version` prints it: "BALLFIGHT Studio 0.6.0 (a842e11, 12 Nov 2026)".
pub fn version_text() -> String {
    format!("BALLFIGHT Studio {}", line())
}

/// "2026-11-12" -> "12 Nov 2026"; anything else (such as "unknown") -> "unknown".
fn date_words(date: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    let parts: Vec<&str> = date.split('-').collect();
    if let [year, month, day] = parts[..]
        && digits(year, 4)
        && digits(month, 2)
        && digits(day, 2)
    {
        let (m, d): (usize, u32) = (month.parse().unwrap_or(0), day.parse().unwrap_or(0));
        if (1..=12).contains(&m) && (1..=31).contains(&d) {
            return format!("{d} {} {year}", MONTHS[m - 1]);
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    /// The plan's pattern, `^\d+\.\d+\.\d+(-rc\.\d+)? \((unknown|[0-9a-f]{7}), (unknown|\d{1,2} [A-Z][a-z]{2} \d{4})\)$`,
    /// checked by hand (the workspace has no regex crate).
    fn reads_plainly(line: &str) -> bool {
        fn number(s: &str) -> bool {
            !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
        }
        let Some((version, rest)) = line.split_once(" (") else {
            return false;
        };
        let Some(inner) = rest.strip_suffix(')') else {
            return false;
        };
        let (plain, rc) = match version.split_once("-rc.") {
            Some((plain, n)) => (plain, Some(n)),
            None => (version, None),
        };
        let parts: Vec<&str> = plain.split('.').collect();
        if parts.len() != 3 || !parts.iter().all(|p| number(p)) || rc.is_some_and(|n| !number(n)) {
            return false;
        }
        let Some((commit, date)) = inner.split_once(", ") else {
            return false;
        };
        let commit_ok = commit == "unknown"
            || (commit.len() == 7 && commit.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
        let date_ok = date == "unknown" || {
            let words: Vec<&str> = date.split(' ').collect();
            words.len() == 3
                && number(words[0])
                && words[0].len() <= 2
                && words[1].len() == 3
                && words[1].chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && words[1].chars().skip(1).all(|c| c.is_ascii_lowercase())
                && number(words[2])
                && words[2].len() == 4
        };
        commit_ok && date_ok
    }

    #[test]
    fn the_line_reads_plainly() {
        assert!(reads_plainly(&line()), "{:?}", line());
        assert_eq!(version_text(), format!("BALLFIGHT Studio {}", line()));
        // The pattern itself takes the plan's examples and refuses what it must.
        assert!(reads_plainly("0.6.0 (a842e11, 12 Nov 2026)"));
        assert!(reads_plainly("0.6.0-rc.1 (unknown, unknown)"));
        assert!(reads_plainly("1.0.0 (a842e11, 1 Dec 2026)"));
        for bad in [
            "0.6 (a842e11, 12 Nov 2026)",
            "0.6.0 (A842E11, 12 Nov 2026)",
            "0.6.0 (a842e11, 2026-11-12)",
            "0.6.0 (a842e11, 12 nov 2026)",
            "0.6.0 (a842e1, 12 Nov 2026)",
            "0.6.0 a842e11, 12 Nov 2026",
        ] {
            assert!(!reads_plainly(bad), "{bad}");
        }
    }

    #[test]
    fn the_date_is_written_in_words() {
        assert_eq!(date_words("2026-11-12"), "12 Nov 2026");
        assert_eq!(date_words("2026-01-05"), "5 Jan 2026");
        assert_eq!(date_words("2027-12-31"), "31 Dec 2027");
        assert_eq!(date_words("unknown"), "unknown");
        assert_eq!(date_words("2026-13-01"), "unknown");
        assert_eq!(date_words("2026-11-1x"), "unknown");
        assert_eq!(date_words(""), "unknown");
    }

    #[test]
    fn the_commit_is_this_checkout() {
        let never = AtomicBool::new(false);
        let args = [OsStr::new("rev-parse"), OsStr::new("--short=7"), OsStr::new("HEAD")];
        // Tests run in the crate's folder, as the build script does.
        let Ok(ran) = bf_pack::run::run(Path::new("git"), &args, Duration::from_secs(20), &never) else {
            assert_eq!(COMMIT, "unknown");
            return;
        };
        if !ran.status.success() {
            assert_eq!(COMMIT, "unknown");
            return;
        }
        let here = String::from_utf8_lossy(&ran.stdout).trim().to_string();
        assert_eq!(COMMIT, here);
        assert_ne!(DATE, "unknown");
    }
}
