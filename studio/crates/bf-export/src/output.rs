//! The output folder (§9): `<output folder>\<a>_vs_<b>\` (or `<a>_vs_<b>_seed<N>\` when that name is taken)
//! holding `<name>.mp4`, `cover.png`, `caption.txt` and the fight file `<name>.toml`. A clip (ruling R41) is
//! `<a>_vs_<b>_frames<A>-<B>\` (or `<a>_vs_<b>_seed<N>_frames<A>-<B>\`) holding only `<name>.mp4`, so the
//! Library, which lists folders with a `cover.png`, never takes it for a video.
//!
//! A video is made in a hidden temporary folder beside its target, `.tmp-<name>-<pid>` (the Library also skips
//! names that start with "."), and moved into place only when every check has passed. Nothing that exists is
//! ever overwritten or deleted: the target name is decided before any work and must be free, and the move is
//! one that cannot replace anything (`MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`): if the name was taken
//! meanwhile, the finished folder stays where it is and the error names it. A failed or cancelled job removes
//! only the files it wrote into its own temporary folder (by their exact paths) and then that folder, which must
//! be empty by then (never a recursive delete, never through a link).

use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bf_core::fightfile::FightFile;
use bf_picture::Rgb8;

use crate::ExportError;

/// `flute_vs_trombone` for `["FLUTE", "TROMBONE"]`: each name in lower case with its spaces as `_` (the app's
/// `wording::folder_stem`).
pub fn folder_stem(names: [&str; 2]) -> String {
    let stem = |name: &str| name.to_lowercase().split_whitespace().collect::<Vec<_>>().join("_");
    format!("{}_vs_{}", stem(names[0]), stem(names[1]))
}

/// Whether anything (a file, a folder, even a broken link) is at `path`; a path that cannot be checked counts as
/// taken.
pub(crate) fn taken(path: &Path) -> bool {
    fs::symlink_metadata(path).map_or_else(|e| e.kind() != io::ErrorKind::NotFound, |_| true)
}

/// The video's folder under `root`: `<root>/flute_vs_trombone`, or `<root>/flute_vs_trombone_seed<N>` when the
/// first is taken. (When that one is taken too the job refuses; see [`crate::run_job`].)
pub fn folder_name(names: [&str; 2], seed: u64, root: &Path) -> PathBuf {
    let stem = folder_stem(names);
    let first = root.join(&stem);
    if taken(&first) {
        root.join(format!("{stem}_seed{seed}"))
    } else {
        first
    }
}

/// A clip's folder under `root` (ruling R41): `<root>/flute_vs_trombone_frames0-90`, or
/// `<root>/flute_vs_trombone_seed<N>_frames0-90` when the first is taken; never the plain name of a video.
pub fn clip_folder_name(names: [&str; 2], seed: u64, frames: &Range<usize>, root: &Path) -> PathBuf {
    let stem = folder_stem(names);
    let (a, b) = (frames.start, frames.end);
    let first = root.join(format!("{stem}_frames{a}-{b}"));
    if taken(&first) {
        root.join(format!("{stem}_seed{seed}_frames{a}-{b}"))
    } else {
        first
    }
}

/// The text of `caption.txt`: the caption, then the tags for Instagram, TikTok and Shorts as the kit's `post`
/// step prints them (`shorts` is its lines, one under the other).
pub fn caption_text(ff: &FightFile) -> String {
    let c = &ff.captions;
    format!(
        "{}\ninstagram: {}\ntiktok: {}\nshorts: {}\n",
        c.caption,
        c.instagram,
        c.tiktok,
        c.shorts.join("\n")
    )
}

/// An RGB picture as PNG bytes.
pub fn png_bytes(img: &Rgb8) -> Vec<u8> {
    assert_eq!(img.c, 3, "a cover is RGB");
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, img.w as u32, img.h as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("writing into memory");
    writer.write_image_data(&img.data).expect("writing into memory");
    writer.finish().expect("writing into memory");
    bytes
}

/// Tries `op` again for a moment while another program (an antivirus scan, an ffmpeg that has just ended) still
/// holds the file. "Not found" and "already exists" are answers, not a busy file: they are never tried again.
fn retry<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut last = None;
    for k in 0..20u64 {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) if matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::AlreadyExists) => return Err(e),
            Err(e) => last = Some(e),
        }
        std::thread::sleep(Duration::from_millis(25 * (k + 1).min(8)));
    }
    Err(last.expect("tried"))
}

