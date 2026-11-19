//! SHA-256 checksums of files, in lower-case hex.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

/// The SHA-256 of a file's bytes, as 64 lower-case hex digits.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    sha256_reader(std::fs::File::open(path)?)
}

/// The SHA-256 of everything a reader gives, as 64 lower-case hex digits.
pub fn sha256_reader(mut r: impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

/// Bytes as lower-case hex.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether `s` is a SHA-256 written the way this crate writes one: 64 lower-case hex digits.
pub fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TempDir;

    #[test]
    fn a_known_file_hashes_right() {
        let dir = TempDir::new("sha");
        let f = dir.path().join("abc.txt");
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(
            sha256_file(&f).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_reader(&b""[..]).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn only_lower_case_hex_of_64_counts() {
        assert!(is_sha256(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
        assert!(!is_sha256(
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        ));
        assert!(!is_sha256("ba7816bf"));
        assert!(!is_sha256("not yet pinned"));
        assert!(!is_sha256(""));
    }
}
