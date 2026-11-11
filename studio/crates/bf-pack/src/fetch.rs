//! Downloads. The real one goes through Windows' own `curl.exe`; tests use a `Download` that copies a local file.
//! A download takes a public `https` address only and never sends a user name, password or token.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Something that can put the file at `url` into the file `to`. The text of an error says why, in plain words.
pub trait Download {
    fn fetch(&self, url: &str, to: &Path) -> Result<(), String>;
}

/// The real download: `%SystemRoot%\System32\curl.exe`, part of Windows 11.
pub struct Curl;

impl Curl {
    /// Where Windows keeps its own curl.
    pub fn program() -> Result<PathBuf, String> {
        #[cfg(windows)]
        {
            let root = std::env::var_os("SystemRoot").ok_or("Windows' folder is not known (SystemRoot is not set).")?;
            let curl = PathBuf::from(root).join("System32").join("curl.exe");
            if curl.is_file() {
                Ok(curl)
            } else {
                Err(format!("Windows' own curl is missing from {}.", curl.display()))
            }
        }
        #[cfg(not(windows))]
        {
            Err("Downloading works only on Windows.".to_string())
        }
    }
}

/// The longest a download may take before it is stopped.
const DOWNLOAD_LIMIT: Duration = Duration::from_secs(60 * 60);

impl Download for Curl {
    fn fetch(&self, url: &str, to: &Path) -> Result<(), String> {
        check_public_url(url)?;
        let curl = Curl::program()?;
        let args = curl_args(url, to);
        let args: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
        let never = AtomicBool::new(false);
        let ran = crate::run::run(&curl, &args, DOWNLOAD_LIMIT, &never)?;
        if ran.status.success() {
            Ok(())
        } else {
            Err(match ran.tail.last() {
                Some(line) => format!("curl said \"{line}\""),
                None => format!("curl ended with {}", ran.status),
            })
        }
    }
}

/// Refuses anything but a public `https` address: no user name or password in it, no spaces or control characters.
/// The refusal never repeats the address, which could hold a secret.
pub fn check_public_url(url: &str) -> Result<(), String> {
    let Some(rest) = url.strip_prefix("https://") else {
        return Err("Only an https address can be downloaded from.".to_string());
    };
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("The download address holds a space or a control character, so it is refused.".to_string());
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Err("The download address holds a user name or password, so it is refused.".to_string());
    }
    if authority.is_empty() || authority.starts_with(':') {
        return Err("The download address names no host.".to_string());
    }
    Ok(())
}

/// curl's arguments, one by one: fail on an HTTP error, no progress meter but errors shown, follow redirects, retry
/// three times, `https` only (also after a redirect), write to `to`.
pub(crate) fn curl_args(url: &str, to: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "-fsSL",
        "--retry",
        "3",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "-o",
    ]
    .map(OsString::from)
    .into();
    args.push(to.as_os_str().to_owned());
    args.push("--url".into());
    args.push(url.into());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_public_https_address_is_taken() {
        assert!(check_public_url("https://www.gyan.dev/ffmpeg/builds/packages/a.zip").is_ok());
        assert!(check_public_url("https://example.org/a.zip?x=1").is_ok());
        for bad in [
            "http://www.gyan.dev/a.zip",
            "file:///C:/a.zip",
            "ftp://example.org/a.zip",
            "https://",
            "https:///a.zip",
            "https://example.org/a b.zip",
            "https://example.org/a\nb.zip",
            "HTTPS//example.org",
        ] {
            assert!(check_public_url(bad).is_err(), "{bad:?} was taken");
        }
    }

    #[test]
    fn an_address_with_a_name_in_it_is_refused_without_repeating_it() {
        for bad in [
            "https://someone:s3cr3tvalue@example.org/a.zip",
            "https://s3cr3tvalue@example.org/a.zip",
        ] {
            let err = check_public_url(bad).unwrap_err();
            assert!(!err.contains("s3cr3tvalue"), "{err}");
            assert!(err.contains("user name or password"), "{err}");
        }
        // An "@" after the host is part of the path, not a name.
        assert!(check_public_url("https://example.org/a@b.zip").is_ok());
    }

    #[test]
    fn curl_gets_fixed_arguments() {
        let to = Path::new(r"C:\cache\a.zip.part");
        let args = curl_args("https://example.org/a.zip", to);
        let want: Vec<OsString> = [
            "-fsSL",
            "--retry",
            "3",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "-o",
            r"C:\cache\a.zip.part",
            "--url",
            "https://example.org/a.zip",
        ]
        .iter()
        .map(OsString::from)
        .collect();
        assert_eq!(args, want);
    }

    #[test]
    fn curl_refuses_before_running_for_a_bad_address() {
        let err = Curl.fetch("http://example.org/a.zip", Path::new("unused")).unwrap_err();
        assert!(err.contains("https"), "{err}");
    }

    /// The real curl with the fixed arguments, pointed at a plain-`http` server on this PC itself: curl must refuse
    /// it before connecting (so nothing is fetched and nothing reaches the server), and write no file.
    #[cfg(windows)]
    #[test]
    fn real_curl_refuses_plain_http_without_connecting() {
        use std::sync::atomic::AtomicBool;
        use std::time::Duration;

        let dir = crate::testdir::TempDir::new("curl-http");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let to = dir.path().join("a.zip.part");
        let args = curl_args(&format!("http://127.0.0.1:{port}/a.zip"), &to);
        let args: Vec<&std::ffi::OsStr> = args.iter().map(OsString::as_os_str).collect();
        let never = AtomicBool::new(false);
        let ran = crate::run::run(&Curl::program().unwrap(), &args, Duration::from_secs(20), &never).unwrap();
        assert!(!ran.status.success());
        assert!(!to.exists());
        match listener.accept() {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            other => panic!("curl reached the plain-http server: {other:?}"),
        }
    }
}
