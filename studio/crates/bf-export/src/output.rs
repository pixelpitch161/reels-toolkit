//! The output folder (§9): `<output folder>\<a>_vs_<b>\` (or `<a>_vs_<b>_seed<N>\` when that name is taken)
//! holding `<name>.mp4`, `cover.png`, `caption.txt` and the fight file `<name>.toml`. A clip (ruling R41) is
//! `<a>_vs_<b>_frames<A>-<B>\` (or `<a>_vs_<b>_seed<N>_frames<A>-<B>\`) holding only `<name>.mp4`, so the
//! Library, which lists folders with a `cover.png`, never takes it for a video.
//!
//! A video is made in a hidden temporary folder beside its target, `.tmp-<name>-<pid>` (the Library also skips
//! names that start with "."), and moved into place only when every check has passed. Nothing that exists is
//! ever overwritten or deleted: the target name is decided before any work and must be free, and the move is
//! one that cannot replace anything (`MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`): if the name was taken
//! meanwhile, the finished folder is shown (no longer hidden) and moved, with the same move, to
//! `<name>_kept-<k>` beside it (k from 1 to [`MAX_TEMP_TRIES`]), and the error names that folder. So a
//! temporary folder never keeps a finished video, unless every one of those names is taken too: then it stays
//! where it is, shown, and the error says so. A failed or cancelled job removes only the files it wrote into its
//! own temporary folder (by their exact paths) and then that folder, which must be empty by then (never a
//! recursive delete, never through a link).
//!
//! The temporary folder's name, exactly: `.tmp-<name>-<pid>` or `.tmp-<name>-<pid>-<n>`, where `<name>` is the
//! video's folder name, `<pid>` is the process id of the program that made it (in decimal, no leading zero) and
//! `<n>` runs from 2 to [`MAX_TEMP_TRIES`] when the plain name was taken. [`temp_folder_owners`] reads it back and
//! [`temp_folder_files`] lists the files a job writes into it, so that the app can clear what an abandoned render
//! left (`bf-app`'s `engine::files::clear_abandoned`).

use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
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

/// How a temporary folder's name starts.
pub const TEMP_PREFIX: &str = ".tmp-";

/// How many names a job tries for its temporary folder (`-2` to `-64` after the plain one), and how many
/// `_kept-<k>` names for a finished folder whose name was taken meanwhile.
pub const MAX_TEMP_TRIES: u32 = 64;

/// The names of this program's temporary folders that a job is still using (made and not yet dropped).
static IN_USE: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn in_use() -> std::sync::MutexGuard<'static, Vec<String>> {
    IN_USE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether a job of this program is still using a temporary folder named `folder_name` (in any output folder).
pub fn temp_folder_in_use(folder_name: &str) -> bool {
    in_use().iter().any(|n| n == folder_name)
}

/// A decimal number as this program writes one: digits only, no leading zero, no sign.
fn plain_number(text: &str) -> Option<u32> {
    let first = text.bytes().next()?;
    if first == b'0' || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Every way `folder_name` reads as one of this program's temporary folders, as (the video's folder name, the
/// pid of the program that made it); empty when it is not one. The form is exactly `.tmp-<name>-<pid>` or
/// `.tmp-<name>-<pid>-<n>` with n from 2 to [`MAX_TEMP_TRIES`]; `<name>` holds `_vs_` (every video's and clip's
/// folder name does, [`folder_stem`]) and no path separator; `<pid>` and `<n>` are written without a leading zero.
/// A name with a try number, such as `.tmp-a_vs_b-1234-2`, can be read two ways (pid 1234 and try 2, or pid 2 of
/// the name `a_vs_b-1234`): both are given, so a caller can refuse when either program is running and accept only
/// the files of either name.
pub fn temp_folder_owners(folder_name: &str) -> Vec<(String, u32)> {
    let Some(rest) = folder_name.strip_prefix(TEMP_PREFIX) else {
        return Vec::new();
    };
    let usable = |name: &str| name.contains("_vs_") && !name.contains(['/', '\\', ':']);
    let mut owners = Vec::new();
    // `.tmp-<name>-<pid>`
    if let Some((name, pid)) = rest.rsplit_once('-')
        && let Some(pid) = plain_number(pid)
        && usable(name)
    {
        owners.push((name.to_string(), pid));
    }
    // `.tmp-<name>-<pid>-<n>`
    if let Some((front, n)) = rest.rsplit_once('-')
        && plain_number(n).is_some_and(|n| (2..=MAX_TEMP_TRIES).contains(&n))
        && let Some((name, pid)) = front.rsplit_once('-')
        && let Some(pid) = plain_number(pid)
        && usable(name)
    {
        owners.push((name.to_string(), pid));
    }
    owners
}

/// The files a job writes into the temporary folder of the video folder `name`, other than the finished video
/// `<name>.mp4` and the fight file `<name>.toml` (which mean the folder holds a finished video). These are all the
/// names `job.rs`'s `make` gives to `TempFolder::write`, `file` and `rename` (a test reads `job.rs` to hold this
/// list to it): the sound for the encode, the video while it is made, the Faster setting's note, the cover and the
/// caption.
pub fn temp_folder_files(name: &str) -> [String; 5] {
    [
        "audio.wav".to_string(),
        format!("{name}.mp4.part"),
        crate::encode::ENCODING_NOTE.to_string(),
        "cover.png".to_string(),
        "caption.txt".to_string(),
    ]
}

/// Takes one mark of `folder_name` out of [`IN_USE`].
fn release(folder_name: &str) {
    let mut marks = in_use();
    if let Some(at) = marks.iter().position(|n| n == folder_name) {
        marks.swap_remove(at);
    }
}

/// `<target>_kept-<k>`, beside `target`.
fn kept_name(target: &Path, k: u32) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(format!("_kept-{k}"));
    target.with_file_name(name)
}

