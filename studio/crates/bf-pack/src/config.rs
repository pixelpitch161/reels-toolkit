//! `update.toml`, beside the installed exe: where the app's own copy of the source is, which repository it follows,
//! the tag prefix that marks a release, the build folder and the downloads folder (plan M6, Task 11; read by the
//! update engine, Task 12).
//!
//! A repository address that holds a user name or password is refused, by `write_config` and by `read_config`, and
//! the refusal never repeats the address.

use std::path::{Path, PathBuf};

use crate::PackError;
use crate::package::io_error;

/// What `update.toml` holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateConfig {
    /// The app's own copy of the repository (route A): `<data>\source`.
    pub source: PathBuf,
    /// The repository it follows (the owner's clone's `origin`), never with a user name or password in it.
    pub remote: String,
    /// Release tags start with this: `studio-v`.
    pub tag_prefix: String,
    /// Cargo's target folder for update builds: `<data>\build`.
    pub build_dir: PathBuf,
    /// Where the ffmpeg archive is kept: `<data>\downloads`.
    pub downloads: PathBuf,
}

/// The tag prefix every release has (plan M6, D1).
pub const TAG_PREFIX: &str = "studio-v";

/// The file's name beside the installed exe.
pub const CONFIG_NAME: &str = "update.toml";

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigFile {
    source: String,
    remote: String,
    tag_prefix: String,
    build_dir: String,
    downloads: String,
}

/// The sentence for an address with a user name or password in it. It never holds the address.
pub const REMOTE_WITH_SECRET: &str = "The repository's address holds a user name or password, so BALLFIGHT Studio does \
     not use or save it. Take the name and password out of the address of the copy on this PC, then try again.";

/// The sentence for an address that is not an https or ssh one.
pub const REMOTE_NOT_HTTPS_OR_SSH: &str = "The repository's address is not an https or ssh address, so BALLFIGHT \
     Studio does not use it for updates.";

/// The addresses updates come from: `https://...`, `ssh://...` or ssh's short form `git@host:path`, and nothing
/// else (no local folder, no `file://`, `git://` or plain `http://`), checked as `check_address` checks them too. The
/// error never repeats the address.
pub fn check_remote(url: &str) -> Result<(), PackError> {
    check_address(url)?;
    let lower = url.to_ascii_lowercase();
    let scp_form = !url.contains("://")
        && url.as_bytes().get(1) != Some(&b':')
        && !url.starts_with(['\\', '/', '.'])
        && url
            .split('/')
            .next()
            .is_some_and(|head| head.contains('@') && head.contains(':'));
    if lower.starts_with("https://") || lower.starts_with("ssh://") || scp_form {
        Ok(())
    } else {
        Err(PackError::Refused(REMOTE_NOT_HTTPS_OR_SSH.to_string()))
    }
}

/// Refuses a repository address that could hold a secret: a user name or password before the host
/// (`https://name:secret@host/...`, `https://name@host/...`), except `git@` for ssh, which is how ssh names its
/// user and can hold no password. Also refuses an empty address and one with spaces or control characters. A local
/// folder passes this check (tests, and the install's own reading of the owner's clone); `check_remote` does not take
/// it. The error never repeats the address.
pub fn check_address(url: &str) -> Result<(), PackError> {
    // A folder on this PC may have spaces in its name; a web address may not.
    let spaced_url = url.contains("://") && url.contains(' ');
    if url.is_empty() || url.starts_with('-') || spaced_url || url.chars().any(char::is_control) {
        return Err(PackError::Refused(
            "The repository's address is empty or not written as an address, so it is not used.".to_string(),
        ));
    }
    let user = match url.split_once("://") {
        Some((scheme, rest)) => {
            let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
            authority
                .rsplit_once('@')
                .map(|(user, _)| (scheme.to_ascii_lowercase(), user))
        }
        // A local folder (`C:\...`, `\\server\...`) or ssh's short form `user@host:path`, where anything before the
        // last `@` ahead of the path is the user part.
        None => {
            let looks_local = url.as_bytes().get(1) == Some(&b':') || url.starts_with('\\') || url.starts_with('/');
            if looks_local {
                None
            } else {
                let head = url.split('/').next().unwrap_or("");
                head.rsplit_once('@').map(|(user, _)| ("ssh".to_string(), user))
            }
        }
    };
    match user {
        None => Ok(()),
        Some((scheme, "git")) if scheme == "ssh" || scheme == "git+ssh" => Ok(()),
        Some(_) => Err(PackError::Refused(REMOTE_WITH_SECRET.to_string())),
    }
}

fn path_text<'a>(what: &str, p: &'a Path) -> Result<&'a str, PackError> {
    p.to_str()
        .ok_or_else(|| PackError::Refused(format!("The {what} folder's name cannot be written in update.toml.")))
}

/// Reads `update.toml`. A missing or unreadable file, a file in another shape, or an address with a secret in it is
/// an error in words.
pub fn read_config(path: &Path) -> Result<UpdateConfig, PackError> {
    let text = std::fs::read_to_string(path).map_err(|e| io_error("read", path, &e))?;
    let file: ConfigFile = toml::from_str(&text).map_err(|_| {
        PackError::Refused(format!(
            "{} is not an update.toml that BALLFIGHT Studio wrote, so it is not used.",
            path.display()
        ))
    })?;
    check_remote(&file.remote)?;
    Ok(UpdateConfig {
        source: PathBuf::from(file.source),
        remote: file.remote,
        tag_prefix: file.tag_prefix,
        build_dir: PathBuf::from(file.build_dir),
        downloads: PathBuf::from(file.downloads),
    })
}

