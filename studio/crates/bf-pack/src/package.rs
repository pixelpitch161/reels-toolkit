//! The package folder and its `MANIFEST.txt` (plan M6, Task 11).
//!
//! A package is the program folder as it will be installed: `ballfight-studio.exe`, `ballfight.ico`, `ffmpeg\`
//! (the pinned ffmpeg and its ABOUT.txt), `guide\` (when the tree has one) and `MANIFEST.txt`, which lists every
//! other file with its size and SHA-256. The install copies exactly the files the manifest lists, and removal
//! deletes exactly those, so the manifest is the one record of what is ours.
//!
//! `MANIFEST.txt` reads:
//!
//! ```text
//! BALLFIGHT Studio package manifest 1
//! version 0.6.0
//! <sha256> <bytes> ballfight-studio.exe
//! <sha256> <bytes> ffmpeg/ffmpeg.exe
//! ```
//!
//! Paths use forward slashes, are relative, and are checked by `safe_relative` wherever a manifest is read.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::PackError;
use crate::ffmpeg::{self, FfmpegPin};
use crate::sha::{is_sha256, sha256_file};

/// The program's file name in the package and the install.
pub const EXE_NAME: &str = "ballfight-studio.exe";
/// The icon file the shortcuts show.
pub const ICON_NAME: &str = "ballfight.ico";
/// The manifest's file name.
pub const MANIFEST_NAME: &str = "MANIFEST.txt";
/// The manifest's first line: what marks a folder as one of ours.
pub const MANIFEST_HEADER: &str = "BALLFIGHT Studio package manifest 1";
/// The folder the app's own ffmpeg lives in. The same name as `bf_export::APP_FFMPEG_DIR` (bf-pack does not depend
/// on bf-export; `ffmpeg_folder_is_the_apps` in this file's tests reads bf-export's source to keep them equal).
pub const FFMPEG_DIR: &str = "ffmpeg";
/// The guide's folder in the package.
pub const GUIDE_DIR: &str = "guide";
/// The list a package being made holds while it is made: every name `package` may write in it, so a package that was
/// stopped part-way (a power cut) can be cleared by `clear_unfinished`. Removed once the manifest is written.
pub const PACKING_NAME: &str = "PACKING.txt";
/// Names at the top of the program folder that are never package files: the manifest, the install's own
/// `update.toml`, the update engine's `.next` and `.previous` folders (Task 12), the names a swap uses inside
/// `.previous` (its marker and the temp names of its two copies), and the list of a package being made.
pub const RESERVED: [&str; 8] = [
    MANIFEST_NAME,
    crate::config::CONFIG_NAME,
    ".next",
    ".previous",
    crate::install::SWAP_MARKER,
    crate::install::COPY_PARTS[0],
    crate::install::COPY_PARTS[1],
    PACKING_NAME,
];

/// The name a running exe is set aside under inside `.previous` when a swap is taken back while it runs (Windows
/// lets a running exe be renamed, not replaced): `ballfight-studio.exe.running-<n>`.
pub fn set_aside_exe_name(n: u32) -> String {
    format!("{EXE_NAME}.running-{n}")
}

/// Whether `name` is such a set-aside exe's name.
pub fn is_set_aside_exe(name: &str) -> bool {
    name.strip_prefix(EXE_NAME)
        .and_then(|rest| rest.strip_prefix(".running-"))
        .is_some_and(|n| !n.is_empty() && n.len() < 6 && n.bytes().all(|b| b.is_ascii_digit()))
}

/// What a package holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub version: String,
    /// `(path, bytes, sha256)`, sorted by path; paths relative with forward slashes.
    pub files: Vec<(String, u64, String)>,
}

impl Manifest {
    /// The manifest as written in `MANIFEST.txt` (Windows line ends).
    pub fn text(&self) -> String {
        let mut s = format!("{MANIFEST_HEADER}\r\nversion {}\r\n", self.version);
        for (path, bytes, sha) in &self.files {
            s.push_str(&format!("{sha} {bytes} {path}\r\n"));
        }
        s
    }

    /// The entry for `path` (forward slashes), compared without regard to case, as Windows compares names.
    pub fn find(&self, path: &str) -> Option<&(String, u64, String)> {
        self.files.iter().find(|(p, _, _)| p.eq_ignore_ascii_case(path))
    }
}

/// Windows' reserved device names, which no file may be called (with or without an extension), including the
/// console's own names and the superscript-digit ports Windows also treats as devices.
const DEVICES: [&str; 30] = [
    "CON",
    "PRN",
    "AUX",
    "NUL",
    "COM1",
    "COM2",
    "COM3",
    "COM4",
    "COM5",
    "COM6",
    "COM7",
    "COM8",
    "COM9",
    "LPT1",
    "LPT2",
    "LPT3",
    "LPT4",
    "LPT5",
    "LPT6",
    "LPT7",
    "LPT8",
    "LPT9",
    "CONIN$",
    "CONOUT$",
    "COM\u{b9}",
    "COM\u{b2}",
    "COM\u{b3}",
    "LPT\u{b9}",
    "LPT\u{b2}",
    "LPT\u{b3}",
];

/// A reading or writing failure as a plain sentence, "Could not {doing} {path}: {reason}.", without Windows' "(os
/// error N)" tail when there is a reason in words.
pub(crate) fn io_error(doing: &str, path: &Path, e: &std::io::Error) -> PackError {
    PackError::Io(format!("Could not {doing} {}: {}.", path.display(), plain_reason(e)))
}