/// Moves the folder `from` to the first free `<target>_kept-<k>` (k from 1 to [`MAX_TEMP_TRIES`]) with the move
/// that never replaces anything, and gives that name; or why not, as a clause. Each name is tried once; a refusal
/// other than "the name is taken" stops at once.
fn keep_beside(from: &Path, target: &Path) -> Result<PathBuf, String> {
    for k in 1..=MAX_TEMP_TRIES {
        let kept = kept_name(target, k);
        match retry(|| move_no_replace(from, &kept)) {
            Ok(()) => return Ok(kept),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{} could not be used ({})", kept.display(), reason(&e))),
        }
    }
    Err(format!(
        "{} to {} are all taken",
        kept_name(target, 1).display(),
        kept_name(target, MAX_TEMP_TRIES).display()
    ))
}

/// A job's temporary folder: made by the job itself, hidden, holding only the files it registered.
pub(crate) struct TempFolder {
    path: PathBuf,
    files: Vec<PathBuf>,
    /// Moved into place, kept on purpose or already removed: nothing to clean up.
    done: bool,
    /// The folder's name in [`IN_USE`] until this is dropped.
    name: String,
}

impl TempFolder {
    /// Makes `<root>/.tmp-<name>-<pid>` (with `-2`, `-3` ... added if that name is taken by anything: what is
    /// there is never touched) and hides it.
    pub fn create(root: &Path, name: &str) -> Result<TempFolder, ExportError> {
        let pid = std::process::id();
        for n in 1..=MAX_TEMP_TRIES {
            let base = format!("{TEMP_PREFIX}{name}-{pid}");
            let folder_name = if n == 1 { base } else { format!("{base}-{n}") };
            let path = root.join(&folder_name);
            // Marked in use before it exists, so nothing of this program ever takes it for an abandoned one.
            in_use().push(folder_name.clone());
            match fs::create_dir(&path) {
                Ok(()) => {
                    set_hidden(&path, true);
                    return Ok(TempFolder {
                        path,
                        files: Vec::new(),
                        done: false,
                        name: folder_name,
                    });
                }
                Err(e) => {
                    release(&folder_name);
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        continue;
                    }
                    return Err(cannot_write(root, &e));
                }
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
    /// then, the finished folder (shown, not hidden) goes to the first free `<target>_kept-<k>` beside it with the
    /// same move, and the error names it; if none of those names can be used, it stays where it is, shown, and the
    /// error names it there. Nothing is ever replaced, and none of this is tried again.
    pub fn move_to(&mut self, target: &Path) -> Result<(), ExportError> {
        set_hidden(&self.path, false);
        match retry(|| move_no_replace(&self.path, target)) {
            Ok(()) => {
                self.done = true;
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                self.done = true; // kept: nothing is lost
                Err(ExportError::Folder(match keep_beside(&self.path, target) {
                    Ok(kept) => format!(
                        "A folder named {} appeared in the output folder while the video was made, so the \
                         finished video was saved in {} beside it instead; nothing was replaced.",
                        target.display(),
                        kept.display()
                    ),
                    Err(why) => format!(
                        "A folder named {} appeared in the output folder while the video was made, and {why}, so \
                         the finished video was left in {}; move it out of there yourself.",
                        target.display(),
                        self.path.display()
                    ),
                }))
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
        release(&self.name);
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
            err.to_string().contains("appeared") && err.to_string().contains("x_kept-1"),
            "{err}"
        );
        assert_eq!(
            fs::read_dir(&target).unwrap().count(),
            0,
            "the empty folder is untouched"
        );
        drop(t);
        assert_eq!(
            fs::read(root.join("x_kept-1").join("v.mp4")).unwrap(),
            b"new",
            "the finished video is kept"
        );
        fs::remove_dir_all(root.join("x_kept-1")).unwrap();

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

    /// The names directly in `root`, sorted.
    fn names_in(root: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_kept_video_is_visible_and_named() {
        let root = temp("kept");
        let name = "flute_vs_trombone";
        let target = root.join(name);
        // The name is taken while the video is made: the finished folder goes to `<name>_kept-1`, shown.
        let mut t = TempFolder::create(&root, name).unwrap();
        t.write(&format!("{name}.mp4"), b"video").unwrap();
        t.write(&format!("{name}.toml"), b"fight").unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(target.join("theirs.txt"), b"theirs").unwrap();
        let err = t.move_to(&target).unwrap_err().to_string();
        drop(t);
        let kept = root.join(format!("{name}_kept-1"));
        assert_eq!(
            err,
            format!(
                "A folder named {} appeared in the output folder while the video was made, so the finished video \
                 was saved in {} beside it instead; nothing was replaced.",
                target.display(),
                kept.display()
            )
        );
        assert_eq!(names_in(&root), [name.to_string(), format!("{name}_kept-1")]);
        assert_eq!(fs::read(kept.join(format!("{name}.mp4"))).unwrap(), b"video");
        assert_eq!(fs::read(kept.join(format!("{name}.toml"))).unwrap(), b"fight");
        assert_eq!(fs::read(target.join("theirs.txt")).unwrap(), b"theirs");
        #[cfg(windows)]
        assert!(!hidden(&kept), "the kept folder is not hidden");

        // `_kept-1` is taken as well (by a file, here): the next free number, and nothing is replaced.
        fs::write(root.join(format!("{name}_kept-2")), b"a file").unwrap();
        let mut t = TempFolder::create(&root, name).unwrap();
        t.write(&format!("{name}.mp4"), b"second").unwrap();
        let err = t.move_to(&target).unwrap_err().to_string();
        drop(t);
        assert!(err.contains(&format!("{name}_kept-3")), "{err}");
        assert_eq!(fs::read(kept.join(format!("{name}.mp4"))).unwrap(), b"video");
        assert_eq!(fs::read(root.join(format!("{name}_kept-2"))).unwrap(), b"a file");
        assert_eq!(
            fs::read(root.join(format!("{name}_kept-3")).join(format!("{name}.mp4"))).unwrap(),
            b"second"
        );

        // Every kept name taken: the finished folder stays, shown, and the words say where.
        for k in 1..=MAX_TEMP_TRIES {
            let _ = fs::create_dir(kept_name(&target, k));
        }
        let mut t = TempFolder::create(&root, name).unwrap();
        t.write(&format!("{name}.mp4"), b"third").unwrap();
        let err = t.move_to(&target).unwrap_err().to_string();
        let left = t.path.clone();
        drop(t);
        assert!(
            err.contains("_kept-64 are all taken") && err.contains(&left.display().to_string()),
            "{err}"
        );
        assert_eq!(fs::read(left.join(format!("{name}.mp4"))).unwrap(), b"third");
        #[cfg(windows)]
        assert!(!hidden(&left), "a folder that keeps a video is not hidden");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_temporary_folder_name_reads_back() {
        let ab = "flute_vs_trombone".to_string();
        assert_eq!(temp_folder_owners(".tmp-flute_vs_trombone-4242"), [(ab.clone(), 4242)]);
        // With a try number every name also reads as pid <n> of the name `<name>-<pid>`: both are given.
        let other = "flute_vs_trombone-4242".to_string();
        assert_eq!(
            temp_folder_owners(".tmp-flute_vs_trombone-4242-2"),
            [(other.clone(), 2), (ab.clone(), 4242)]
        );
        assert_eq!(
            temp_folder_owners(".tmp-flute_vs_trombone-4242-64"),
            [(other.clone(), 64), (ab.clone(), 4242)]
        );
        assert_eq!(
            temp_folder_owners(".tmp-flute_vs_trombone_seed7-12"),
            [("flute_vs_trombone_seed7".to_string(), 12)]
        );
        // A clip's name holds a hyphen; with a try number it reads two ways.
        assert_eq!(
            temp_folder_owners(".tmp-a_vs_b_frames0-90-77"),
            [("a_vs_b_frames0-90".to_string(), 77)]
        );
        assert_eq!(
            temp_folder_owners(".tmp-a_vs_b_frames0-90-77-3"),
            [
                ("a_vs_b_frames0-90-77".to_string(), 3),
                ("a_vs_b_frames0-90".to_string(), 77)
            ]
        );
        // What this program makes reads back as itself.
        let root = temp("names");
        let t = TempFolder::create(&root, "flute_vs_trombone").unwrap();
        let made = t.path.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(temp_folder_owners(&made), [(ab.clone(), std::process::id())]);
        drop(t);
        fs::remove_dir_all(&root).unwrap();
        // Names that only look alike.
        for name in [
            ".tmp-x-4242",
            ".tmp-flute_vs_trombone",
            ".tmp-flute_vs_trombone-",
            ".tmp-flute_vs_trombone-0",
            ".tmp-flute_vs_trombone-04242",
            ".tmp-flute_vs_trombone-+4242",
            ".tmp-flute_vs_trombone-42a",
            ".tmp-flute_vs_trombone-99999999999",
            ".tmp-flute_vs_trombone-4242.old",
            ".tmp-flute_vs_trombone-4242 (2)",
            ".tmp--4242",
            ".tmp-_vs_-",
            ".TMP-flute_vs_trombone-4242",
            "tmp-flute_vs_trombone-4242",
            "x.tmp-flute_vs_trombone-4242",
            " .tmp-flute_vs_trombone-4242",
            "flute_vs_trombone",
        ] {
            assert_eq!(temp_folder_owners(name), [], "{name}");
        }
        // A try number outside 2 to 64 is not a try number (the name then reads only one way, or none).
        assert_eq!(
            temp_folder_owners(".tmp-flute_vs_trombone-4242-65"),
            [("flute_vs_trombone-4242".to_string(), 65)]
        );
        assert_eq!(temp_folder_owners(".tmp-flute_vs_trombone-4242-01"), []);
    }

    #[test]
    fn a_temporary_folder_is_in_use_until_dropped() {
        // A name no other test uses: the marks are this whole test program's.
        let video = "in_use_vs_test";
        let root = temp("in use");
        let t = TempFolder::create(&root, video).unwrap();
        let name = t.path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(temp_folder_in_use(&name));
        let mut u = TempFolder::create(&root, video).unwrap();
        let second = u.path.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(second, format!("{name}-2"));
        assert!(temp_folder_in_use(&second));
        u.write("x.txt", b"x").unwrap();
        // Moved into place or thrown away: in use until dropped, then not.
        u.move_to(&root.join(video)).unwrap();
        assert!(temp_folder_in_use(&second));
        drop(u);
        assert!(!temp_folder_in_use(&second));
        assert!(temp_folder_in_use(&name));
        drop(t);
        assert!(!temp_folder_in_use(&name));
        // A name that could not be made is never left marked.
        fs::write(root.join("a file"), b"x").unwrap();
        assert!(TempFolder::create(&root.join("a file"), video).is_err());
        assert!(!temp_folder_in_use(&name));
        fs::remove_dir_all(&root).unwrap();
    }

    /// The list of files `temp_folder_files` gives is held to what `job.rs` writes into a temporary folder: every
    /// call that names a file in it, with the names those calls take.
    #[test]
    fn the_temporary_files_are_the_jobs() {
        let job = include_str!("job.rs");
        let mut calls: Vec<&str> = Vec::new();
        for call in ["tmp.write(", "tmp.file(", "tmp.rename("] {
            for (at, _) in job.match_indices(call) {
                let rest = &job[at + call.len()..];
                calls.push(&rest[..rest.find(')').unwrap() + 1]);
            }
        }
        calls.sort_unstable();
        assert_eq!(
            calls,
            [
                "\"audio.wav\", &wav16_bytes(&st.master)",
                "\"caption.txt\", caption_text(ff)",
                "\"cover.png\", &png_bytes(&upscale3(&cover)",
                "&format!(\"{name}.toml\")",
                "&part)",
                "&part, &mp4)",
                "ENCODING_NOTE, note.as_bytes()",
            ],
            "job.rs writes another file into its temporary folder: add it to temp_folder_files (or, if it holds a \
             finished video, to the files clear_abandoned refuses)"
        );
        assert!(job.contains("let (part, mp4) = (format!(\"{name}.mp4.part\"), format!(\"{name}.mp4\"));"));
        assert_eq!(
            temp_folder_files("a_vs_b"),
            [
                "audio.wav",
                "a_vs_b.mp4.part",
                "encoding.txt",
                "cover.png",
                "caption.txt"
            ]
        );
    }
}
