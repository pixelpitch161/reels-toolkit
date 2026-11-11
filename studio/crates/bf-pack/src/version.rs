//! Version numbers: `0.6.0` and release candidates `0.6.0-rc.1`, nothing else. A candidate sorts before its
//! release: `0.6.0-rc.1 < 0.6.0-rc.2 < 0.6.0 < 0.6.1 < 0.10.0`.

use std::cmp::Ordering;
use std::fmt;

/// A version as the Studio writes them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// `Some(n)` for `-rc.n`; a candidate sorts before the release of the same numbers.
    pub rc: Option<u64>,
}

// Written out, not derived: a derived order would put `None` (the release) before `Some` (its candidates).
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.rc, other.rc) {
                (None, None) => Ordering::Equal,
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (Some(a), Some(b)) => a.cmp(&b),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(rc) = self.rc {
            write!(f, "-rc.{rc}")?;
        }
        Ok(())
    }
}

/// A number as a version writes it: digits only, no leading zero (except "0" itself), no sign.
fn number(s: &str) -> Option<u64> {
    let plain = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'));
    if plain { s.parse().ok() } else { None }
}

/// `"0.6.0"` or `"0.6.0-rc.1"`; anything else (`"v0.6"`, `"0.6.0-beta"`, spaces, leading zeros) is `None`.
pub fn parse_version(s: &str) -> Option<Version> {
    let (numbers, rc) = match s.split_once('-') {
        Some((numbers, pre)) => (numbers, Some(number(pre.strip_prefix("rc.")?)?)),
        None => (s, None),
    };
    let mut parts = numbers.split('.');
    let major = number(parts.next()?)?;
    let minor = number(parts.next()?)?;
    let patch = number(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(Version {
        major,
        minor,
        patch,
        rc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        parse_version(s).unwrap_or_else(|| panic!("{s} should read"))
    }

    #[test]
    fn versions_order() {
        assert!(v("0.6.0-rc.1") < v("0.6.0"));
        assert!(v("0.6.0") < v("0.6.1"));
        assert!(v("0.6.1") < v("0.10.0"));
        assert!(v("0.6.0-rc.1") < v("0.6.0-rc.2"));
        assert!(v("0.6.0-rc.2") < v("0.6.0"));
        assert!(v("0.9.9") < v("1.0.0-rc.1"));
        assert_eq!(v("0.6.0"), v("0.6.0"));
        for bad in [
            "v0.6",
            "0.6",
            "0.6.0-beta",
            "0.6.0-rc",
            "0.6.0-rc.",
            "0.6.0-rc.x",
            "0.6.0.1",
            "",
            " 0.6.0",
            "0.6.0 ",
            "01.6.0",
            "0.6.0-rc.01",
            "+1.0.0",
            "0.6.0-rc.1-rc.2",
            "1..0",
        ] {
            assert_eq!(parse_version(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_version_writes_as_it_reads() {
        for s in ["0.6.0", "0.6.0-rc.1", "1.0.0", "0.10.12-rc.3"] {
            assert_eq!(v(s).to_string(), s);
        }
        assert_eq!(
            v("0.6.0-rc.1"),
            Version {
                major: 0,
                minor: 6,
                patch: 0,
                rc: Some(1)
            }
        );
    }
}