/// An error's reason in words: Windows' own text without its "(os error N)" tail or a closing full stop.
pub(crate) fn plain_reason(e: &std::io::Error) -> String {
    let text = e.to_string();
    let words = match text.rfind(" (os error ") {
        Some(at) if text.ends_with(')') => text[..at].to_string(),
        _ => text,
    };
    let words = words.trim().trim_end_matches('.').to_string();
    if words.is_empty() { e.to_string() } else { words }
}

/// Whether `p` is a plain relative path inside a folder: forward slashes between non-empty names, no `.` or `..`,
/// no drive, no `\`, `:` or other character Windows refuses, no name ending in a dot or space, no device name, and
/// not one of the `RESERVED` names at the top.
pub fn safe_relative(p: &str) -> bool {
    if p.is_empty() || p.len() > 1000 {
        return false;
    }
    let mut parts = p.split('/').peekable();
    let first = parts.peek().copied().unwrap_or("");
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(first)) {
        return false;
    }
    p.split('/').all(|name| {
        let stem = name.split('.').next().unwrap_or("");
        !name.is_empty()
            && name != "."
            && name != ".."
            && !name.ends_with('.')
            && !name.ends_with(' ')
            && !name.starts_with(' ')
            && !name.chars().any(|c| c.is_control() || "\\:*?\"<>|".contains(c))
            && !DEVICES.iter().any(|d| d.eq_ignore_ascii_case(stem.trim_end()))
    })
}

/// `root` joined with a manifest path (forward slashes), one name at a time.
pub fn join_relative(root: &Path, p: &str) -> PathBuf {
    let mut out = root.to_path_buf();
    for name in p.split('/') {
        out.push(name);
    }
    out
}

/// Reads a manifest's text. Anything but our header, a version line and well-formed file lines with safe paths is
/// refused in words.
pub fn parse_manifest(text: &str) -> Result<Manifest, PackError> {
    let not_ours =
        |why: &str| PackError::Refused(format!("The file is not a manifest BALLFIGHT Studio wrote ({why})."));
    let mut lines = text.lines();
    if lines.next() != Some(MANIFEST_HEADER) {
        return Err(not_ours("its first line is different"));
    }
    let version = lines
        .next()
        .and_then(|l| l.strip_prefix("version "))
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || ".-".contains(c)))
        .ok_or_else(|| not_ours("its version line is missing"))?
        .to_string();
    let mut files: Vec<(String, u64, String)> = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, ' ');
        let (Some(sha), Some(bytes), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            return Err(not_ours("a line has too few parts"));
        };
        let bytes: u64 = match bytes.parse() {
            Ok(n) if bytes.bytes().all(|b| b.is_ascii_digit()) => n,
            _ => return Err(not_ours("a size is not a number")),
        };
        if !is_sha256(sha) {
            return Err(not_ours("a checksum is not 64 hex digits"));
        }
        if !safe_relative(path) {
            return Err(not_ours("a file name is not a plain name inside the folder"));
        }
        // Each path once (as Windows compares names), and no file that is also another file's folder.
        let lower = path.to_lowercase();
        for (other, _, _) in &files {
            let o = other.to_lowercase();
            if o == lower || lower.starts_with(&format!("{o}/")) || o.starts_with(&format!("{lower}/")) {
                return Err(not_ours("a file is listed twice"));
            }
        }
        files.push((path.to_string(), bytes, sha.to_string()));
    }
    files.sort();
    Ok(Manifest { version, files })
}

/// Reads `MANIFEST.txt` at `path`. A missing file or one that is not ours is an error in words.
pub fn read_manifest(path: &Path) -> Result<Manifest, PackError> {
    if is_link(path) {
        return Err(PackError::Refused(format!(
            "{} is a link, so it is not read.",
            path.display()
        )));
    }
    let text = fs::read_to_string(path).map_err(|e| io_error("read", path, &e))?;
    parse_manifest(&text).map_err(|e| PackError::Refused(format!("{}: {e}", path.display())))
}

/// The files and folders inside `dir`, relative with forward slashes, folders before what they hold; links are
/// listed (as `Kind::Link`) and never entered.
pub(crate) fn walk(dir: &Path) -> Result<Vec<(String, Kind)>, PackError> {
    let mut out = Vec::new();
    walk_into(dir, "", &mut out)?;
    Ok(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    File,
    Dir,
    Link,
}

fn walk_into(dir: &Path, base: &str, out: &mut Vec<(String, Kind)>) -> Result<(), PackError> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| io_error("read the folder", dir, &e))?
        .collect::<Result<_, _>>()
        .map_err(|e| io_error("read the folder", dir, &e))?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = if base.is_empty() {
            name
        } else {
            format!("{base}/{name}")
        };
        let path = e.path();
        if is_link(&path) {
            out.push((rel, Kind::Link));
        } else if path.is_dir() {
            out.push((rel.clone(), Kind::Dir));
            walk_into(&path, &rel, out)?;
        } else {
            out.push((rel, Kind::File));
        }
    }
    Ok(())
}