/// A plain reason for a failed file operation.
fn reason(e: &io::Error) -> &'static str {
    match e.kind() {
        io::ErrorKind::PermissionDenied => "access is denied",
        io::ErrorKind::StorageFull => "the drive is full",
        io::ErrorKind::ReadOnlyFilesystem => "the drive is read-only",
        io::ErrorKind::NotFound => "a part of the path is missing",
        io::ErrorKind::AlreadyExists => "the name is taken",
        _ => "the system refused",
    }
}

/// Moves the file or folder `from` to `to` on the same drive, refusing (`AlreadyExists`) if anything at all is at
/// `to`: an empty folder and a file are refused too, which `fs::rename` would replace on Windows.
#[cfg(windows)]
pub(crate) fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
    let (a, b) = (wide(from), wide(to));
    // SAFETY: two NUL-terminated UTF-16 strings that live across the call; without MOVEFILE_REPLACE_EXISTING the
    // call fails when the new name exists.
    let ok = unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), MOVEFILE_WRITE_THROUGH) };
    if ok != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Elsewhere (the app is for Windows): a check, then `rename`, which leaves a short window.
#[cfg(not(windows))]
pub(crate) fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    if taken(to) {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    fs::rename(from, to)
}

#[cfg(windows)]
fn wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// Sets or clears the Windows Hidden attribute of `path` (best effort: a folder that cannot be hidden still works).
#[cfg(windows)]
fn set_hidden(path: &Path, hidden: bool) {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW,
    };
    let p = wide(path);
    // SAFETY: a NUL-terminated UTF-16 string that lives across both calls.
    unsafe {
        let now = GetFileAttributesW(p.as_ptr());
        if now == INVALID_FILE_ATTRIBUTES {
            return;
        }
        let want = if hidden {
            now | FILE_ATTRIBUTE_HIDDEN
        } else {
            now & !FILE_ATTRIBUTE_HIDDEN
        };
        if want != now {
            SetFileAttributesW(p.as_ptr(), want);
        }
    }
}

#[cfg(not(windows))]
fn set_hidden(_: &Path, _: bool) {}

