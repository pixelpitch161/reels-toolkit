//! Where the reference files are: found from this crate's own folder, so a test works from any working directory.

use std::path::{Path, PathBuf};

/// The repository root (the folder holding `studio/`), three levels above this crate's `Cargo.toml`
/// (`<repo>/studio/crates/bf-testdata`).
pub fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .ancestors()
        .nth(3)
        .unwrap_or_else(|| panic!("{} is not <repo>/studio/crates/bf-testdata", manifest.display()));
    assert!(
        root.join("studio").join("Cargo.toml").is_file(),
        "{} has no studio/Cargo.toml: bf-testdata is not at <repo>/studio/crates/bf-testdata",
        root.display()
    );
    root.to_path_buf()
}

/// `<repo>/studio/reference`.
pub fn reference_dir() -> PathBuf {
    repo_root().join("studio").join("reference")
}

/// `reference_dir().join(rel)`; panics naming the full path if the file or folder is missing.
pub fn reference(rel: &str) -> PathBuf {
    let path = reference_dir().join(rel);
    assert!(path.exists(), "reference file {} is missing", path.display());
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_holds_the_workspace() {
        let root = repo_root();
        assert!(root.join("studio").join("Cargo.toml").is_file(), "{}", root.display());
        assert_eq!(reference_dir(), root.join("studio").join("reference"));
    }

    #[test]
    fn reference_finds_a_file_that_exists() {
        assert_eq!(reference("manifest.json"), reference_dir().join("manifest.json"));
    }

    #[test]
    #[should_panic(expected = "no-such-file.npz")]
    fn reference_panics_naming_a_missing_file() {
        reference("no-such-file.npz");
    }
}