/// Whether the item at `path` is a link of any kind (a symbolic link, a junction or another reparse point), without
/// following it. A missing item is not a link.
pub fn is_link(path: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || meta.file_type().is_symlink()
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

/// Copies `from` to `to` (which must not exist yet) and gives the SHA-256 and size of what was written.
pub(crate) fn copy_new(from: &Path, to: &Path) -> Result<(String, u64), PackError> {
    let mut src = fs::File::open(from).map_err(|e| io_error("open", from, &e))?;
    let mut dst = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)
        .map_err(|e| io_error("make", to, &e))?;
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut bytes = 0u64;
    loop {
        let n = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(io_error("read", from, &e)),
        };
        sha2::Digest::update(&mut hasher, &buf[..n]);
        dst.write_all(&buf[..n]).map_err(|e| io_error("write", to, &e))?;
        bytes += n as u64;
    }
    dst.sync_all().map_err(|e| io_error("write", to, &e))?;
    Ok((crate::sha::hex(&sha2::Digest::finalize(hasher)), bytes))
}

/// Makes the package for `exe` in `out`, which must be missing or empty and must not be inside a cargo target
/// folder (an `ffmpeg\` there would be used by a developer's build). `tree` is a checkout of the repository (the
/// folder holding `studio\`): the icon, the guide, the ffmpeg pin and the version come from it. `ffmpeg` is the
/// folder of checked ffmpeg files that `ffmpeg::ensure` gave. If anything fails, the files this call wrote are
/// removed again, one by one.
pub fn package(exe: &Path, tree: &Path, ffmpeg: &Path, out: &Path) -> Result<Manifest, PackError> {
    if inside_cargo_target(out) {
        return Err(PackError::Refused(format!(
            "{} is inside a cargo build folder, where an ffmpeg folder would be used by builds made there, so no \
             package was made. Choose a folder elsewhere.",
            out.display()
        )));
    }
    if is_link(out) {
        return Err(PackError::Refused(format!(
            "{} is a link to another place, so no package was made there.",
            out.display()
        )));
    }
    let made_out = !out.exists();
    if !made_out {
        let empty = out.is_dir()
            && fs::read_dir(out)
                .map_err(|e| io_error("read the folder", out, &e))?
                .next()
                .is_none();
        if !empty {
            return Err(PackError::Refused(format!(
                "{} is not empty, so no package was made there. Choose a new folder.",
                out.display()
            )));
        }
    }
    let studio = tree.join("studio");
    let version = workspace_version(&studio.join("Cargo.toml"))?;
    let pin_path = studio.join("packaging").join("ffmpeg.toml");
    let pin_text = fs::read_to_string(&pin_path).map_err(|e| io_error("read", &pin_path, &e))?;
    let pin = ffmpeg::parse_pin(&pin_text)?;

    let mut w = Written::default();
    let made = fill(exe, &studio, ffmpeg, out, made_out, &pin, &version, &mut w);
    match &made {
        Err(PackError::Io(t)) if t == STOPPED_FOR_A_TEST => {}
        Err(_) => w.undo(),
        Ok(_) => {}
    }
    made
}

/// The words of a package stopped on purpose by a test, as a power cut would stop it: nothing taken back.
const STOPPED_FOR_A_TEST: &str = "The package was stopped part-way for a test.";

