//! Downloads. The real one goes through Windows' own `curl.exe`; tests use a `Download` that copies a local file.
//! A download takes a public `https` address only and never sends a user name, password or token.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Something that can put the file at `url` into the file `to`, stopping when it would be larger than `max_bytes`.
/// The text of an error says why, in plain words.
pub trait Download {
    fn fetch(&self, url: &str, max_bytes: u64, to: &Path) -> Result<(), String>;
}

/// The real download: `C:\Windows\System32\curl.exe`, part of Windows 11.
pub struct Curl;

impl Curl {
    /// Where Windows keeps its own curl. Never looked up on `PATH`.
    ///
    /// The standard library offers Windows' system folder only through the `SystemRoot` variable (asking Windows
    /// itself needs unsafe code), so the variable is checked: it must be a drive root's `Windows` folder, and
    /// `System32\curl.exe` must be an existing file there.
    pub fn program() -> Result<PathBuf, String> {
        #[cfg(windows)]
        {
            let root = std::env::var_os("SystemRoot").ok_or("Windows' folder is not known (SystemRoot is not set).")?;
            curl_under(&root)
        }
        #[cfg(not(windows))]
        {
            Err("Downloading works only on Windows.".to_string())
        }
    }
}

/// `<root>\System32\curl.exe`, when `root` is exactly `<drive letter>:\Windows` and that file exists.
pub(crate) fn curl_under(root: &OsStr) -> Result<PathBuf, String> {
    let text = root.to_string_lossy();
    let b = text.as_bytes();
    let drive_windows =
        b.len() == 10 && b[0].is_ascii_alphabetic() && b[1..3] == *b":\\" && text[3..].eq_ignore_ascii_case("Windows");
    if !drive_windows {
        return Err(format!(
            "Windows' folder is given as {text}, which is not a drive's Windows folder, so curl is not used from it."
        ));
    }
    let curl = PathBuf::from(root).join("System32").join("curl.exe");
    if curl.is_file() {
        Ok(curl)
    } else {
        Err(format!("Windows' own curl is missing from {}.", curl.display()))
    }
}

/// The longest a download may take before it is stopped.
const DOWNLOAD_LIMIT: Duration = Duration::from_secs(60 * 60);