/// The output folder `root`, made if it is missing; an error in words if it is a file or cannot be made.
pub(crate) fn prepare_root(root: &Path) -> Result<(), ExportError> {
    match fs::metadata(root) {
        Ok(m) if m.is_dir() => Ok(()),
        Ok(_) => Err(ExportError::Folder(format!(
            "The output folder {} is not a folder (a file has that name), so nothing was saved; choose another \
             folder.",
            root.display()
        ))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => fs::create_dir_all(root).map_err(|e| {
            ExportError::Folder(format!(
                "The output folder {} is missing and could not be made ({}), so nothing was saved; choose another \
                 folder.",
                root.display(),
                reason(&e)
            ))
        }),
        Err(e) => Err(ExportError::Folder(format!(
            "The output folder {} could not be checked ({}), so nothing was saved; check that its drive is \
             connected, or choose another folder.",
            root.display(),
            reason(&e)
        ))),
    }
}

/// A job's temporary folder: made by the job itself, hidden, holding only the files it registered.
pub(crate) struct TempFolder {
    path: PathBuf,
    files: Vec<PathBuf>,
    /// Moved into place, kept on purpose or already removed: nothing to clean up.
    done: bool,
}

impl TempFolder {
    /// Makes `<root>/.tmp-<name>-<pid>` (with `-2`, `-3` ... added if that name is taken by anything: what is
    /// there is never touched) and hides it.
    pub fn create(root: &Path, name: &str) -> Result<TempFolder, ExportError> {
        let pid = std::process::id();
        for n in 1..=64u32 {
            let base = format!(".tmp-{name}-{pid}");
            let path = root.join(if n == 1 { base } else { format!("{base}-{n}") });
            match fs::create_dir(&path) {
                Ok(()) => {
                    set_hidden(&path, true);
                    return Ok(TempFolder {
                        path,
                        files: Vec::new(),
                        done: false,
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(cannot_write(root, &e)),
            }
        }
        Err(ExportError::Folder(format!(
            "The output folder {} already holds 64 temporary folders of this app named .tmp-{name}-{pid}, so \
             nothing was saved; delete them, then try again.",
            root.display()
        )))
    }

    /// The path of the file `name` in the folder, registered so a failed job removes it.
    pub fn file(&mut self, name: &str) -> PathBuf {
        let p = self.path.join(name);
        if !self.files.contains(&p) {
            self.files.push(p.clone());
        }
        p
    }

    /// Writes `bytes` to the file `name`.
    pub fn write(&mut self, name: &str, bytes: &[u8]) -> Result<PathBuf, ExportError> {
        let p = self.file(name);
        fs::write(&p, bytes).map_err(|e| cannot_write(&self.path, &e))?;
        Ok(p)
    }

    /// Renames the file `from` to `to` inside the folder (never over a file there).
    pub fn rename(&mut self, from: &str, to: &str) -> Result<PathBuf, ExportError> {
        let (a, b) = (self.path.join(from), self.file(to));
        retry(|| move_no_replace(&a, &b)).map_err(|e| {
            ExportError::Folder(format!(
                "The finished file {} could not be renamed to {} ({}), so nothing was saved; try again.",
                a.display(),
                b.display(),
                reason(&e)
            ))
        })?;
        Ok(b)
    }

    /// Removes the file `name` (registered before) now.
    pub fn remove(&mut self, name: &str) -> Result<(), ExportError> {
        let p = self.path.join(name);
        match retry(|| fs::remove_file(&p)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ExportError::Folder(format!(
                "The file {} could not be removed from the temporary folder ({}), so nothing was saved; try again.",
                p.display(),
                reason(&e)
            ))),
        }?;
        self.files.retain(|f| *f != p);
        Ok(())
    }

    /// Moves the folder to `target` with a move that cannot replace anything. If anything is at `target` by
    /// then, the finished folder is kept where it is (shown again, not hidden) and the error names both; that is
    /// never tried again.
    pub fn move_to(&mut self, target: &Path) -> Result<(), ExportError> {
        set_hidden(&self.path, false);
        match retry(|| move_no_replace(&self.path, target)) {
            Ok(()) => {
                self.done = true;
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                self.done = true; // kept: nothing is lost
                Err(ExportError::Folder(format!(
                    "A folder named {} appeared in the output folder while the video was made, so the finished \
                     video was left in {}; move it out of there yourself.",
                    target.display(),
                    self.path.display()
                )))
            }
            Err(e) => {
                set_hidden(&self.path, true);
                Err(ExportError::Folder(format!(
                    "The finished video could not be moved to {} ({}), so nothing was saved; check the output \
                     folder, then render it again.",
                    target.display(),
                    reason(&e)
                )))
            }
        }
    }

    /// Removes the registered files and then the folder itself (only if it is a real folder, not a link, and
    /// empty by then). `Err` is a clause naming what was left behind.
    pub fn discard(&mut self) -> Result<(), String> {
        if self.done {
            return Ok(());
        }
        self.done = true;
        let is_dir = fs::symlink_metadata(&self.path).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink());
        if !is_dir {
            return Ok(());
        }
        for f in &self.files {
            match retry(|| fs::remove_file(f)) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(_) => break,
            }
        }
        match retry(|| fs::remove_dir(&self.path)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(format!(
                "the temporary folder {} could not be removed, so delete it yourself",
                self.path.display()
            )),
        }
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = self.discard();
    }
}

/// "The output folder X cannot be written to (reason) ...".
fn cannot_write(dir: &Path, e: &io::Error) -> ExportError {
    ExportError::Folder(format!(
        "The output folder {} cannot be written to ({}), so nothing was saved; check that it is not read-only and \
         that the drive has free space, or choose another folder.",
        dir.display(),
        reason(e)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(test: &str) -> PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp")
            .join(format!("bf-export output Zoë! {test} {}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[cfg(windows)]
    fn hidden(p: &Path) -> bool {
        use std::os::windows::fs::MetadataExt;
        fs::metadata(p).unwrap().file_attributes() & 2 != 0
    }

    #[test]
    fn stems_follow_the_app() {
        assert_eq!(folder_stem(["FLUTE", "TROMBONE"]), "flute_vs_trombone");
        assert_eq!(folder_stem(["ALTO SAX", "BASS  CLARINET"]), "alto_sax_vs_bass_clarinet");
    }

    #[test]
    fn a_failed_job_removes_only_its_own_files() {
        let root = temp("discard");
        let mut t = TempFolder::create(&root, "flute_vs_trombone").unwrap();
        assert!(
            t.path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".tmp-flute_vs_trombone-")
        );
        #[cfg(windows)]
        assert!(hidden(&t.path), "the temporary folder is hidden");
        // a second job with the same name gets its own folder
        let mut u = TempFolder::create(&root, "flute_vs_trombone").unwrap();
        assert_ne!(t.path, u.path);
        t.write("a.txt", b"mine").unwrap();
        // a file the job did not write stays, and so does the folder that holds it
        fs::write(t.path.join("someone else's.txt"), b"theirs").unwrap();
        assert!(t.discard().is_err());
        assert_eq!(fs::read(t.path.join("someone else's.txt")).unwrap(), b"theirs");
        assert!(!t.path.join("a.txt").exists());
        u.write("b.txt", b"x").unwrap();
        drop(u);
        let left: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().path()).collect();
        assert_eq!(left, [t.path.clone()]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_move_primitive_never_replaces_anything() {
        // the move itself, with no check before it: an empty folder, a file and a full folder all stay
        let root = temp("primitive");
        let from = root.join("new");
        fs::create_dir(&from).unwrap();
        fs::write(from.join("v.mp4"), b"new").unwrap();
        let empty = root.join("empty");
        fs::create_dir(&empty).unwrap();
        let file = root.join("a file");
        fs::write(&file, b"keep").unwrap();
        let full = root.join("full");
        fs::create_dir(&full).unwrap();
        fs::write(full.join("x"), b"keep").unwrap();
        for to in [&empty, &file, &full] {
            let e = move_no_replace(&from, to).unwrap_err();
            assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{}: {e}", to.display());
        }
        assert!(empty.is_dir() && fs::read_dir(&empty).unwrap().count() == 0);
        assert_eq!(fs::read(&file).unwrap(), b"keep");
        assert_eq!(fs::read(full.join("x")).unwrap(), b"keep");
        assert_eq!(fs::read(from.join("v.mp4")).unwrap(), b"new");
        // a file onto a file
        let other = root.join("other file");
        fs::write(&other, b"other").unwrap();
        assert_eq!(
            move_no_replace(&file, &other).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            (fs::read(&file).unwrap(), fs::read(&other).unwrap()),
            (b"keep".to_vec(), b"other".to_vec())
        );
        // a free name works
        move_no_replace(&from, &root.join("moved")).unwrap();
        assert_eq!(fs::read(root.join("moved").join("v.mp4")).unwrap(), b"new");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_name_taken_meanwhile_keeps_the_finished_folder() {
        let root = temp("move");
        let mut t = TempFolder::create(&root, "x").unwrap();
        t.write("v.mp4", b"new").unwrap();
        let target = root.join("x");
        fs::create_dir(&target).unwrap();
        let err = t.move_to(&target).unwrap_err();
        assert!(matches!(err, ExportError::Folder(_)));
        assert!(
            err.to_string().contains("appeared") && err.to_string().contains(".tmp-x-"),
            "{err}"
        );
        assert_eq!(
            fs::read_dir(&target).unwrap().count(),
            0,
            "the empty folder is untouched"
        );
        let kept = t.path.clone();
        drop(t);
        assert_eq!(
            fs::read(kept.join("v.mp4")).unwrap(),
            b"new",
            "the finished video is kept"
        );

        let mut t = TempFolder::create(&root, "y").unwrap();
        t.write("v.mp4", b"new").unwrap();
        t.rename("v.mp4", "w.mp4").unwrap();
        let target = root.join("y");
        t.move_to(&target).unwrap();
        drop(t); // moved: nothing is removed
        assert_eq!(fs::read(target.join("w.mp4")).unwrap(), b"new");
        #[cfg(windows)]
        assert!(!hidden(&target), "the video's folder is not hidden");
        assert_eq!(folder_name(["X", "Y"], 7, &root), root.join("x_vs_y"));
        fs::create_dir(root.join("x_vs_y")).unwrap();
        assert_eq!(folder_name(["X", "Y"], 7, &root), root.join("x_vs_y_seed7"));
        assert_eq!(
            clip_folder_name(["X", "Y"], 7, &(0..90), &root),
            root.join("x_vs_y_frames0-90")
        );
        fs::create_dir(root.join("x_vs_y_frames0-90")).unwrap();
        assert_eq!(
            clip_folder_name(["X", "Y"], 7, &(0..90), &root),
            root.join("x_vs_y_seed7_frames0-90")
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