#[cfg(test)]
thread_local! {
    /// For tests: stop `fill` after this many of its steps, leaving everything as it is then.
    static STOP_AFTER: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    static STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// One step of `fill` is done; a test may stop it here.
fn step_done() -> Result<(), PackError> {
    #[cfg(test)]
    {
        let n = STEPS.with(|s| {
            s.set(s.get() + 1);
            s.get()
        });
        if STOP_AFTER.with(|s| s.get()).is_some_and(|stop| n >= stop) {
            return Err(PackError::Io(STOPPED_FOR_A_TEST.to_string()));
        }
    }
    Ok(())
}

/// The files and folders a package call wrote, so that a failure can take back exactly those.
#[derive(Default)]
struct Written {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

impl Written {
    fn undo(&self) {
        for f in self.files.iter().rev() {
            if !is_link(f) && f.is_file() {
                let _ = fs::remove_file(f);
            }
        }
        for d in self.dirs.iter().rev() {
            // Only an empty folder goes; never one with anything in it.
            let _ = fs::remove_dir(d);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill(
    exe: &Path,
    studio: &Path,
    ffmpeg_dir: &Path,
    out: &Path,
    made_out: bool,
    pin: &FfmpegPin,
    version: &str,
    w: &mut Written,
) -> Result<Manifest, PackError> {
    if made_out {
        fs::create_dir_all(out).map_err(|e| io_error("make the folder", out, &e))?;
        w.dirs.push(out.to_path_buf());
    }
    let guide = studio.join("packaging").join(GUIDE_DIR);
    let guide_entries = if guide.is_dir() && !is_link(&guide) {
        let entries = walk(&guide)?;
        for (rel, kind) in &entries {
            let whole = format!("{GUIDE_DIR}/{rel}");
            if *kind == Kind::Link || !safe_relative(&whole) {
                return Err(PackError::Refused(format!(
                    "The guide's {rel} is a link or has a name Windows cannot keep, so no package was made."
                )));
            }
        }
        Some(entries)
    } else {
        None
    };

    // First the list of every name this call may write, made safe on disk before anything else is written, so a
    // package stopped part-way is cleared by that list (`clear_unfinished`).
    let mut list = vec![
        format!("F {EXE_NAME}"),
        format!("F {ICON_NAME}"),
        format!("D {FFMPEG_DIR}"),
    ];
    for name in pin.files.iter().map(|f| f.name.as_str()).chain(["ABOUT.txt"]) {
        list.push(format!("F {FFMPEG_DIR}/{name}"));
        list.push(format!("F {FFMPEG_DIR}/.{name}.part"));
    }
    if let Some(entries) = &guide_entries {
        list.push(format!("D {GUIDE_DIR}"));
        for (rel, kind) in entries {
            let tag = if *kind == Kind::Dir { 'D' } else { 'F' };
            list.push(format!("{tag} {GUIDE_DIR}/{rel}"));
        }
    }
    list.push(format!("F {MANIFEST_NAME}"));
    let journal = out.join(PACKING_NAME);
    w.files.push(journal.clone());
    let mut text = format!("{PACKING_HEADER}\r\n");
    for line in &list {
        text.push_str(line);
        text.push_str("\r\n");
    }
    write_new_synced(&journal, text.as_bytes())?;
    step_done()?;

    let to = out.join(EXE_NAME);
    w.files.push(to.clone());
    copy_new(exe, &to)?;
    step_done()?;
    let icon = studio
        .join("crates")
        .join("bf-app")
        .join("assets")
        .join("icon")
        .join(ICON_NAME);
    let to = out.join(ICON_NAME);
    w.files.push(to.clone());
    copy_new(&icon, &to)?;
    step_done()?;

    // The folder is new, so every name `place` may write in it is ours to take back.
    let ff = out.join(FFMPEG_DIR);
    w.dirs.push(ff.clone());
    w.files.extend(pin.files.iter().map(|f| ff.join(&f.name)));
    w.files.push(ff.join("ABOUT.txt"));
    ffmpeg::place(pin, ffmpeg_dir, &ff)?;
    step_done()?;

    if let Some(entries) = guide_entries {
        let to = out.join(GUIDE_DIR);
        fs::create_dir(&to).map_err(|e| io_error("make the folder", &to, &e))?;
        w.dirs.push(to.clone());
        for (rel, kind) in entries {
            let whole = format!("{GUIDE_DIR}/{rel}");
            let dest = join_relative(out, &whole);
            if kind == Kind::Dir {
                fs::create_dir(&dest).map_err(|e| io_error("make the folder", &dest, &e))?;
                w.dirs.push(dest);
            } else {
                w.files.push(dest.clone());
                copy_new(&join_relative(&guide, &rel), &dest)?;
            }
            step_done()?;
        }
    }

    let mut files = Vec::new();
    for (rel, kind) in walk(out)? {
        match kind {
            Kind::Dir => {}
            Kind::File if rel == PACKING_NAME => {}
            Kind::Link => {
                return Err(PackError::Refused(format!(
                    "{rel} in the package is a link, so no package was made."
                )));
            }
            Kind::File => {
                let f = join_relative(out, &rel);
                let sha = sha256_file(&f).map_err(|e| io_error("read", &f, &e))?;
                let bytes = fs::metadata(&f).map_err(|e| io_error("read", &f, &e))?.len();
                files.push((rel, bytes, sha));
            }
        }
    }
    files.sort();
    let manifest = Manifest {
        version: version.to_string(),
        files,
    };
    // Read back through the parser, so a package never holds a manifest that cannot be read.
    parse_manifest(&manifest.text())?;
    let to = out.join(MANIFEST_NAME);
    w.files.push(to.clone());
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&to)
        .map_err(|e| io_error("make", &to, &e))?;
    f.write_all(manifest.text().as_bytes())
        .and_then(|()| f.sync_all())
        .map_err(|e| io_error("write", &to, &e))?;
    drop(f);
    step_done()?;
    // The package is whole: its list goes. (Stopped just before this, the folder holds a finished package and the
    // list; `clear_unfinished` still clears it by the list, which names the manifest too.)
    fs::remove_file(&journal).map_err(|e| io_error("remove", &journal, &e))?;
    Ok(manifest)
}

/// The first line of a package's `PACKING.txt`.
const PACKING_HEADER: &str = "BALLFIGHT Studio package being made 1";

/// Writes a new file and makes sure it is on the disk before going on.
fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), PackError> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| io_error("make", path, &e))?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| io_error("write", path, &e))
}

