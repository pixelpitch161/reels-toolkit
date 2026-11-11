//! The pinned ffmpeg (plan M6, decision D3): `studio/packaging/ffmpeg.toml` names the archive's address, size and
//! SHA-256 and the SHA-256 of each file the app needs from it. `ensure` downloads and checks it into a cache folder;
//! `place` copies the checked files, plus `ABOUT.txt`, into the folder that becomes the app's `ffmpeg\`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::fetch::Download;
use crate::sha::{is_sha256, sha256_file};
use crate::{PackError, io_error};

/// The words `archive_sha256` holds until the archive's checksum has been read from the real archive. While it does,
/// `ensure` and `place` refuse: nothing is downloaded, and no file can pass for the archive.
pub const NOT_PINNED: &str = "not yet pinned";

/// The ffmpeg the app ships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfmpegPin {
    pub version: String,
    pub build: String,
    pub url: String,
    /// 64 lower-case hex digits, or `NOT_PINNED`.
    pub archive_sha256: String,
    pub archive_bytes: u64,
    pub licence: String,
    pub source_note: String,
    pub files: Vec<PinnedFile>,
}

/// One file the app needs from the archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinnedFile {
    /// Its name in the app's `ffmpeg\` folder, for example "ffmpeg.exe".
    pub name: String,
    /// Its path inside the archive, with forward slashes.
    pub in_archive: String,
    pub sha256: String,
}

impl FfmpegPin {
    /// Whether the archive's checksum and size are known, so that a download can be checked.
    pub fn archive_pinned(&self) -> bool {
        is_sha256(&self.archive_sha256) && self.archive_bytes > 0
    }

    /// The archive's file name: the last part of its address.
    pub fn archive_name(&self) -> &str {
        self.url.rsplit('/').next().unwrap_or("")
    }

    /// The folder in the cache that holds this archive's checked files.
    fn unpacked_name(&self) -> String {
        format!("{}-{}", self.version, &self.archive_sha256[..12])
    }

    fn refuse_unpinned(&self) -> Result<(), PackError> {
        if self.archive_pinned() {
            Ok(())
        } else {
            Err(PackError::Refused(format!(
                "The ffmpeg {} archive's checksum is not pinned yet in packaging/ffmpeg.toml, so nothing is \
                 downloaded or used.",
                self.version
            )))
        }
    }
}

/// The pin file as written.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PinFile {
    version: String,
    build: String,
    url: String,
    archive_sha256: String,
    archive_bytes: u64,
    licence: String,
    source_note: String,
    files: Vec<PinFileEntry>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PinFileEntry {
    name: String,
    in_archive: String,
    sha256: String,
}

/// The pin this build was made with (`studio/packaging/ffmpeg.toml`).
pub fn pin() -> FfmpegPin {
    parse_pin(include_str!("../../../packaging/ffmpeg.toml")).expect("studio/packaging/ffmpeg.toml is a good pin")
}