/// Writes `update.toml` (through a `.part` file, renamed when complete). Refuses an address with a user name or
/// password in it, without writing anything.
pub fn write_config(path: &Path, c: &UpdateConfig) -> Result<(), PackError> {
    check_remote(&c.remote)?;
    let file = ConfigFile {
        source: path_text("source", &c.source)?.to_string(),
        remote: c.remote.clone(),
        tag_prefix: c.tag_prefix.clone(),
        build_dir: path_text("build", &c.build_dir)?.to_string(),
        downloads: path_text("downloads", &c.downloads)?.to_string(),
    };
    let body = toml::to_string(&file)
        .map_err(|_| PackError::Refused("The update settings could not be written as text.".to_string()))?;
    let text = format!(
        "# Written by BALLFIGHT Studio's install: where updates come from and are built. Not for editing.\n{body}"
    );
    let mut part = path.as_os_str().to_os_string();
    part.push(".part");
    let part = PathBuf::from(part);
    let written = std::fs::write(&part, text)
        .map_err(|e| io_error("write", &part, &e))
        .and_then(|()| std::fs::rename(&part, path).map_err(|e| io_error("rename", &part, &e)));
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TempDir;

    fn sample(root: &Path) -> UpdateConfig {
        UpdateConfig {
            source: root.join("data").join("source"),
            remote: "https://github.com/someone/InstrumentBattleMachine.git".to_string(),
            tag_prefix: TAG_PREFIX.to_string(),
            build_dir: root.join("data").join("build"),
            downloads: root.join("data").join("downloads"),
        }
    }

    #[test]
    fn a_config_reads_back() {
        let dir = TempDir::new("config-back");
        let path = dir.path().join(CONFIG_NAME);
        let c = sample(dir.path());
        write_config(&path, &c).unwrap();
        assert_eq!(read_config(&path).unwrap(), c);
        assert!(!dir.path().join("update.toml.part").exists());
    }

    #[test]
    fn a_token_in_the_remote_is_refused() {
        let dir = TempDir::new("config-token");
        let path = dir.path().join(CONFIG_NAME);
        for secret in [
            "https://someone:ghp_SECRETTOKEN123@github.com/someone/repo.git",
            "https://ghp_SECRETTOKEN123@github.com/someone/repo.git",
            "http://someone:SECRETTOKEN123@example.org/repo",
            "ssh://someone:SECRETTOKEN123@example.org/repo",
            "someone:SECRETTOKEN123@example.org:repo",
        ] {
            let mut c = sample(dir.path());
            c.remote = secret.to_string();
            let err = write_config(&path, &c).unwrap_err().to_string();
            assert!(!err.contains("SECRETTOKEN123"), "{err}");
            assert!(!err.contains("someone"), "{err}");
            assert!(err.contains("user name or password"), "{err}");
            assert!(!path.exists(), "nothing is written for {secret:?}");
            assert!(!dir.path().join("update.toml.part").exists());
        }
        // A file someone wrote by hand with a secret in it is refused too, and the words do not repeat it.
        std::fs::write(
            &path,
            "source = 'C:/s'\nremote = 'https://x:SECRETTOKEN123@github.com/r'\ntag_prefix = 'studio-v'\n\
             build_dir = 'C:/b'\ndownloads = 'C:/d'\n",
        )
        .unwrap();
        let err = read_config(&path).unwrap_err().to_string();
        assert!(!err.contains("SECRETTOKEN123"), "{err}");
    }

    #[test]
    fn plain_addresses_are_taken() {
        for fine in [
            "https://github.com/someone/repo.git",
            "HTTPS://github.com/someone/repo.git",
            "git@github.com:someone/repo.git",
            "ssh://git@github.com/someone/repo.git",
        ] {
            check_remote(fine).unwrap_or_else(|e| panic!("{fine}: {e}"));
        }
        for bad in ["", "https://github.com/a b", "https://github.com/a\nb"] {
            assert!(check_remote(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn only_https_and_ssh_remotes_are_taken() {
        for local_or_plain in [
            r"F:\!Ai Stuff\InstrumentBattleMachine",
            "F:/!Ai Stuff/InstrumentBattleMachine",
            r"\\server\share\repo",
            "/c/repos/x",
            "./x",
            "file:///C:/repos/x",
            "git://example.org/x.git",
            "http://example.org/x.git",
            "ext::sh -c touch% x",
        ] {
            assert_eq!(
                check_remote(local_or_plain),
                Err(PackError::Refused(REMOTE_NOT_HTTPS_OR_SSH.to_string())),
                "{local_or_plain}"
            );
            // A config naming one is refused when written and when read.
            let dir = TempDir::new("config-local");
            let path = dir.path().join(CONFIG_NAME);
            let mut c = sample(dir.path());
            c.remote = local_or_plain.to_string();
            assert!(write_config(&path, &c).is_err());
            assert!(!path.exists());
        }
        // The address form check alone still takes a local folder (the owner's clone is read that way).
        check_address(r"F:\!Ai Stuff\InstrumentBattleMachine").unwrap();
    }

    #[test]
    fn a_broken_config_says_so_in_words() {
        let dir = TempDir::new("config-broken");
        let path = dir.path().join(CONFIG_NAME);
        let err = read_config(&path).unwrap_err().to_string();
        assert!(err.contains("update.toml"), "{err}");
        std::fs::write(&path, "source = 3\n").unwrap();
        let err = read_config(&path).unwrap_err().to_string();
        assert!(err.contains("update.toml") && err.ends_with('.'), "{err}");
    }
}