/// Clears a package folder that `package` began and did not finish (a power cut, a closed window), by its own list
/// `PACKING.txt`: each file the list names (whatever its bytes, since a copy may have stopped part-way), then the
/// folders it names when they are empty, the list, and `out` itself. Gives `true` when `out` is gone. Anything in
/// `out` that the list does not name, or any link, stops it in words with nothing removed. Without a list: a missing
/// folder, or a finished package (its manifest and no list), is left as it is (`false`); a folder that holds only
/// empty folders is removed; anything else is an error in words.
pub fn clear_unfinished(out: &Path) -> Result<bool, PackError> {
    if fs::symlink_metadata(out).is_err() {
        return Ok(false);
    }
    let refuse = || {
        PackError::Refused(format!(
            "{} holds files that BALLFIGHT Studio cannot tell are its own, so it was left as it is. Move them \
             elsewhere, then try again.",
            out.display()
        ))
    };
    if is_link(out) || !out.is_dir() {
        return Err(refuse());
    }
    let entries = walk(out)?;
    let journal = out.join(PACKING_NAME);
    let remove_dirs = |dirs: &[PathBuf]| -> Result<(), PackError> {
        for d in dirs.iter().rev() {
            fs::remove_dir(d).map_err(|e| io_error("remove the empty folder", d, &e))?;
        }
        fs::remove_dir(out).map_err(|e| io_error("remove the empty folder", out, &e))
    };
    if is_link(&journal) || !journal.is_file() {
        if entries.iter().all(|(_, k)| *k == Kind::Dir) {
            let dirs: Vec<PathBuf> = entries.iter().map(|(rel, _)| join_relative(out, rel)).collect();
            remove_dirs(&dirs)?;
            return Ok(true);
        }
        if out.join(MANIFEST_NAME).is_file() {
            return Ok(false);
        }
        return Err(refuse());
    }
    let text = fs::read_to_string(&journal).map_err(|e| io_error("read", &journal, &e))?;
    let mut lines = text.lines();
    if lines.next() != Some(PACKING_HEADER) {
        return Err(refuse());
    }
    let (mut files, mut dirs) = (Vec::new(), Vec::new());
    // A line cut short by a power cut names nothing; nothing else was written before the list was whole.
    for line in lines {
        let Some((tag, rel)) = line.split_once(' ') else {
            continue;
        };
        if !(safe_relative(rel) || rel == MANIFEST_NAME) {
            continue;
        }
        match tag {
            "F" => files.push(rel.to_string()),
            "D" => dirs.push(rel.to_string()),
            _ => {}
        }
    }
    let mut found_dirs = Vec::new();
    for (rel, kind) in &entries {
        let listed = match kind {
            Kind::Link => false,
            Kind::File => rel == PACKING_NAME || files.iter().any(|f| f == rel),
            Kind::Dir => dirs.iter().any(|d| d == rel),
        };
        if !listed {
            return Err(refuse());
        }
        if *kind == Kind::Dir {
            found_dirs.push(join_relative(out, rel));
        }
    }
    for (rel, kind) in &entries {
        if *kind == Kind::File && rel != PACKING_NAME {
            let f = join_relative(out, rel);
            fs::remove_file(&f).map_err(|e| io_error("remove", &f, &e))?;
        }
    }
    fs::remove_file(&journal).map_err(|e| io_error("remove", &journal, &e))?;
    remove_dirs(&found_dirs)?;
    Ok(true)
}

/// `[workspace.package] version` of a checkout's `studio\Cargo.toml`, which must be a version as the Studio writes
/// them.
fn workspace_version(cargo_toml: &Path) -> Result<String, PackError> {
    let text = fs::read_to_string(cargo_toml).map_err(|e| io_error("read", cargo_toml, &e))?;
    let table: toml::Table = toml::from_str(&text)
        .map_err(|_| PackError::Refused(format!("{} could not be read.", cargo_toml.display())))?;
    table
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .filter(|v| crate::version::parse_version(v).is_some())
        .map(str::to_string)
        .ok_or_else(|| PackError::Refused(format!("{} names no version of the Studio.", cargo_toml.display())))
}

/// Whether `dir` is in a cargo target folder: one of its folders that exists holds cargo's `CACHEDIR.TAG` or
/// `.rustc_info.json`, or its path has `target\release` or `target\debug` in it.
fn inside_cargo_target(dir: &Path) -> bool {
    let marked = dir
        .ancestors()
        .any(|a| a.join("CACHEDIR.TAG").is_file() || a.join(".rustc_info.json").is_file());
    let names: Vec<String> = dir
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    let named = names
        .windows(2)
        .any(|w| w[0] == "target" && (w[1] == "release" || w[1] == "debug"));
    marked || named
}

/// Checks a package folder: our `MANIFEST.txt`, every file it lists there with its size and SHA-256, and nothing
/// else in the folder (no other file, no link). Gives the manifest.
pub fn check_package(dir: &Path) -> Result<Manifest, PackError> {
    check_package_with(dir, false)
}

/// `check_package` for a version kept in `.previous` (or brought back from there by "Go back"): it may also hold the
/// copy of `update.toml` an install or update kept with it, when that reads as one of ours.
pub fn check_kept_package(dir: &Path) -> Result<Manifest, PackError> {
    check_package_with(dir, true)
}