/// A name made of letters, digits, '.', '-' and '_' only, not starting with '.': safe as a file or folder name.
fn plain_name(s: &str) -> bool {
    !s.is_empty() && !s.starts_with('.') && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// Reads a pin file's text (also another version's, read from its source tree) and checks every value.
pub fn parse_pin(text: &str) -> Result<FfmpegPin, PackError> {
    let bad = |why: String| Err(PackError::Refused(format!("The ffmpeg pin is not usable: {why}.")));
    let raw: PinFile = match toml::from_str(text) {
        Ok(raw) => raw,
        Err(e) => return bad(format!("it could not be read ({})", e.message())),
    };
    if !plain_name(&raw.version) {
        return bad(format!("the version \"{}\" is not a plain version", raw.version));
    }
    if raw.build.trim().is_empty() {
        return bad("it names no build".into());
    }
    if let Err(why) = crate::fetch::check_public_url(&raw.url) {
        return bad(format!("its address is refused ({})", why.trim_end_matches('.')));
    }
    let archive = raw.url.rsplit('/').next().unwrap_or("");
    if !plain_name(archive) || !archive.ends_with(".zip") {
        return bad("its address does not end in a .zip file name".into());
    }
    if raw.archive_sha256 == NOT_PINNED {
        if raw.archive_bytes != 0 {
            return bad("the archive has a size but no checksum".into());
        }
    } else if !is_sha256(&raw.archive_sha256) {
        return bad(format!(
            "the archive's checksum is neither 64 lower-case hex digits nor \"{NOT_PINNED}\""
        ));
    } else if raw.archive_bytes == 0 {
        return bad("the archive has a checksum but no size".into());
    }
    if !raw.licence.starts_with("GPL") {
        return bad(format!(
            "the licence \"{}\" is not a GPL licence, and only a GPL build has libx264",
            raw.licence
        ));
    }
    if raw.source_note.trim().is_empty() {
        return bad("it does not say where the source code is".into());
    }
    if raw.files.is_empty() {
        return bad("it names no files".into());
    }
    let mut files = Vec::with_capacity(raw.files.len());
    for f in raw.files {
        if !plain_name(&f.name) || f.name.eq_ignore_ascii_case(ABOUT) {
            return bad(format!("\"{}\" is not a plain file name", f.name));
        }
        if files.iter().any(|g: &PinnedFile| g.name.eq_ignore_ascii_case(&f.name)) {
            return bad(format!("{} is named twice", f.name));
        }
        if f.in_archive.is_empty() || f.in_archive.contains('\\') || f.in_archive.ends_with('/') {
            return bad(format!("the path of {} inside the archive is not a file path", f.name));
        }
        if !is_sha256(&f.sha256) {
            return bad(format!("the checksum of {} is not 64 lower-case hex digits", f.name));
        }
        files.push(PinnedFile {
            name: f.name,
            in_archive: f.in_archive,
            sha256: f.sha256,
        });
    }
    Ok(FfmpegPin {
        version: raw.version,
        build: raw.build,
        url: raw.url,
        archive_sha256: raw.archive_sha256,
        archive_bytes: raw.archive_bytes,
        licence: raw.licence,
        source_note: raw.source_note,
        files,
    })
}

/// The name of the note written beside the placed files.
const ABOUT: &str = "ABOUT.txt";

/// Makes sure `cache` holds the pinned archive and, unpacked beside it, every pinned file, all checked; gives the
/// folder of checked files. A matching archive already in `cache` is not downloaded again; an unpacked folder already
/// there is checked again, not trusted. Nothing outside `cache` is written.
pub fn ensure(pin: &FfmpegPin, cache: &Path, dl: &dyn Download) -> Result<PathBuf, PackError> {
    pin.refuse_unpinned()?;
    crate::fetch::check_public_url(&pin.url).map_err(PackError::Refused)?;
    std::fs::create_dir_all(cache).map_err(|e| io_error("make the folder", cache, &e))?;

    let archive = cache.join(pin.archive_name());
    if !archive_matches(pin, &archive)? {
        download(pin, &archive, dl)?;
    }

    let folder = cache.join(pin.unpacked_name());
    if folder.is_dir() {
        if files_match(pin, &folder)? {
            return Ok(folder);
        }
        std::fs::remove_dir_all(&folder).map_err(|e| io_error("remove the folder", &folder, &e))?;
    }
    unpack(pin, &archive, &folder)?;
    Ok(folder)
}

/// Whether `archive` is there with the pinned size and checksum. A wrong one is removed.
fn archive_matches(pin: &FfmpegPin, archive: &Path) -> Result<bool, PackError> {
    let Ok(meta) = std::fs::metadata(archive) else {
        return Ok(false);
    };
    let matches = meta.is_file()
        && meta.len() == pin.archive_bytes
        && sha256_file(archive).map_err(|e| io_error("read", archive, &e))? == pin.archive_sha256;
    if !matches {
        std::fs::remove_file(archive).map_err(|e| io_error("remove", archive, &e))?;
    }
    Ok(matches)
}

/// Downloads the archive to `<name>.part`, checks its size and checksum, and only then gives it its name. Any
/// failure removes the `.part` file.
fn download(pin: &FfmpegPin, archive: &Path, dl: &dyn Download) -> Result<(), PackError> {
    let part = with_suffix(archive, ".part");
    remove_if_there(&part)?;
    let checked = (|| {
        dl.fetch(&pin.url, &part).map_err(|e| {
            PackError::Download(format!(
                "The ffmpeg {} archive could not be downloaded from {}: {}.",
                pin.version,
                pin.url,
                e.trim_end_matches('.')
            ))
        })?;
        let got = std::fs::metadata(&part)
            .map_err(|_| {
                PackError::Download(format!(
                    "The ffmpeg {} archive could not be downloaded from {}: no file came down.",
                    pin.version, pin.url
                ))
            })?
            .len();
        if got != pin.archive_bytes {
            return Err(PackError::Download(format!(
                "The ffmpeg {} archive that came down has {got} bytes instead of {}, so it was removed.",
                pin.version, pin.archive_bytes
            )));
        }
        let sha = sha256_file(&part).map_err(|e| io_error("read", &part, &e))?;
        if sha != pin.archive_sha256 {
            return Err(PackError::Checksum {
                what: format!("The downloaded ffmpeg {} archive", pin.version),
                want: pin.archive_sha256.clone(),
                got: sha,
            });
        }
        std::fs::rename(&part, archive).map_err(|e| io_error("rename", &part, &e))
    })();
    if checked.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    checked
}

/// Whether every pinned file in `folder` has its pinned checksum.
fn files_match(pin: &FfmpegPin, folder: &Path) -> Result<bool, PackError> {
    for f in &pin.files {
        let path = folder.join(&f.name);
        if !path.is_file() || sha256_file(&path).map_err(|e| io_error("read", &path, &e))? != f.sha256 {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Unpacks the pinned files from `archive` into `<folder>.part`, checking each as it is written, then gives the
/// folder its name. Any failure removes the half-made folder.
fn unpack(pin: &FfmpegPin, archive: &Path, folder: &Path) -> Result<(), PackError> {
    let part = with_suffix(folder, ".part");
    if part.exists() {
        std::fs::remove_dir_all(&part).map_err(|e| io_error("remove the folder", &part, &e))?;
    }
    let done = (|| {
        std::fs::create_dir_all(&part).map_err(|e| io_error("make the folder", &part, &e))?;
        let file = std::fs::File::open(archive).map_err(|e| io_error("open", archive, &e))?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| {
            PackError::Archive(format!(
                "The ffmpeg {} archive could not be read as a zip file: {e}.",
                pin.version
            ))
        })?;
        for f in &pin.files {
            let mut entry = zip.by_name(&f.in_archive).map_err(|e| {
                PackError::Archive(format!(
                    "{} is missing from the ffmpeg {} archive ({}): {e}.",
                    f.in_archive, pin.version, f.name
                ))
            })?;
            let to = part.join(&f.name);
            let sha = copy_hashing(&mut entry, &to).map_err(|e| {
                PackError::Archive(format!(
                    "{} could not be unpacked from the ffmpeg archive: {e}.",
                    f.name
                ))
            })?;
            if sha != f.sha256 {
                return Err(PackError::Checksum {
                    what: format!("{} inside the ffmpeg {} archive", f.name, pin.version),
                    want: f.sha256.clone(),
                    got: sha,
                });
            }
        }
        drop(zip);
        std::fs::rename(&part, folder).map_err(|e| io_error("rename the folder", &part, &e))
    })();
    if done.is_err() {
        let _ = std::fs::remove_dir_all(&part);
    }
    done
}

/// Copies everything `from` gives into a new file `to`, and gives the SHA-256 of what was written.
fn copy_hashing(from: &mut impl Read, to: &Path) -> std::io::Result<String> {
    let mut out = std::fs::File::create(to)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = match from.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
    }
    out.sync_all()?;
    Ok(crate::sha::hex(&hasher.finalize()))
}

/// Copies the checked files from `verified` (what `ensure` gave) into `dest`, which becomes the app's `ffmpeg\`
/// folder, checking each copy, and writes `ABOUT.txt` beside them. Nothing outside `dest` is written.
pub fn place(pin: &FfmpegPin, verified: &Path, dest: &Path) -> Result<(), PackError> {
    pin.refuse_unpinned()?;
    std::fs::create_dir_all(dest).map_err(|e| io_error("make the folder", dest, &e))?;
    for f in &pin.files {
        let target = dest.join(&f.name);
        if target.is_file() && sha256_file(&target).map_err(|e| io_error("read", &target, &e))? == f.sha256 {
            continue;
        }
        let from = verified.join(&f.name);
        let part = dest.join(format!(".{}.part", f.name));
        let placed = (|| {
            let mut src = std::fs::File::open(&from).map_err(|e| io_error("open", &from, &e))?;
            let sha = copy_hashing(&mut src, &part).map_err(|e| io_error("write", &part, &e))?;
            if sha != f.sha256 {
                return Err(PackError::Checksum {
                    what: format!("{} in {}", f.name, verified.display()),
                    want: f.sha256.clone(),
                    got: sha,
                });
            }
            std::fs::rename(&part, &target).map_err(|e| io_error("rename", &part, &e))
        })();
        if placed.is_err() {
            let _ = std::fs::remove_file(&part);
        }
        placed?;
    }
    let about = dest.join(ABOUT);
    let part = dest.join(format!(".{ABOUT}.part"));
    let written = std::fs::write(&part, about_text(pin))
        .map_err(|e| io_error("write", &part, &e))
        .and_then(|()| std::fs::rename(&part, &about).map_err(|e| io_error("rename", &part, &e)));
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    written
}

/// What `ABOUT.txt` says: what this ffmpeg is, where it came from, its licence and where its source is.
fn about_text(pin: &FfmpegPin) -> String {
    let mut s = String::new();
    s.push_str(&format!("ffmpeg {} ({})\r\n\r\n", pin.version, pin.build));
    s.push_str(
        "BALLFIGHT Studio runs this ffmpeg as a separate program: it starts ffmpeg.exe and ffprobe.exe and talks to \
         them only through their arguments and pipes.\r\n\r\n",
    );
    s.push_str(&format!("Archive: {}\r\n", pin.url));
    s.push_str(&format!("Archive SHA-256: {}\r\n", pin.archive_sha256));
    s.push_str(&format!("Archive size: {} bytes\r\n\r\n", pin.archive_bytes));
    s.push_str(&format!(
        "Licence: {}. The licence text is LICENSE.txt in this folder, unchanged from the archive.\r\n",
        pin.licence
    ));
    s.push_str(&format!("Source code: {}\r\n\r\n", pin.source_note));
    s.push_str("Files:\r\n");
    for f in &pin.files {
        s.push_str(&format!("  {}  {}\r\n", f.name, f.sha256));
    }
    s
}

/// `path` with `suffix` added to its file name.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn remove_if_there(path: &Path) -> Result<(), PackError> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(io_error("remove", path, &e)),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha::{is_sha256, sha256_file};
    use crate::testdir::TempDir;
    use std::cell::Cell;
    use std::io::Write;

    /// A download that copies a local file (or a cut-short part of it), and counts its calls.
    struct LocalFile {
        from: PathBuf,
        keep: Option<usize>,
        calls: Cell<usize>,
    }

    impl LocalFile {
        fn new(from: &Path) -> LocalFile {
            LocalFile {
                from: from.to_path_buf(),
                keep: None,
                calls: Cell::new(0),
            }
        }
    }

    impl Download for LocalFile {
        fn fetch(&self, _url: &str, to: &Path) -> Result<(), String> {
            self.calls.set(self.calls.get() + 1);
            let mut bytes = std::fs::read(&self.from).map_err(|e| e.to_string())?;
            if let Some(n) = self.keep {
                bytes.truncate(n);
            }
            std::fs::write(to, bytes).map_err(|e| e.to_string())
        }
    }

    /// A download that never works, as on a PC with no network.
    struct NoNetwork;

    impl Download for NoNetwork {
        fn fetch(&self, _url: &str, _to: &Path) -> Result<(), String> {
            Err("could not resolve host".into())
        }
    }

    const FFMPEG: &[u8] = b"pretend ffmpeg.exe bytes";
    const FFPROBE: &[u8] = b"pretend ffprobe.exe bytes";
    const LICENCE: &[u8] = b"GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007\n";

    fn hex(bytes: &[u8]) -> String {
        crate::sha::sha256_reader(bytes).unwrap()
    }

    /// Builds a zip in `dir` laid out like the real archive (plus a file the app does not need).
    fn make_archive(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (inside, bytes) in [
            ("ffmpeg-9.0.2-full_build/bin/ffmpeg.exe", FFMPEG),
            ("ffmpeg-9.0.2-full_build/bin/ffprobe.exe", FFPROBE),
            ("ffmpeg-9.0.2-full_build/bin/ffplay.exe", &b"not needed"[..]),
            ("ffmpeg-9.0.2-full_build/LICENSE", LICENCE),
        ] {
            zip.start_file(inside, opts).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    /// A pin that matches the archive `make_archive` built.
    fn pin_for(archive: &Path) -> FfmpegPin {
        FfmpegPin {
            version: "9.0.2".into(),
            build: "full_build-www.gyan.dev".into(),
            url: "https://example.org/builds/ffmpeg-9.0.2-full_build.zip".into(),
            archive_sha256: sha256_file(archive).unwrap(),
            archive_bytes: std::fs::metadata(archive).unwrap().len(),
            licence: "GPL-3.0-or-later".into(),
            source_note: "The source is published at https://example.org/source.".into(),
            files: vec![
                PinnedFile {
                    name: "ffmpeg.exe".into(),
                    in_archive: "ffmpeg-9.0.2-full_build/bin/ffmpeg.exe".into(),
                    sha256: hex(FFMPEG),
                },
                PinnedFile {
                    name: "ffprobe.exe".into(),
                    in_archive: "ffmpeg-9.0.2-full_build/bin/ffprobe.exe".into(),
                    sha256: hex(FFPROBE),
                },
                PinnedFile {
                    name: "LICENSE.txt".into(),
                    in_archive: "ffmpeg-9.0.2-full_build/LICENSE".into(),
                    sha256: hex(LICENCE),
                },
            ],
        }
    }

    /// A source folder (where the fake download copies from) and a cache folder, side by side.
    fn setup(name: &str) -> (TempDir, PathBuf, FfmpegPin, PathBuf) {
        let dir = TempDir::new(name);
        let src = dir.path().join("server");
        std::fs::create_dir_all(&src).unwrap();
        let archive = make_archive(&src, "ffmpeg-9.0.2-full_build.zip");
        let pin = pin_for(&archive);
        let cache = dir.path().join("cache");
        (dir, archive, pin, cache)
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = match std::fs::read_dir(dir) {
            Ok(rd) => rd
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => Vec::new(),
        };
        v.sort();
        v
    }

    #[test]
    fn the_pin_reads() {
        let p = pin();
        assert_eq!(p.version, "9.0.2");
        let names: Vec<&str> = p.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["ffmpeg.exe", "ffprobe.exe", "LICENSE.txt"]);
        for f in &p.files {
            assert!(is_sha256(&f.sha256), "{}: {}", f.name, f.sha256);
        }
        // The archive's own checksum is either pinned (64 hex digits and a size) or plainly not pinned yet.
        if p.archive_pinned() {
            assert!(is_sha256(&p.archive_sha256));
            assert!(p.archive_bytes > 0);
        } else {
            assert_eq!(p.archive_sha256, NOT_PINNED);
        }
        assert!(p.licence.contains("GPL"), "{}", p.licence);
        assert!(p.url.starts_with("https://"));
        assert_eq!(p.archive_name(), "ffmpeg-9.0.2-full_build.zip");
        assert!(!p.source_note.is_empty());
    }

    #[test]
    fn a_bad_pin_is_refused() {
        let good = include_str!("../../../packaging/ffmpeg.toml");
        assert!(parse_pin(good).is_ok());
        let sha = "589e50b766d251afdf181dd664d40bd94407e200b019989fd7468c7d118a28d0";
        for (from, to) in [
            ("url = \"https://", "url = \"http://"),
            ("url = \"https://", "url = \"https://name:secret@"),
            ("archive_sha256 = \"not yet pinned\"", "archive_sha256 = \"0000\""),
            ("archive_sha256 = \"not yet pinned\"", "archive_sha256 = \"\""),
            (sha, "589E50B766D251AFDF181DD664D40BD94407E200B019989FD7468C7D118A28D0"),
            ("name = \"ffmpeg.exe\"", "name = \"..\\\\ffmpeg.exe\""),
            ("name = \"ffmpeg.exe\"", "name = \"ffprobe.exe\""),
            ("name = \"ffmpeg.exe\"", "name = \"ABOUT.txt\""),
            ("licence = \"GPL-3.0-or-later\"", "licence = \"LGPL-2.1\""),
            ("version = \"9.0.2\"", "version = \"9.0.2/../x\""),
        ] {
            assert!(good.contains(from), "{from}");
            let bad = good.replacen(from, to, 1);
            assert!(parse_pin(&bad).is_err(), "taken after {from:?} -> {to:?}");
        }
        // A pinned archive needs its size too.
        let pinned_without_size = good.replacen(
            "archive_sha256 = \"not yet pinned\"",
            &format!("archive_sha256 = \"{sha}\""),
            1,
        );
        assert!(parse_pin(&pinned_without_size).is_err());
        let pinned = pinned_without_size.replacen("archive_bytes = 0", "archive_bytes = 123", 1);
        assert!(parse_pin(&pinned).unwrap().archive_pinned());
    }

    #[test]
    fn an_unpinned_archive_is_never_fetched() {
        let (_dir, archive, mut pin, cache) = setup("unpinned");
        pin.archive_sha256 = NOT_PINNED.into();
        pin.archive_bytes = 0;
        let dl = LocalFile::new(&archive);
        let err = ensure(&pin, &cache, &dl).unwrap_err();
        assert!(matches!(err, PackError::Refused(_)), "{err:?}");
        assert!(err.to_string().contains("not pinned"), "{err}");
        assert_eq!(dl.calls.get(), 0);
        assert!(names_in(&cache).is_empty(), "{:?}", names_in(&cache));
        // Not even an archive already in the cache is used, since nothing could check it.
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::copy(&archive, cache.join(pin.archive_name())).unwrap();
        assert!(matches!(ensure(&pin, &cache, &dl), Err(PackError::Refused(_))));
        let out = cache.join("out");
        assert!(matches!(place(&pin, &cache, &out), Err(PackError::Refused(_))));
        assert!(!out.exists());
    }

    #[test]
    fn a_cached_archive_is_not_fetched_again() {
        let (_dir, archive, pin, cache) = setup("cached");
        let dl = LocalFile::new(&archive);
        let got = ensure(&pin, &cache, &dl).unwrap();
        assert_eq!(dl.calls.get(), 1);
        assert_eq!(got, cache.join(format!("9.0.2-{}", &pin.archive_sha256[..12])));
        for f in &pin.files {
            assert_eq!(sha256_file(&got.join(&f.name)).unwrap(), f.sha256);
        }
        assert_eq!(names_in(&got), ["LICENSE.txt", "ffmpeg.exe", "ffprobe.exe"]);
        let dl2 = LocalFile::new(&archive);
        assert_eq!(ensure(&pin, &cache, &dl2).unwrap(), got);
        assert_eq!(dl2.calls.get(), 0);
        // Only the archive and its unpacked folder are in the cache.
        assert_eq!(
            names_in(&cache),
            [
                got.file_name().unwrap().to_string_lossy().into_owned(),
                pin.archive_name().to_string(),
            ]
        );
    }

    #[test]
    fn a_wrong_cached_archive_is_fetched_again() {
        let (_dir, archive, pin, cache) = setup("wrong-cached");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join(pin.archive_name()), b"an old, broken download").unwrap();
        let dl = LocalFile::new(&archive);
        ensure(&pin, &cache, &dl).unwrap();
        assert_eq!(dl.calls.get(), 1);
        assert_eq!(
            sha256_file(&cache.join(pin.archive_name())).unwrap(),
            pin.archive_sha256
        );
    }

    #[test]
    fn a_wrong_archive_is_refused_and_removed() {
        let (dir, _archive, pin, cache) = setup("wrong-archive");
        // Another archive of the same size: the size passes, the checksum does not.
        let other_dir = dir.path().join("other");
        std::fs::create_dir_all(&other_dir).unwrap();
        let other = other_dir.join("other.zip");
        let mut bytes = std::fs::read(_archive).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        std::fs::write(&other, &bytes).unwrap();
        let err = ensure(&pin, &cache, &LocalFile::new(&other)).unwrap_err();
        match &err {
            PackError::Checksum { want, got, .. } => {
                assert_eq!(want, &pin.archive_sha256);
                assert_eq!(got, &sha256_file(&other).unwrap());
            }
            e => panic!("{e:?}"),
        }
        assert!(err.to_string().contains(&pin.archive_sha256), "{err}");
        assert!(names_in(&cache).is_empty(), "{:?}", names_in(&cache));
    }

    #[test]
    fn a_short_download_is_refused() {
        let (_dir, archive, pin, cache) = setup("short");
        let mut dl = LocalFile::new(&archive);
        dl.keep = Some(pin.archive_bytes as usize / 2);
        let err = ensure(&pin, &cache, &dl).unwrap_err();
        assert!(matches!(err, PackError::Download(_)), "{err:?}");
        assert!(err.to_string().contains(&pin.archive_bytes.to_string()), "{err}");
        assert!(names_in(&cache).is_empty(), "{:?}", names_in(&cache));
    }

    #[test]
    fn a_wrong_file_inside_is_refused() {
        let (_dir, archive, mut pin, cache) = setup("wrong-inside");
        pin.files[1].sha256 = hex(b"some other ffprobe");
        let err = ensure(&pin, &cache, &LocalFile::new(&archive)).unwrap_err();
        match &err {
            PackError::Checksum { what, want, got } => {
                assert!(what.contains("ffprobe.exe"), "{what}");
                assert_eq!(want, &pin.files[1].sha256);
                assert_eq!(got, &hex(FFPROBE));
            }
            e => panic!("{e:?}"),
        }
        // The archive matched its pin and stays; no unpacked folder (finished or half-made) is left.
        assert_eq!(names_in(&cache), [pin.archive_name().to_string()]);
    }

    #[test]
    fn a_file_missing_from_the_archive_is_refused() {
        let (_dir, archive, mut pin, cache) = setup("missing-inside");
        pin.files[0].in_archive = "ffmpeg-9.0.2-full_build/bin/nothing.exe".into();
        let err = ensure(&pin, &cache, &LocalFile::new(&archive)).unwrap_err();
        assert!(matches!(err, PackError::Archive(_)), "{err:?}");
        assert!(err.to_string().contains("nothing.exe"), "{err}");
        assert_eq!(names_in(&cache), [pin.archive_name().to_string()]);
    }

    #[test]
    fn a_tampered_unpacked_file_is_unpacked_again() {
        let (_dir, archive, pin, cache) = setup("tampered");
        let got = ensure(&pin, &cache, &LocalFile::new(&archive)).unwrap();
        std::fs::write(got.join("ffmpeg.exe"), b"changed after unpacking").unwrap();
        std::fs::remove_file(got.join("LICENSE.txt")).unwrap();
        let dl = LocalFile::new(&archive);
        assert_eq!(ensure(&pin, &cache, &dl).unwrap(), got);
        assert_eq!(dl.calls.get(), 0);
        for f in &pin.files {
            assert_eq!(sha256_file(&got.join(&f.name)).unwrap(), f.sha256);
        }
    }

    #[test]
    fn no_download_says_so_in_words() {
        let (_dir, _archive, pin, cache) = setup("no-network");
        let err = ensure(&pin, &cache, &NoNetwork).unwrap_err();
        assert!(matches!(err, PackError::Download(_)), "{err:?}");
        let text = err.to_string();
        assert!(text.contains("could not be downloaded"), "{text}");
        assert!(text.contains("could not resolve host"), "{text}");
        assert!(names_in(&cache).is_empty(), "{:?}", names_in(&cache));
    }

    #[test]
    fn place_writes_about_and_licence() {
        let (dir, archive, pin, cache) = setup("place");
        let verified = ensure(&pin, &cache, &LocalFile::new(&archive)).unwrap();
        let dest = dir.path().join("package").join("ffmpeg");
        place(&pin, &verified, &dest).unwrap();
        assert_eq!(
            names_in(&dest),
            ["ABOUT.txt", "LICENSE.txt", "ffmpeg.exe", "ffprobe.exe"]
        );
        assert_eq!(std::fs::read(dest.join("LICENSE.txt")).unwrap(), LICENCE);
        assert_eq!(std::fs::read(dest.join("ffmpeg.exe")).unwrap(), FFMPEG);
        let about = std::fs::read_to_string(dest.join("ABOUT.txt")).unwrap();
        for want in [
            "9.0.2",
            "full_build-www.gyan.dev",
            &pin.url,
            &pin.archive_sha256,
            &pin.source_note,
        ] {
            assert!(about.contains(want), "ABOUT.txt lacks {want:?}:\n{about}");
        }
        assert!(about.contains("runs this ffmpeg as a separate program"), "{about}");
        assert!(about.contains("GPL-3.0-or-later"), "{about}");
        // Placing again over the same files is fine, and nothing else lands next to the package's ffmpeg folder.
        place(&pin, &verified, &dest).unwrap();
        assert_eq!(names_in(dest.parent().unwrap()), ["ffmpeg"]);
    }

    #[test]
    fn place_refuses_a_changed_verified_file() {
        let (dir, archive, pin, cache) = setup("place-changed");
        let verified = ensure(&pin, &cache, &LocalFile::new(&archive)).unwrap();
        std::fs::write(verified.join("ffprobe.exe"), b"changed").unwrap();
        let dest = dir.path().join("ffmpeg");
        let err = place(&pin, &verified, &dest).unwrap_err();
        assert!(matches!(err, PackError::Checksum { .. }), "{err:?}");
        assert!(
            !names_in(&dest).iter().any(|n| n.contains("ffprobe")),
            "{:?}",
            names_in(&dest)
        );
    }

    /// The real download, run by name only (it needs the internet, and the owner's go-ahead: ruling M6-R1). It
    /// refuses while the archive is not pinned yet.
    #[test]
    #[ignore = "downloads the real ffmpeg archive (about 200 MB); run by name, only with the owner's go-ahead"]
    fn the_real_archive_fetches() {
        let dir = TempDir::new("real");
        let p = pin();
        let got = ensure(&p, &dir.path().join("cache"), &crate::fetch::Curl).unwrap();
        for f in &p.files {
            assert_eq!(sha256_file(&got.join(&f.name)).unwrap(), f.sha256, "{}", f.name);
        }
        println!(
            "the archive is {} bytes",
            std::fs::metadata(dir.path().join("cache").join(p.archive_name()))
                .unwrap()
                .len()
        );
    }
}