impl Download for Curl {
    fn fetch(&self, url: &str, max_bytes: u64, to: &Path) -> Result<(), String> {
        check_public_url(url)?;
        let curl = Curl::program()?;
        let args = curl_args(url, max_bytes, to);
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
    let host = if let Some(v6) = authority.strip_prefix('[') {
        match v6.split_once(']') {
            Some((inside, _port)) => inside,
            None => return Err("The download address names no host.".to_string()),
        }
    } else {
        authority.split(':').next().unwrap_or("")
    };
    if host.is_empty() {
        return Err("The download address names no host.".to_string());
    }
    if !public_host(host) {
        return Err("The download address names this PC or a private network, so it is refused.".to_string());
    }
    Ok(())
}

/// Whether `host` (a name, a dotted IPv4 address, or an IPv6 address without its brackets) can be a public server:
/// not `localhost`, not a loopback, private, link-local or unspecified address, and not a number that curl would read
/// as an address in another form (such as `2130706433` or `0x7f.1`).
fn public_host(host: &str) -> bool {
    use std::net::{Ipv4Addr, Ipv6Addr};

    let name = host.trim_end_matches('.').to_ascii_lowercase();
    if name.is_empty() || name == "localhost" || name.ends_with(".localhost") {
        return false;
    }
    let v4_public = |a: Ipv4Addr| {
        let [x, y, ..] = a.octets();
        !(x == 0 || x == 127 || x == 10 || (x == 172 && (16..32).contains(&y)) || (x == 192 && y == 168))
            && !(x == 169 && y == 254)
            && !a.is_broadcast()
    };
    if let Ok(a) = name.parse::<Ipv4Addr>() {
        return v4_public(a);
    }
    if let Ok(a) = name.parse::<Ipv6Addr>() {
        if let Some(v4) = a.to_ipv4_mapped() {
            return v4_public(v4);
        }
        let first = a.segments()[0];
        return !(a.is_loopback() || a.is_unspecified() || (first & 0xffc0) == 0xfe80 || (first & 0xfe00) == 0xfc00);
    }
    // A name whose last part is a number (or starts with 0x) is an address in disguise; real names end in letters.
    let last = name.rsplit('.').next().unwrap_or("");
    !(last.bytes().all(|b| b.is_ascii_digit()) || last.starts_with("0x"))
}

/// curl's arguments, one by one: read no configuration file (`-q`, which must come first, so a `.curlrc` or
/// `_curlrc` can add nothing), fail on an HTTP error, no progress meter but errors shown, follow redirects, retry
/// three times, `https` only (also after a redirect), give up on a connection after 30 s or a transfer slower than
/// 1 KB/s for 2 minutes, stop at `max_bytes`, write to `to`.
pub(crate) fn curl_args(url: &str, max_bytes: u64, to: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "-q",
        "-fsSL",
        "--retry",
        "3",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "30",
        "--speed-limit",
        "1024",
        "--speed-time",
        "120",
        "--max-filesize",
    ]
    .map(OsString::from)
    .into();
    args.push(max_bytes.to_string().into());
    args.push("-o".into());
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
    fn this_pc_and_private_networks_are_refused() {
        for bad in [
            "https://localhost/a.zip",
            "https://LOCALHOST./a.zip",
            "https://files.localhost/a.zip",
            "https://127.0.0.1/a.zip",
            "https://127.8.9.10:8443/a.zip",
            "https://[::1]/a.zip",
            "https://[::1]:443/a.zip",
            "https://[::]/a.zip",
            "https://[::ffff:127.0.0.1]/a.zip",
            "https://[::ffff:192.168.1.1]/a.zip",
            "https://[fe80::1]/a.zip",
            "https://[fd12:3456::1]/a.zip",
            "https://10.1.2.3/a.zip",
            "https://172.16.0.1/a.zip",
            "https://172.31.255.255/a.zip",
            "https://192.168.0.10/a.zip",
            "https://169.254.1.1/a.zip",
            "https://0.0.0.0/a.zip",
            "https://2130706433/a.zip",
            "https://0x7f.1/a.zip",
            "https://127.1/a.zip",
        ] {
            let err = check_public_url(bad).expect_err(bad);
            assert!(
                err.contains("this PC or a private network") || err.contains("no host"),
                "{bad}: {err}"
            );
        }
        for good in [
            "https://172.15.0.1/a.zip",
            "https://172.32.0.1/a.zip",
            "https://8.8.8.8/a.zip",
            "https://[2001:db8::1]/a.zip",
            "https://example.org:8443/a.zip",
            "https://www.gyan.dev/a.zip",
        ] {
            assert!(check_public_url(good).is_ok(), "{good} was refused");
        }
    }

    #[test]
    fn curl_comes_only_from_a_drives_windows_folder() {
        for bad in [
            r"C:\Users\me\Windows",
            r"\\server\share\Windows",
            r"C:\Windows\..\Temp",
            "Windows",
            "",
        ] {
            assert!(curl_under(OsStr::new(bad)).is_err(), "{bad:?} was taken");
        }
        // A drive's Windows folder without curl in it is refused too.
        let missing = curl_under(OsStr::new(r"Q:\Windows")).unwrap_err();
        assert!(missing.contains("missing"), "{missing}");
        #[cfg(windows)]
        {
            let curl = Curl::program().unwrap();
            assert!(curl.ends_with(r"System32\curl.exe"), "{}", curl.display());
        }
    }

    #[test]
    fn curl_gets_fixed_arguments() {
        let to = Path::new(r"C:\cache\a.zip.part");
        let args = curl_args("https://example.org/a.zip", 123_456, to);
        let want: Vec<OsString> = [
            "-q",
            "-fsSL",
            "--retry",
            "3",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "30",
            "--speed-limit",
            "1024",
            "--speed-time",
            "120",
            "--max-filesize",
            "123456",
            "-o",
            r"C:\cache\a.zip.part",
            "--url",
            "https://example.org/a.zip",
        ]
        .iter()
        .map(OsString::from)
        .collect();
        assert_eq!(args, want);
        assert_eq!(args[0], "-q", "-q only works as curl's first argument");
    }

    #[test]
    fn curl_refuses_before_running_for_a_bad_address() {
        let err = Curl
            .fetch("http://example.org/a.zip", 1, Path::new("unused"))
            .unwrap_err();
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
        // `check_public_url` would refuse this address first; the test calls curl with the fixed arguments directly,
        // to prove curl's own https-only rule.
        let args = curl_args(&format!("http://127.0.0.1:{port}/a.zip"), 1000, &to);
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