fn check_package_with(dir: &Path, kept_config: bool) -> Result<Manifest, PackError> {
    if is_link(dir) || !dir.is_dir() {
        return Err(PackError::Refused(format!(
            "The package {} is not a folder, so nothing was installed.",
            dir.display()
        )));
    }
    let manifest = read_manifest(&dir.join(MANIFEST_NAME))?;
    for (rel, kind) in walk(dir)? {
        match kind {
            Kind::Dir => {}
            Kind::Link => {
                return Err(PackError::Refused(format!(
                    "{rel} in the package {} is a link, so nothing was installed.",
                    dir.display()
                )));
            }
            Kind::File if rel == MANIFEST_NAME => {}
            Kind::File if kept_config && is_set_aside_exe(&rel) => {}
            Kind::File
                if kept_config
                    && rel == crate::config::CONFIG_NAME
                    && !is_link(&dir.join(&rel))
                    && crate::config::read_config(&dir.join(&rel)).is_ok() => {}
            Kind::File => {
                if manifest.files.iter().all(|(p, _, _)| p != &rel) {
                    return Err(PackError::Refused(format!(
                        "{rel} is in the package {} but not in its manifest, so nothing was installed.",
                        dir.display()
                    )));
                }
            }
        }
    }
    for (rel, bytes, sha) in &manifest.files {
        let f = join_relative(dir, rel);
        if !f.is_file() {
            return Err(PackError::Refused(format!(
                "{rel} is missing from the package {}, so nothing was installed.",
                dir.display()
            )));
        }
        let got = sha256_file(&f).map_err(|e| io_error("read", &f, &e))?;
        let len = fs::metadata(&f).map_err(|e| io_error("read", &f, &e))?.len();
        if &got != sha || len != *bytes {
            return Err(PackError::Checksum {
                what: format!("{rel} in the package {}", dir.display()),
                want: sha.clone(),
                got,
            });
        }
    }
    Ok(manifest)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::testdir::TempDir;

    pub(crate) const EXE: &[u8] = b"pretend ballfight-studio.exe";
    pub(crate) const ICO: &[u8] = b"pretend icon";
    const FFMPEG: &[u8] = b"pretend ffmpeg";
    const FFPROBE: &[u8] = b"pretend ffprobe";
    const LICENCE: &[u8] = b"GNU GENERAL PUBLIC LICENSE";

    fn hex(bytes: &[u8]) -> String {
        crate::sha::sha256_reader(bytes).unwrap()
    }

    /// A fake checkout in `root\tree` (version, icon, a two-page guide, a pin matching the fake ffmpeg files), the
    /// fake exe, and the checked ffmpeg folder. Gives `(exe, tree, ffmpeg)`.
    pub(crate) fn fake_inputs(root: &Path, version: &str) -> (PathBuf, PathBuf, PathBuf) {
        let tree = root.join("tree");
        let studio = tree.join("studio");
        fs::create_dir_all(studio.join("crates/bf-app/assets/icon")).unwrap();
        fs::create_dir_all(studio.join("packaging/guide/pictures")).unwrap();
        fs::write(
            studio.join("Cargo.toml"),
            format!("[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"{version}\"\n"),
        )
        .unwrap();
        fs::write(studio.join("crates/bf-app/assets/icon/ballfight.ico"), ICO).unwrap();
        fs::write(studio.join("packaging/guide/index.html"), b"<p>The guide.</p>").unwrap();
        fs::write(studio.join("packaging/guide/pictures/start.png"), b"pretend picture").unwrap();
        let pin = format!(
            "version = \"9.0.2\"\nbuild = \"full_build-www.gyan.dev\"\n\
             url = \"https://example.org/ffmpeg-9.0.2-full_build.zip\"\n\
             archive_sha256 = \"{}\"\narchive_bytes = 100\nlicence = \"GPL-3.0-or-later\"\n\
             source_note = \"The source is at https://example.org/source.\"\n\n\
             [[files]]\nname = \"ffmpeg.exe\"\nin_archive = \"x/bin/ffmpeg.exe\"\nsha256 = \"{}\"\n\n\
             [[files]]\nname = \"ffprobe.exe\"\nin_archive = \"x/bin/ffprobe.exe\"\nsha256 = \"{}\"\n\n\
             [[files]]\nname = \"LICENSE.txt\"\nin_archive = \"x/LICENSE\"\nsha256 = \"{}\"\n",
            hex(b"pretend archive"),
            hex(FFMPEG),
            hex(FFPROBE),
            hex(LICENCE)
        );
        fs::write(studio.join("packaging/ffmpeg.toml"), pin).unwrap();
        let checked = root.join("checked");
        fs::create_dir_all(&checked).unwrap();
        fs::write(checked.join("ffmpeg.exe"), FFMPEG).unwrap();
        fs::write(checked.join("ffprobe.exe"), FFPROBE).unwrap();
        fs::write(checked.join("LICENSE.txt"), LICENCE).unwrap();
        let exe = root.join("built").join(EXE_NAME);
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, EXE).unwrap();
        (exe, tree, checked)
    }

    /// A package of the fake inputs at `root\<name>`.
    pub(crate) fn fake_package(root: &Path, name: &str, version: &str) -> PathBuf {
        let inputs = root.join(format!("{name}-inputs"));
        let (exe, tree, checked) = fake_inputs(&inputs, version);
        let out = root.join(name);
        package(&exe, &tree, &checked, &out).unwrap();
        out
    }

    fn all_files(dir: &Path, base: &str, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().into_owned();
            let rel = if base.is_empty() {
                name
            } else {
                format!("{base}/{name}")
            };
            if e.file_type().unwrap().is_dir() {
                all_files(&e.path(), &rel, out);
            } else {
                out.push(rel);
            }
        }
    }

    #[test]
    fn the_package_holds_every_file() {
        let dir = TempDir::new("package-every");
        let (exe, tree, checked) = fake_inputs(dir.path(), "0.6.0");
        let out = dir.path().join("package");
        let m = package(&exe, &tree, &checked, &out).unwrap();
        assert_eq!(m.version, "0.6.0");
        let listed: Vec<&str> = m.files.iter().map(|(p, _, _)| p.as_str()).collect();
        assert_eq!(
            listed,
            [
                "ballfight-studio.exe",
                "ballfight.ico",
                "ffmpeg/ABOUT.txt",
                "ffmpeg/LICENSE.txt",
                "ffmpeg/ffmpeg.exe",
                "ffmpeg/ffprobe.exe",
                "guide/index.html",
                "guide/pictures/start.png",
            ]
        );
        // Every file listed is there with that size and hash, and nothing else is in the folder.
        let mut on_disk = Vec::new();
        all_files(&out, "", &mut on_disk);
        on_disk.sort();
        let mut want: Vec<String> = listed.iter().map(|s| s.to_string()).collect();
        want.push(MANIFEST_NAME.to_string());
        want.sort();
        assert_eq!(on_disk, want);
        for (p, bytes, sha) in &m.files {
            let f = join_relative(&out, p);
            assert_eq!(fs::metadata(&f).unwrap().len(), *bytes, "{p}");
            assert_eq!(&sha256_file(&f).unwrap(), sha, "{p}");
        }
        assert_eq!(m.find("ballfight-studio.exe").unwrap().2, hex(EXE));
        // MANIFEST.txt says the same, and reads back.
        let text = fs::read_to_string(out.join(MANIFEST_NAME)).unwrap();
        assert!(
            text.starts_with("BALLFIGHT Studio package manifest 1\r\nversion 0.6.0\r\n"),
            "{text}"
        );
        assert!(text.contains(&format!("{} {} ballfight-studio.exe\r\n", hex(EXE), EXE.len())));
        assert_eq!(read_manifest(&out.join(MANIFEST_NAME)).unwrap(), m);
        assert_eq!(check_package(&out).unwrap(), m);
    }

    #[test]
    fn a_package_stopped_anywhere_is_cleared_by_its_list() {
        let dir = TempDir::new("package-stopped");
        let (exe, tree, checked) = fake_inputs(dir.path(), "0.6.0");
        // Steps: the list, the exe, the icon, ffmpeg, two guide entries (a folder and two files), the manifest.
        let mut stopped_ones = 0;
        for stop in 1..=12 {
            let out = dir.path().join(format!("out-{stop}"));
            STEPS.with(|s| s.set(0));
            STOP_AFTER.with(|s| s.set(Some(stop)));
            let made = package(&exe, &tree, &checked, &out);
            STOP_AFTER.with(|s| s.set(None));
            match made {
                Err(PackError::Io(t)) if t == STOPPED_FOR_A_TEST => {
                    stopped_ones += 1;
                    assert!(out.join(PACKING_NAME).is_file(), "step {stop}: the list is there");
                    assert!(
                        check_package(&out).is_err(),
                        "step {stop}: a stopped package never passes"
                    );
                    assert_eq!(clear_unfinished(&out), Ok(true), "step {stop}");
                    assert!(!out.exists(), "step {stop}: nothing is left");
                }
                Ok(_) => {
                    assert!(!out.join(PACKING_NAME).exists());
                    check_package(&out).unwrap();
                    // A finished package is not unfinished: it stays.
                    assert_eq!(clear_unfinished(&out), Ok(false));
                    assert!(out.join(MANIFEST_NAME).is_file());
                }
                Err(e) => panic!("step {stop}: {e}"),
            }
        }
        assert!(stopped_ones >= 6, "{stopped_ones}");

        // Something the list does not name stops the clearing, with nothing removed.
        let out = dir.path().join("out-foreign");
        STEPS.with(|s| s.set(0));
        STOP_AFTER.with(|s| s.set(Some(3)));
        let _ = package(&exe, &tree, &checked, &out);
        STOP_AFTER.with(|s| s.set(None));
        fs::write(out.join("notes.txt"), b"a person's file").unwrap();
        let before = {
            let mut v = Vec::new();
            all_files(&out, "", &mut v);
            v.sort();
            v
        };
        let err = clear_unfinished(&out).unwrap_err().to_string();
        assert!(err.contains("cannot tell are its own"), "{err}");
        let mut after = Vec::new();
        all_files(&out, "", &mut after);
        after.sort();
        assert_eq!(before, after);

        // A list cut short with nothing else, and a folder of empty folders, go; a missing folder is nothing to do.
        let cut = dir.path().join("cut");
        fs::create_dir(&cut).unwrap();
        fs::write(
            cut.join(PACKING_NAME),
            "BALLFIGHT Studio package being made 1\r\nF ballfi",
        )
        .unwrap();
        assert_eq!(clear_unfinished(&cut), Ok(true));
        let empty = dir.path().join("empty");
        fs::create_dir_all(empty.join("ffmpeg")).unwrap();
        assert_eq!(clear_unfinished(&empty), Ok(true));
        assert!(!empty.exists());
        assert_eq!(clear_unfinished(&dir.path().join("missing")), Ok(false));
        // Without a list, files that are not a finished package are never taken.
        let odd = dir.path().join("odd");
        fs::create_dir(&odd).unwrap();
        fs::write(odd.join(EXE_NAME), EXE).unwrap();
        assert!(clear_unfinished(&odd).is_err());
        assert!(odd.join(EXE_NAME).is_file());
    }

    #[test]
    fn a_package_without_a_guide_has_none() {
        let dir = TempDir::new("package-noguide");
        let (exe, tree, checked) = fake_inputs(dir.path(), "0.6.0");
        fs::remove_file(tree.join("studio/packaging/guide/pictures/start.png")).unwrap();
        fs::remove_dir(tree.join("studio/packaging/guide/pictures")).unwrap();
        fs::remove_file(tree.join("studio/packaging/guide/index.html")).unwrap();
        fs::remove_dir(tree.join("studio/packaging/guide")).unwrap();
        let m = package(&exe, &tree, &checked, &dir.path().join("p")).unwrap();
        assert!(m.files.iter().all(|(p, _, _)| !p.starts_with("guide/")));
    }

    #[test]
    fn a_package_is_never_made_in_a_cargo_target_folder() {
        let dir = TempDir::new("package-target");
        let (exe, tree, checked) = fake_inputs(dir.path(), "0.6.0");
        // A folder cargo made (it writes CACHEDIR.TAG at the top of its target folder).
        let target = dir.path().join("build-here");
        fs::create_dir_all(target.join("release")).unwrap();
        fs::write(
            target.join("CACHEDIR.TAG"),
            b"Signature: 8a477f597d28d172789f06886806bc55",
        )
        .unwrap();
        for out in [
            target.join("release"),
            target.join("release").join("pkg"),
            dir.path().join("target/release/p"),
        ] {
            let err = package(&exe, &tree, &checked, &out).unwrap_err().to_string();
            assert!(err.contains("build folder"), "{err}");
            assert!(!out.join(EXE_NAME).exists());
        }
    }

    #[test]
    fn a_package_needs_an_empty_folder_and_cleans_up_after_itself() {
        let dir = TempDir::new("package-empty");
        let (exe, tree, checked) = fake_inputs(dir.path(), "0.6.0");
        let out = dir.path().join("p");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("mine.txt"), b"a person's file").unwrap();
        let err = package(&exe, &tree, &checked, &out).unwrap_err().to_string();
        assert!(err.contains("not empty"), "{err}");
        assert_eq!(fs::read(out.join("mine.txt")).unwrap(), b"a person's file");

        // A changed ffmpeg file stops the package part-way: what it wrote is removed again, the folder too.
        fs::write(checked.join("ffprobe.exe"), b"changed").unwrap();
        let out2 = dir.path().join("p2");
        assert!(matches!(
            package(&exe, &tree, &checked, &out2),
            Err(PackError::Checksum { .. })
        ));
        assert!(!out2.exists(), "the half-made package is gone");
    }

    #[test]
    fn a_changed_or_extra_file_fails_the_check() {
        let dir = TempDir::new("package-check");
        let out = fake_package(dir.path(), "p", "0.6.0");
        check_package(&out).unwrap();
        fs::write(out.join("extra.txt"), b"x").unwrap();
        let err = check_package(&out).unwrap_err().to_string();
        assert!(err.contains("extra.txt"), "{err}");
        fs::remove_file(out.join("extra.txt")).unwrap();
        fs::write(out.join("ballfight.ico"), b"changed").unwrap();
        assert!(matches!(check_package(&out), Err(PackError::Checksum { .. })));
        fs::remove_file(out.join("ballfight.ico")).unwrap();
        let err = check_package(&out).unwrap_err().to_string();
        assert!(err.contains("ballfight.ico"), "{err}");
    }

    #[test]
    fn only_safe_paths_are_read() {
        for fine in [
            "ballfight-studio.exe",
            "ffmpeg/ffmpeg.exe",
            "guide/pictures/a b.png",
            ".hidden",
        ] {
            assert!(safe_relative(fine), "{fine}");
        }
        for bad in [
            "",
            "/abs",
            "C:/x",
            "a//b",
            "../x",
            "a/../b",
            "./a",
            "a\\b",
            "a:b",
            "a/",
            "dot.",
            "space ",
            "nul",
            "CON.txt",
            "com1",
            "MANIFEST.txt",
            "update.toml",
            ".previous/x",
            ".NEXT/x",
            "a\tb",
            "conin$",
            "CONOUT$.log",
            "dir/CONIN$",
            "COM\u{b9}",
            "com\u{b2}.txt",
            "LPT\u{b3}",
        ] {
            assert!(!safe_relative(bad), "{bad:?}");
        }
        for fine in ["COM10", "console.txt", "lpt", "COM\u{b9}x"] {
            assert!(safe_relative(fine), "{fine:?}");
        }
        let sha = hex(b"x");
        let good = format!("{MANIFEST_HEADER}\nversion 0.6.0\n{sha} 1 a.txt\n");
        assert_eq!(
            parse_manifest(&good).unwrap().files,
            vec![("a.txt".to_string(), 1, sha.clone())]
        );
        for bad in [
            format!("Some other manifest\nversion 0.6.0\n{sha} 1 a.txt\n"),
            format!("{MANIFEST_HEADER}\n{sha} 1 a.txt\n"),
            format!("{MANIFEST_HEADER}\nversion 0.6.0\n{sha} 1 ../a.txt\n"),
            format!("{MANIFEST_HEADER}\nversion 0.6.0\n{sha} x a.txt\n"),
            format!("{MANIFEST_HEADER}\nversion 0.6.0\nabc 1 a.txt\n"),
            format!("{MANIFEST_HEADER}\nversion 0.6.0\n{sha} 1 a.txt\n{sha} 1 A.TXT\n"),
            format!("{MANIFEST_HEADER}\nversion 0.6.0\n{sha} 1 a.txt\n{sha} 1 a.txt/b\n"),
        ] {
            assert!(parse_manifest(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn errors_read_without_the_os_error_number() {
        let e = std::io::Error::from_raw_os_error(5);
        let said = io_error("remove", Path::new(r"C:\x\a.txt"), &e).to_string();
        assert!(!said.contains("os error"), "{said}");
        assert!(said.starts_with(r"Could not remove C:\x\a.txt: ") && said.ends_with('.') && !said.ends_with(".."));
        assert_eq!(plain_reason(&std::io::Error::other("refused")), "refused");
    }

    #[test]
    fn ffmpeg_folder_is_the_apps() {
        let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../bf-export/src/encode.rs"))
            .expect("bf-export's encode.rs");
        assert!(
            src.contains(&format!("pub const APP_FFMPEG_DIR: &str = \"{FFMPEG_DIR}\";")),
            "bf-export's APP_FFMPEG_DIR changed"
        );
    }
}
