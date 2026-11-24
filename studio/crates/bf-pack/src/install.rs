//! The install and the shortcuts (plan M6, Task 11; decision D2: a plain per-user folder and two shortcuts, no
//! installer, no admin rights, no registry).
//!
//! Every function takes its folders from a `Layout`. Only `Layout::system()` names the real ones
//! (`%LOCALAPPDATA%\Programs\BALLFIGHT Studio`, the Start menu's `Programs` folder, the desktop); tests use
//! `Layout::under(<temp folder>)`.
//!
//! **The install never destroys a working copy.** Files are copied into `BALLFIGHT Studio.installing` beside the
//! program folder and checked against the package's manifest. Only then are they put in place: a fresh install is
//! one rename of the whole folder; over an earlier install, `.previous` gets a copy of the earlier manifest, each file
//! the earlier manifest lists moves into `.previous` (as an update does), each new file moves in, and the last step
//! replaces the top `MANIFEST.txt`, so the folder always has a manifest of ours. If a step fails, every step is
//! undone; if the swap is stopped from outside (a power cut), running the same install again finishes it. A file in
//! the program folder that the earlier manifest does not list (a person's own) is never replaced or moved: if a new
//! file would land on it, the install stops in words before anything changes.
//!
//! **Shortcuts** are written by `lnk`, a small writer of Windows' shell link format (MS-SHLLINK) in this file: no
//! shell command line and no COM interface tables; the target's item id list comes from Windows itself (the only
//! unsafe code, in `lnk::id_list`). Tests read every link back through Windows' own shell object.

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::PackError;
use crate::config::{CONFIG_NAME, TAG_PREFIX, UpdateConfig, check_remote, write_config};
use crate::package::io_error;
use crate::package::{
    EXE_NAME, ICON_NAME, MANIFEST_NAME, Manifest, check_package, is_link, join_relative, read_manifest,
};
use crate::sha::sha256_file;

/// The program folder's name, and the shortcuts' name.
pub const APP_NAME: &str = "BALLFIGHT Studio";
/// The shortcut file's name, in the Start menu and on the desktop.
pub const LINK_NAME: &str = "BALLFIGHT Studio.lnk";
/// Where an install builds the new folder, beside the program folder.
pub const STAGING_NAME: &str = "BALLFIGHT Studio.installing";
/// The words when the app is open while an install would replace it.
pub const CLOSE_FIRST: &str = "Close BALLFIGHT Studio first.";

/// Where things go on this PC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The program folder itself: `%LOCALAPPDATA%\Programs\BALLFIGHT Studio`.
    pub programs: PathBuf,
    /// The folder the Start menu shortcut goes in: `%APPDATA%\Microsoft\Windows\Start Menu\Programs`.
    pub start_menu: PathBuf,
    /// The desktop folder (Windows' own answer, which follows a desktop moved to OneDrive).
    pub desktop: PathBuf,
    /// The app's data folder: `%LOCALAPPDATA%\BALLFIGHT Studio\data` (logs, the source copy, builds, downloads).
    pub data: PathBuf,
}

impl Layout {
    /// The real folders of this PC's user. `None` when Windows does not say where they are.
    pub fn system() -> Option<Layout> {
        let base = directories::BaseDirs::new()?;
        let user = directories::UserDirs::new()?;
        let project = directories::ProjectDirs::from("", "", APP_NAME)?;
        Some(Layout {
            programs: base.data_local_dir().join("Programs").join(APP_NAME),
            start_menu: base
                .config_dir()
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
            desktop: user.desktop_dir()?.to_path_buf(),
            data: project.data_local_dir().to_path_buf(),
        })
    }

    /// Everything under `root`: `Programs\BALLFIGHT Studio`, `Start Menu\Programs`, `Desktop`, `data`.
    pub fn under(root: &Path) -> Layout {
        Layout {
            programs: root.join("Programs").join(APP_NAME),
            start_menu: root.join("Start Menu").join("Programs"),
            desktop: root.join("Desktop"),
            data: root.join("data"),
        }
    }

    /// The installed program.
    pub fn exe(&self) -> PathBuf {
        self.programs.join(EXE_NAME)
    }

    /// The installed icon file.
    pub fn icon(&self) -> PathBuf {
        self.programs.join(ICON_NAME)
    }

    /// The Start menu shortcut.
    pub fn start_menu_link(&self) -> PathBuf {
        self.start_menu.join(LINK_NAME)
    }

    /// The desktop shortcut.
    pub fn desktop_link(&self) -> PathBuf {
        self.desktop.join(LINK_NAME)
    }

    /// Where an install builds the new folder.
    pub fn staging(&self) -> PathBuf {
        self.programs.with_file_name(STAGING_NAME)
    }
}

/// `p` in the one form paths are compared in: the longest part of it that exists, as Windows itself names it (long
/// names rather than 8.3 short ones such as `RUNNER~1`, links followed), then the rest as given, without the `\\?\`
/// that Windows puts in front. A path given in short form and the same path read back from Windows in long form
/// then compare equal. A path whose start cannot be looked up is given back as it is.
pub(crate) fn long_form(p: &Path) -> PathBuf {
    let mut existing = p;
    let mut rest = Vec::new();
    loop {
        if let Ok(full) = fs::canonicalize(existing) {
            let mut out = without_verbatim(full);
            for name in rest.iter().rev() {
                out.push(name);
            }
            return out;
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => return p.to_path_buf(),
        }
    }
}

/// `\\?\C:\x` as `C:\x`, and `\\?\UNC\server\share` as `\\server\share`.
fn without_verbatim(p: PathBuf) -> PathBuf {
    let Some(text) = p.to_str() else { return p };
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{unc}"))
    } else if let Some(local) = text.strip_prefix(r"\\?\") {
        PathBuf::from(local)
    } else {
        p
    }
}

/// The names of a path in its long form (`long_form`), lower-cased, without `.` parts; `None` for a relative path or
/// one with `..`.
fn names(p: &Path) -> Option<Vec<String>> {
    if !p.is_absolute() || p.components().any(|c| c == Component::ParentDir) {
        return None;
    }
    let p = long_form(p);
    let mut out = Vec::new();
    for c in p.components() {
        match c {
            Component::Prefix(x) => out.push(x.as_os_str().to_string_lossy().to_lowercase()),
            Component::RootDir => out.push("\\".to_string()),
            Component::CurDir => {}
            Component::ParentDir => return None,
            Component::Normal(n) => out.push(n.to_string_lossy().to_lowercase()),
        }
    }
    Some(out)
}

/// Whether `a` is `b` or inside it, comparing names without regard to case as Windows does.
pub(crate) fn same_or_inside(a: &Path, b: &Path) -> bool {
    match (names(a), names(b)) {
        (Some(a), Some(b)) => a.len() >= b.len() && a[..b.len()] == b[..],
        _ => false,
    }
}

/// Checks that a layout's program folder is one this crate may write into or remove from: an absolute path named
/// exactly `BALLFIGHT Studio`, not a link, and not the same as, inside or around the data folder, its parent, the
/// Start menu, the desktop or the user's own folder. The words name the folder and say nothing was changed.
pub(crate) fn check_layout(layout: &Layout) -> Result<(), PackError> {
    let p = &layout.programs;
    let refuse = |why: &str| {
        Err(PackError::Refused(format!(
            "The program folder {} {why}, so nothing was changed.",
            p.display()
        )))
    };
    if names(p).is_none() {
        return refuse("is not a full path");
    }
    if p.file_name() != Some(OsStr::new(APP_NAME)) {
        return refuse("is not named BALLFIGHT Studio");
    }
    let mut others: Vec<&Path> = vec![&layout.data, &layout.start_menu, &layout.desktop];
    // The data folder's parent (`%LOCALAPPDATA%\BALLFIGHT Studio` on a real PC, also named BALLFIGHT Studio) is
    // never the program folder. Beside the data folder is fine: `Layout::under` puts both in one test root.
    if layout
        .data
        .parent()
        .is_some_and(|data_root| names(p) == names(data_root))
    {
        return refuse("is the app's data folder");
    }
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from);
    if let Some(home) = &home {
        others.push(home);
    }
    for other in others {
        if same_or_inside(other, p) {
            return refuse(&format!("holds {}", other.display()));
        }
    }
    if same_or_inside(p, &layout.data) {
        return refuse("is inside the app's data folder");
    }
    if is_link(p) {
        return refuse("is a link to another place");
    }
    Ok(())
}

/// Whether Windows refuses to open the file for writing because a running program holds it (a running exe is
/// held so; renaming it would still work, which is why this is asked first).
pub(crate) fn in_use(path: &Path) -> bool {
    match fs::OpenOptions::new().write(true).open(path) {
        Ok(_) => false,
        // ERROR_SHARING_VIOLATION and ERROR_LOCK_VIOLATION.
        Err(e) => matches!(e.raw_os_error(), Some(32) | Some(33)),
    }
}

/// Installs the package folder `package` (made by `package::package`) into `layout.programs`, with `config` as its
/// `update.toml`. See the module notes for how a working copy is kept. An install that stopped part-way (a power
/// cut, a closed window) is finished by running the same install again.
pub fn install(package: &Path, layout: &Layout, config: &UpdateConfig) -> Result<(), PackError> {
    install_stopping(package, layout, config, None)
}

/// The words of a swap stopped on purpose by a test (as a power cut would stop it: nothing undone).
const STOPPED: &str = "The install was stopped part-way for a test.";

/// `install`, which for a test stops after `stop_after` steps of the swap and leaves everything as it is then.
fn install_stopping(
    package: &Path,
    layout: &Layout,
    config: &UpdateConfig,
    stop_after: Option<usize>,
) -> Result<(), PackError> {
    check_layout(layout)?;
    let new = check_package(package)?;
    let target = &layout.programs;
    let _lock = swap_lock(target)?;
    let parent = target
        .parent()
        .ok_or_else(|| PackError::Refused("The program folder has no parent folder.".to_string()))?;
    fs::create_dir_all(parent).map_err(|e| io_error("make the folder", parent, &e))?;
    let old = installed_manifest(target)?;
    let staging = layout.staging();

    // A swap stopped after its last real step (the new manifest is in place) left only its marker: take it away.
    // If that swap was of this very package, the install is already done.
    if old.is_some() && swap_state(target) == SwapState::Committed {
        let marker = target.join(".previous").join(SWAP_MARKER);
        fs::remove_file(&marker).map_err(|e| io_error("remove", &marker, &e))?;
        if old.as_ref().is_some_and(|o| o.text() == new.text()) {
            clear_leftover_staging(&staging, old.as_ref())?;
            return Ok(());
        }
    }

    // Everything that could stop the install is checked before anything is written.
    let mut housekeeping = Vec::new();
    if let Some(old) = &old {
        refuse_running(target, old)?;
        check_conflicts(target, old, &new)?;
        // A `.previous` that an earlier swap was filling when it stopped (its marker is there) is finished, not
        // cleared.
        let resuming = swap_state(target) == SwapState::Stopped;
        for name in [".previous", ".next"] {
            let d = target.join(name);
            if (name == ".previous" && resuming) || fs::symlink_metadata(&d).is_err() {
                continue;
            }
            housekeeping.push(plan_clear(&d)?);
        }
    }
    // The installed manifest also names what is ours there: a stop just after the last step leaves staging without
    // its own manifest.
    clear_leftover_staging(&staging, old.as_ref())?;
    if let Err(e) = stage(package, &new, &staging, config) {
        let _ = clear_leftover_staging(&staging, None);
        return Err(e);
    }

    let done = match &old {
        None => {
            // An empty folder of that name is taken away so the new one can take its place in one rename.
            if target.exists() {
                fs::remove_dir(target).map_err(|e| io_error("replace the empty folder", target, &e))?;
            }
            fs::rename(&staging, target).map_err(|e| io_error("put in place", target, &e))
        }
        Some(old) => {
            let mut cleared = Ok(());
            for r in &housekeeping {
                let stayed = crate::remove::remove_now(r);
                if let Some((p, why)) = stayed.first() {
                    cleared = Err(PackError::Io(format!(
                        "A file of the copy kept from before, {}, could not be cleared away ({why}), so nothing was \
                         installed; version {} is still installed, and the copy kept from before it may be \
                         incomplete.",
                        p.display(),
                        old.version
                    )));
                    break;
                }
            }
            let cleared_any = !housekeeping.is_empty();
            cleared.and_then(|()| {
                swap_in(target, old, &new, &staging, stop_after, AGAIN_INSTALL).map_err(|e| match e {
                    PackError::Io(text) if cleared_any && text != STOPPED => {
                        PackError::Io(format!("{text} The copy kept from the version before it was cleared."))
                    }
                    other => other,
                })
            })
        }
    };
    match &done {
        Err(PackError::Io(text)) if text == STOPPED => {}
        Err(_) => {
            let _ = clear_leftover_staging(&staging, None);
        }
        Ok(()) => {}
    }
    done
}

/// The file a swap writes into `.previous` before its first change and removes after its last: while it is there,
/// the swap has not finished.
pub(crate) const SWAP_MARKER: &str = "INSTALLING";
/// The names the swap copies the earlier manifest and `update.toml` through, so each copy is whole or absent.
pub(crate) const COPY_PARTS: [&str; 2] = [".MANIFEST.txt.part", ".update.toml.part"];

/// Where an earlier swap stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SwapState {
    /// No swap is under way (no marker).
    Done,
    /// A swap was stopped before its last real step: the top manifest is still the earlier one, and `.previous`
    /// holds no manifest yet or a copy of it.
    Stopped,
    /// A swap was stopped after it put the new manifest in place: only its marker is left.
    Committed,
}

fn swap_state(target: &Path) -> SwapState {
    let prev = target.join(".previous");
    let marker = prev.join(SWAP_MARKER);
    if is_link(&prev) || is_link(&marker) || !marker.is_file() {
        return SwapState::Done;
    }
    let kept = prev.join(MANIFEST_NAME);
    if fs::symlink_metadata(&kept).is_err() {
        return SwapState::Stopped;
    }
    let same = fs::read(&kept)
        .ok()
        .is_some_and(|k| fs::read(target.join(MANIFEST_NAME)).ok() == Some(k));
    if same { SwapState::Stopped } else { SwapState::Committed }
}

/// The manifest of what is installed in `target`: `None` when the folder is missing or empty, an error when it holds
/// anything without a manifest of ours.
fn installed_manifest(target: &Path) -> Result<Option<Manifest>, PackError> {
    if fs::symlink_metadata(target).is_err() {
        return Ok(None);
    }
    let not_ours = || {
        PackError::Refused(format!(
            "A folder {} is already there and was not made by BALLFIGHT Studio's install, so nothing was installed. \
             Move it elsewhere first.",
            target.display()
        ))
    };
    if is_link(target) || !target.is_dir() {
        return Err(not_ours());
    }
    let empty = fs::read_dir(target)
        .map_err(|e| io_error("read the folder", target, &e))?
        .next()
        .is_none();
    if empty {
        return Ok(None);
    }
    read_manifest(&target.join(MANIFEST_NAME))
        .map(Some)
        .map_err(|_| not_ours())
}

/// Refuses while a program the earlier manifest lists is running from the folder.
fn refuse_running(target: &Path, old: &Manifest) -> Result<(), PackError> {
    let running = old
        .files
        .iter()
        .filter(|(p, _, _)| p.to_ascii_lowercase().ends_with(".exe"))
        .any(|(p, _, _)| in_use(&join_relative(target, p)));
    if running {
        Err(PackError::Refused(CLOSE_FIRST.to_string()))
    } else {
        Ok(())
    }
}

/// Whether any part of `rel` under `root` (each folder on the way, and the item itself) is a link.
pub(crate) fn path_has_link(root: &Path, rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    (0..parts.len()).any(|i| is_link(&join_relative(root, &parts[..=i].join("/"))))
}

/// Stops when a new file would land on something in the program folder that is not ours: a file the earlier
/// manifest does not list (unless it already has exactly the new bytes), a folder, or a link.
fn check_conflicts(target: &Path, old: &Manifest, new: &Manifest) -> Result<(), PackError> {
    let conflict = |p: &Path| {
        Err(PackError::Refused(format!(
            "{} is in the program folder and was not put there by BALLFIGHT Studio, so nothing was installed. Move \
             it elsewhere, then install again.",
            p.display()
        )))
    };
    let ours = |rel: &str| old.files.iter().any(|(p, _, _)| p == rel) || rel == MANIFEST_NAME || rel == CONFIG_NAME;
    for (rel, _, sha) in &new.files {
        let parts: Vec<&str> = rel.split('/').collect();
        for i in 0..parts.len() {
            let prefix = parts[..=i].join("/");
            let here = join_relative(target, &prefix);
            if fs::symlink_metadata(&here).is_err() {
                break;
            }
            if is_link(&here) {
                return conflict(&here);
            }
            let last = i + 1 == parts.len();
            if !last {
                if here.is_dir() {
                    continue;
                }
                // A file where the new one needs a folder: fine only if it is ours (it moves to .previous).
                if ours(&prefix) {
                    break;
                }
                return conflict(&here);
            }
            if here.is_dir() {
                return conflict(&here);
            }
            if !ours(rel) && sha256_file(&here).map_err(|e| io_error("read", &here, &e))? != *sha {
                return conflict(&here);
            }
        }
    }
    Ok(())
}

/// The removal of an earlier `.previous` or `.next`: everything in it must be ours (by its own manifest), or it is
/// left alone and the install stops in words. A folder holding nothing but empty folders counts as empty.
fn plan_clear(dir: &Path) -> Result<crate::remove::Removal, PackError> {
    let mut r = crate::remove::Removal::default();
    let foreign = |what: &Path| {
        PackError::Refused(format!(
            "{} holds {}, which BALLFIGHT Studio did not put there, so nothing was installed. Move it elsewhere first.",
            dir.display(),
            what.display()
        ))
    };
    if is_link(dir) || !dir.is_dir() {
        return Err(foreign(dir));
    }
    let entries = crate::package::walk(dir)?;
    // Only folders, and perhaps the marker and copy names of a swap stopped before it copied anything.
    let swap_name =
        |rel: &str| rel == SWAP_MARKER || COPY_PARTS.contains(&rel) || crate::package::is_set_aside_exe(rel);
    let nearly_empty = entries.iter().all(|(rel, kind)| {
        *kind == crate::package::Kind::Dir || (*kind == crate::package::Kind::File && swap_name(rel))
    });
    if nearly_empty {
        for (rel, kind) in &entries {
            if *kind == crate::package::Kind::File {
                let f = join_relative(dir, rel);
                let sha = sha256_file(&f).map_err(|e| io_error("read", &f, &e))?;
                r.files.push((f, sha));
            }
        }
        r.dirs.extend(
            entries
                .iter()
                .rev()
                .filter(|(_, k)| *k == crate::package::Kind::Dir)
                .map(|(rel, _)| join_relative(dir, rel)),
        );
        r.dirs.push(dir.to_path_buf());
        return Ok(r);
    }
    let manifest = dir.join(MANIFEST_NAME);
    if !manifest.is_file() {
        return Err(foreign(dir));
    }
    if read_manifest(&manifest).is_err() {
        return Err(PackError::Refused(format!(
            "{} cannot be read, so nothing was installed. Delete that file, then install again.",
            manifest.display()
        )));
    }
    crate::remove::plan_folder(dir, &mut r, None)?;
    if let Some((p, _)) = r.kept.first() {
        return Err(foreign(p));
    }
    Ok(r)
}

/// Clears what a stopped install left in the staging folder: only the names its own manifest (or `also`, the
/// manifest it held before that was moved into place) lists, whatever their bytes, since a copy may have stopped
/// part-way; its manifest and `update.toml`; then its empty folders. Anything else in it stops the install in words,
/// with nothing removed.
fn clear_leftover_staging(staging: &Path, also: Option<&Manifest>) -> Result<(), PackError> {
    if fs::symlink_metadata(staging).is_err() {
        return Ok(());
    }
    let refuse = || {
        PackError::Refused(format!(
            "{} holds files that BALLFIGHT Studio cannot tell are its own, so nothing was installed. Move it \
             elsewhere first.",
            staging.display()
        ))
    };
    if is_link(staging) || !staging.is_dir() {
        return Err(refuse());
    }
    let entries = crate::package::walk(staging)?;
    let manifest = read_manifest(&staging.join(MANIFEST_NAME)).ok();
    let listed = |rel: &str| {
        rel == MANIFEST_NAME
            || rel == CONFIG_NAME
            || rel == "update.toml.part"
            || manifest
                .iter()
                .chain(also)
                .any(|m| m.files.iter().any(|(p, _, _)| p == rel))
    };
    let mut dirs = Vec::new();
    for (rel, kind) in &entries {
        match kind {
            crate::package::Kind::Link => return Err(refuse()),
            crate::package::Kind::Dir => dirs.push(join_relative(staging, rel)),
            crate::package::Kind::File if listed(rel) => {}
            crate::package::Kind::File => return Err(refuse()),
        }
    }
    for (rel, kind) in &entries {
        if *kind == crate::package::Kind::File && rel != MANIFEST_NAME {
            let f = join_relative(staging, rel);
            fs::remove_file(&f).map_err(|e| io_error("remove", &f, &e))?;
        }
    }
    let m = staging.join(MANIFEST_NAME);
    if m.is_file() {
        fs::remove_file(&m).map_err(|e| io_error("remove", &m, &e))?;
    }
    for d in dirs.iter().rev() {
        fs::remove_dir(d).map_err(|e| io_error("remove the empty folder", d, &e))?;
    }
    fs::remove_dir(staging).map_err(|e| io_error("remove the empty folder", staging, &e))
}

/// Copies the package into the staging folder (its manifest first, so a stopped install can be cleared), checks
/// every file against the manifest, and writes `update.toml`.
fn stage(package: &Path, new: &Manifest, staging: &Path, config: &UpdateConfig) -> Result<(), PackError> {
    fs::create_dir(staging).map_err(|e| io_error("make the folder", staging, &e))?;
    let m = staging.join(MANIFEST_NAME);
    fs::write(&m, new.text()).map_err(|e| io_error("write", &m, &e))?;
    for (rel, bytes, sha) in &new.files {
        let to = join_relative(staging, rel);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|e| io_error("make the folder", dir, &e))?;
        }
        let (got, len) = crate::package::copy_new(&join_relative(package, rel), &to)?;
        if &got != sha || len != *bytes {
            return Err(PackError::Checksum {
                what: format!("{rel} as copied for the install"),
                want: sha.clone(),
                got,
            });
        }
    }
    write_config(&staging.join(CONFIG_NAME), config)
}

/// Makes the missing folders between `root` and `path`'s folder, noting each one made.
fn make_parents(path: &Path, root: &Path, undo: &mut Vec<Undo>) -> Result<(), PackError> {
    let Some(dir) = path.parent() else { return Ok(()) };
    let mut missing: Vec<&Path> = dir
        .ancestors()
        .take_while(|a| *a != root && same_or_inside(a, root))
        .filter(|a| fs::symlink_metadata(a).is_err())
        .collect();
    missing.reverse();
    for d in missing {
        fs::create_dir(d).map_err(|e| io_error("make the folder", d, &e))?;
        undo.push(Undo::Made(d.to_path_buf()));
    }
    Ok(())
}

/// One step of a swap, as it is taken back if a later step fails.
enum Undo {
    /// A folder made: removed again if empty.
    Made(PathBuf),
    /// A file copied to a place where nothing was: removed again.
    Copied(PathBuf),
    /// A file moved from the first place to the second: moved back.
    Moved(PathBuf, PathBuf),
    /// A file replaced by a newer one; the second is a copy of the one it replaced: copied back.
    Replaced(PathBuf, PathBuf),
    /// The new exe put over the earlier one in one replacing rename (staged, in place, the earlier one's copy in
    /// `.previous`): the new one goes back to staging and the earlier one comes back from `.previous`, each by a link
    /// and a replacing rename, so an exe is at its place at every moment.
    PutExe(PathBuf, PathBuf, PathBuf),
}

/// Makes `to` (which must not exist) the same file as `from`: a hard link, or a whole copy (through a temp name) where
/// links are not possible. A copy must have the SHA-256 `sha` when that is given, or it is removed and this fails.
fn link_or_copy(from: &Path, to: &Path, sha: Option<&str>) -> std::io::Result<()> {
    if fs::hard_link(from, to).is_ok() {
        return Ok(());
    }
    let mut part = to.as_os_str().to_os_string();
    part.push(".part");
    let part = PathBuf::from(part);
    let copied = fs::copy(from, &part).and_then(|_| match sha {
        Some(want) if sha256_file(&part)? != want => Err(std::io::Error::other(
            "the copy does not have the bytes it was made from",
        )),
        _ => fs::rename(&part, to),
    });
    if copied.is_err() {
        let _ = fs::remove_file(&part);
    }
    copied
}

#[cfg(test)]
thread_local! {
    /// For tests: refuse to rename over a file that a running program holds, as Windows does where a replacing
    /// rename has no POSIX meaning (older Windows, other file systems). On NTFS on Windows 10 and 11, Rust's rename
    /// replaces even a running program's file.
    pub(crate) static NO_REPLACE_WHILE_RUNNING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `fs::rename`, which may replace `to`.
fn rename_over(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if NO_REPLACE_WHILE_RUNNING.with(|n| n.get()) && to.is_file() && in_use(to) {
        return Err(std::io::Error::from_raw_os_error(5));
    }
    fs::rename(from, to)
}

/// Puts the exe `from` (the earlier one, kept in `.previous`) at the exe's place `to` in one replacing rename, by way
/// of a link (or a checked copy) beside it, so an exe is at the place at every moment. When the file at `to` is a
/// running program that Windows will not let be replaced, it is first renamed aside, into `aside_dir`, as
/// `ballfight-studio.exe.running-N` (renaming a running exe is allowed); a later swap or removal clears it by that
/// name once it is no longer running. `from` stays as it is.
fn replace_exe(from: &Path, to: &Path, aside_dir: &Path, sha: Option<&str>) -> std::io::Result<()> {
    let mut name = std::ffi::OsString::from(".");
    name.push(to.file_name().unwrap_or_default());
    name.push(".restore");
    let part = to.with_file_name(name);
    let _ = fs::remove_file(&part);
    link_or_copy(from, &part, sha)?;
    let put = rename_over(&part, to).or_else(|first| {
        if fs::symlink_metadata(to).is_err() {
            return Err(first);
        }
        let aside = (1..100_000)
            .map(|n| aside_dir.join(crate::package::set_aside_exe_name(n)))
            .find(|p| fs::symlink_metadata(p).is_err())
            .ok_or(first)?;
        fs::rename(to, &aside)?;
        fs::rename(&part, to).inspect_err(|_| {
            let _ = fs::rename(&aside, to);
        })
    });
    if put.is_err() {
        let _ = fs::remove_file(&part);
    }
    put
}
/// Whether `rel` (a manifest path) is the program itself, the file the shortcuts open.
fn is_the_exe(rel: &str) -> bool {
    rel.eq_ignore_ascii_case(EXE_NAME)
}

/// Counts a swap's steps, and stops after the number a test asks for.
struct Steps {
    taken: usize,
    stop_after: Option<usize>,
}

impl Steps {
    fn next(&mut self) -> Result<(), PackError> {
        if self.stop_after.is_some_and(|n| self.taken >= n) {
            return Err(PackError::Io(STOPPED.to_string()));
        }
        self.taken += 1;
        Ok(())
    }
}

/// Puts the staged files in place over an earlier install.
///
/// The top `MANIFEST.txt` stays in place until the very last step, which replaces it with the new one, so the
/// folder is always one of ours. Before that: `.previous` gets a copy of the earlier manifest and `update.toml`;
/// each file the earlier manifest lists moves into `.previous` (unless a stopped swap already moved it, or any part
/// of its path is a link); each new file moves in (unless the same bytes are already there); `update.toml` is
/// replaced. If any step fails, every step is taken back in reverse. A swap stopped from outside (a power cut) is
/// finished by the next install: listed names are ours, new files already in place have the new bytes, and staging
/// is cleared by its own manifest.
///
/// **The exe is never away from its place** (where the shortcuts point): the earlier exe gets a hard link (or a copy)
/// in `.previous` and stays where it is; the new exe is the first new file put in, by one rename that replaces the
/// earlier one; taking that back puts the earlier one back the same way. Every stop leaves the whole earlier or the
/// whole new exe there. `again` says what finishes a swap that could not be taken back.
fn swap_in(
    target: &Path,
    old: &Manifest,
    new: &Manifest,
    staging: &Path,
    stop_after: Option<usize>,
    again: &str,
) -> Result<(), PackError> {
    let prev = target.join(".previous");
    let mut undo: Vec<Undo> = Vec::new();
    let mut steps = Steps { taken: 0, stop_after };

    let moved = (|| {
        if fs::symlink_metadata(&prev).is_err() {
            steps.next()?;
            fs::create_dir(&prev).map_err(|e| io_error("make the folder", &prev, &e))?;
            undo.push(Undo::Made(prev.clone()));
        }
        // The marker goes in before the first change and comes out after the last: while it is there, the next
        // install knows this swap did not finish.
        let marker = prev.join(SWAP_MARKER);
        if fs::symlink_metadata(&marker).is_err() {
            steps.next()?;
            fs::write(&marker, "BALLFIGHT Studio is putting a new version in place.\r\n")
                .map_err(|e| io_error("write", &marker, &e))?;
            undo.push(Undo::Copied(marker.clone()));
        }
        for (name, part) in [MANIFEST_NAME, CONFIG_NAME].into_iter().zip(COPY_PARTS) {
            let from = target.join(name);
            let to = prev.join(name);
            let part = prev.join(part);
            if is_link(&from) || !from.is_file() || is_link(&to) || is_link(&part) {
                continue;
            }
            let was_there = fs::symlink_metadata(&to).is_ok();
            steps.next()?;
            // Through a temp name, so the copy is whole or not there at all.
            let copied = fs::copy(&from, &part)
                .map_err(|e| io_error("copy the earlier version's", &from, &e))
                .and_then(|_| fs::rename(&part, &to).map_err(|e| io_error("put in place", &to, &e)));
            if copied.is_err() {
                let _ = fs::remove_file(&part);
            }
            copied?;
            if !was_there {
                undo.push(Undo::Copied(to));
            }
        }
        for (rel, _, sha) in &old.files {
            let from = join_relative(target, rel);
            let to = join_relative(&prev, rel);
            let already_kept = fs::symlink_metadata(&to).is_ok();
            if already_kept || path_has_link(target, rel) || path_has_link(&prev, rel) || !from.is_file() {
                continue;
            }
            // Only the whole earlier exe is kept (by a link or a copy) and it is left at its place.
            if is_the_exe(rel) && sha256_file(&from).ok().as_ref() != Some(sha) {
                continue;
            }
            make_parents(&to, &prev, &mut undo)?;
            steps.next()?;
            if is_the_exe(rel) {
                link_or_copy(&from, &to, Some(sha))
                    .map_err(|e| io_error("keep a copy of the earlier version's", &from, &e))?;
                undo.push(Undo::Copied(to));
            } else {
                fs::rename(&from, &to).map_err(|e| io_error("move the earlier version's", &from, &e))?;
                undo.push(Undo::Moved(from, to));
            }
        }
        // The exe first, so the earlier files' going leaves the new exe (or the earlier one) in place.
        let mut order: Vec<&(String, u64, String)> = new.files.iter().collect();
        order.sort_by_key(|(rel, _, _)| !is_the_exe(rel));
        for (rel, _, sha) in order {
            let from = join_relative(staging, rel);
            let to = join_relative(target, rel);
            let in_place = !is_link(&to) && to.is_file() && sha256_file(&to).is_ok_and(|now| &now == sha);
            if in_place {
                continue;
            }
            make_parents(&to, target, &mut undo)?;
            steps.next()?;
            fs::rename(&from, &to).map_err(|e| io_error("put in place", &to, &e))?;
            if is_the_exe(rel) {
                undo.push(Undo::PutExe(from, to, join_relative(&prev, rel)));
            } else {
                undo.push(Undo::Moved(from, to));
            }
        }
        // An install brings its `update.toml`; an update (`put_in_place`) brings none, and the one in place stays.
        if staging.join(CONFIG_NAME).is_file() {
            let config = target.join(CONFIG_NAME);
            steps.next()?;
            let kept_config = prev.join(CONFIG_NAME);
            fs::rename(staging.join(CONFIG_NAME), &config).map_err(|e| io_error("put in place", &config, &e))?;
            if kept_config.is_file() {
                undo.push(Undo::Replaced(config, kept_config));
            } else {
                undo.push(Undo::Moved(staging.join(CONFIG_NAME), config));
            }
        }
        // The last step, and the one that makes the new version the installed one.
        let manifest = target.join(MANIFEST_NAME);
        steps.next()?;
        fs::rename(staging.join(MANIFEST_NAME), &manifest).map_err(|e| io_error("put in place", &manifest, &e))?;
        // Done: the marker comes out. If that fails, the next install sees the new manifest in place and only
        // removes the marker, so it is never a reason to undo.
        steps.next()?;
        let _ = fs::remove_file(&marker);
        Ok::<(), PackError>(())
    })();

    if let Err(e) = moved {
        if matches!(&e, PackError::Io(t) if t == STOPPED) {
            return Err(e);
        }
        let mut not_back = Vec::new();
        for step in undo.iter().rev() {
            // When something could not be put back, the marker stays (the swap is not finished, and the next start
            // or install finishes or takes it back), and so do the copies in `.previous` (the earlier exe's, the
            // manifest's), since taking the swap back later needs them.
            if !not_back.is_empty() && matches!(step, Undo::Copied(_) | Undo::Made(_)) {
                continue;
            }
            let back = match step {
                Undo::Made(d) => {
                    let _ = fs::remove_dir(d);
                    Ok(())
                }
                Undo::Copied(f) => fs::remove_file(f).map(|_| ()),
                Undo::Moved(from, to) => fs::rename(to, from),
                Undo::Replaced(f, copy) => fs::copy(copy, f).map(|_| ()),
                Undo::PutExe(staged, placed, kept) => put_exe_back(staged, placed, kept),
            };
            if back.is_err() {
                let p = match step {
                    Undo::Made(p) | Undo::Copied(p) | Undo::Replaced(p, _) => p,
                    Undo::Moved(from, _) | Undo::PutExe(from, _, _) => from,
                };
                not_back.push(p.display().to_string());
            }
        }
        let text = if not_back.is_empty() {
            format!("{e} So nothing changed: version {} is still installed.", old.version)
        } else {
            format!(
                "{e} Putting the earlier version back failed for {}; {again}.",
                not_back.join(", ")
            )
        };
        return Err(PackError::Io(text));
    }

    // Folders the earlier version had and the new one does not: removed when empty, deepest first.
    let mut old_dirs: Vec<PathBuf> = old
        .files
        .iter()
        .flat_map(|(rel, _, _)| {
            let parts: Vec<&str> = rel.split('/').collect();
            (1..parts.len()).map(move |n| parts[..n].join("/")).collect::<Vec<_>>()
        })
        .map(|d| join_relative(target, &d))
        .collect();
    old_dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    old_dirs.dedup();
    for d in old_dirs {
        if !is_link(&d) {
            let _ = fs::remove_dir(&d);
        }
    }
    let _ = clear_leftover_staging(staging, Some(new));
    Ok(())
}

#[cfg(test)]
thread_local! {
    /// For a test: putting the earlier exe back fails, as it would if its kept copy could not be read.
    pub(crate) static FAIL_EXE_BACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Takes back the placing of the new exe: the new one returns to `staged` by a link (the place keeps it meanwhile),
/// then the earlier one comes back from its copy in `.previous` by one replacing rename (`replace_exe`).
fn put_exe_back(staged: &Path, placed: &Path, kept: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if FAIL_EXE_BACK.with(|f| f.get()) {
        return Err(std::io::Error::other("putting it back failed for a test"));
    }
    if fs::symlink_metadata(staged).is_err() {
        link_or_copy(placed, staged, None)?;
    }
    replace_exe(kept, placed, kept.parent().unwrap_or(kept), None)
}

/// What finishes an install that could not be taken back.
const AGAIN_INSTALL: &str = "installing again finishes the install";
/// What finishes or undoes an update that could not be taken back.
pub const AGAIN_UPDATE: &str = "starting BALLFIGHT Studio again finishes or undoes the update by itself";

/// The words when another program is putting a version in place in the same folder right now.
pub const SWAP_BUSY: &str = "An update is being put in place. Start the app again in a moment.";

/// The lock that keeps two swaps (an install's, an update's, "Go back", a start finishing a stopped one) out of one
/// program folder at the same time: an exclusive lock on `BALLFIGHT Studio.update.lock` beside the folder, held
/// until this is dropped.
pub struct SwapLock {
    _file: fs::File,
}

/// The lock file of `target`'s swaps, beside it.
pub fn swap_lock_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".update.lock");
    target.with_file_name(name)
}

/// Takes the swap lock of `target`, or refuses with `SWAP_BUSY` when another program holds it.
pub fn swap_lock(target: &Path) -> Result<SwapLock, PackError> {
    let path = swap_lock_path(target);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| io_error("make the folder", dir, &e))?;
    }
    if is_link(&path) {
        return Err(PackError::Refused(format!(
            "{} is a link, so it is not used.",
            path.display()
        )));
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| match e.raw_os_error() {
            // Held open by another program without sharing.
            Some(32) | Some(33) => PackError::Refused(SWAP_BUSY.to_string()),
            _ => io_error("open", &path, &e),
        })?;
    match file.try_lock() {
        Ok(()) => Ok(SwapLock { _file: file }),
        Err(fs::TryLockError::WouldBlock) => Err(PackError::Refused(SWAP_BUSY.to_string())),
        Err(fs::TryLockError::Error(e)) => Err(io_error("lock", &path, &e)),
    }
}

/// Whether a swap into `target` (an install's or an update's) was stopped before it finished: its marker is still in
/// `.previous`. Starting the app finishes or undoes an update's; installing again finishes an install's.
pub fn swap_unfinished(target: &Path) -> bool {
    swap_state(target) != SwapState::Done
}

/// The update's swap (plan M6, Task 12): puts the checked package in `staged` (the program folder's `.next`, made by
/// the update engine) in place of the version installed in `target`, by the same steps as an install over an earlier
/// one (`swap_in`): the marker first, each file the installed manifest lists into a fresh `.previous` (kept for "Go
/// back"), each new file in, the new manifest last. A person's files are never moved; `update.toml` stays as it is.
/// If a step fails, every step is undone and the earlier version stays installed, whole. A swap that was stopped from
/// outside (a power cut) is finished by calling this again (or by installing again). Gives the new manifest.
///
/// Refused, with nothing changed: no version of ours in `target`; a staged folder that is not a whole package by its
/// manifest; a new file that would land on something that is not ours; and, as `CLOSE_FIRST`, a program of the
/// installed version that is still running (a program already replaced by its new bytes, such as the new version
/// finishing its own swap, does not count). An earlier `.previous` is cleared first, only by its own manifest.
///
/// It takes the swap lock (`SWAP_BUSY` when another program holds it); `put_in_place_held` is for a caller that
/// already holds it.
pub fn put_in_place(target: &Path, staged: &Path) -> Result<Manifest, PackError> {
    let _lock = swap_lock(target)?;
    put_in_place_stopping(target, staged, None)
}

/// `put_in_place` for a caller holding the swap lock.
pub fn put_in_place_held(target: &Path, staged: &Path, _lock: &SwapLock) -> Result<Manifest, PackError> {
    put_in_place_stopping(target, staged, None)
}

/// Takes back an update's swap that was stopped part-way (its marker is there and the installed manifest is still the
/// earlier one): each new file goes back to `staged` (when `staged` has its manifest), each earlier file comes back
/// from `.previous`, the exe by one replacing rename, and the swap's own files in `.previous` (its marker and copies)
/// go. The earlier version is then installed as it was; the version kept before the update is not, since the swap
/// had cleared it. Nothing a person put anywhere moves. For a caller holding the swap lock.
pub fn undo_stopped(target: &Path, staged: &Path, _lock: &SwapLock) -> Result<Manifest, PackError> {
    let Some(old) = installed_manifest(target)? else {
        return Err(PackError::Refused(format!(
            "No version of BALLFIGHT Studio is installed in {}, so nothing was put back.",
            target.display()
        )));
    };
    if swap_state(target) != SwapState::Stopped {
        return Err(PackError::Refused(
            "No update is part-way in place, so there is nothing to put back.".to_string(),
        ));
    }
    let prev = target.join(".previous");
    let whole = |f: &Path, sha: &str| !is_link(f) && f.is_file() && sha256_file(f).is_ok_and(|now| now == sha);
    // The new files back where they were staged (only those the staged manifest names, with its bytes).
    if let Ok(new) = read_manifest(&staged.join(MANIFEST_NAME)) {
        for (rel, _, sha) in &new.files {
            let placed = join_relative(target, rel);
            let back = join_relative(staged, rel);
            // A file of the same bytes in both versions that never moved is the earlier one: it stays.
            let same_never_moved =
                old.find(rel).is_some_and(|(_, _, s)| s == sha) && !whole(&join_relative(&prev, rel), sha);
            if !whole(&placed, sha) || same_never_moved || fs::symlink_metadata(&back).is_ok() {
                continue;
            }
            if let Some(dir) = back.parent() {
                fs::create_dir_all(dir).map_err(|e| io_error("make the folder", dir, &e))?;
            }
            if is_the_exe(rel) {
                // The place keeps an exe: the new one is linked back first, the earlier one replaces it below.
                link_or_copy(&placed, &back, Some(sha)).map_err(|e| io_error("put back", &placed, &e))?;
            } else {
                fs::rename(&placed, &back).map_err(|e| io_error("put back", &placed, &e))?;
            }
        }
    }
    // The earlier files back from .previous.
    for (rel, _, sha) in &old.files {
        let here = join_relative(target, rel);
        let kept = join_relative(&prev, rel);
        if whole(&here, sha) || !whole(&kept, sha) || path_has_link(target, rel) {
            continue;
        }
        if let Some(dir) = here.parent() {
            fs::create_dir_all(dir).map_err(|e| io_error("make the folder", dir, &e))?;
        }
        if is_the_exe(rel) {
            replace_exe(&kept, &here, &prev, Some(sha)).map_err(|e| io_error("put back", &here, &e))?;
        } else {
            fs::rename(&kept, &here).map_err(|e| io_error("put back", &here, &e))?;
        }
    }
    // The swap's own files in .previous go; the marker last.
    for (rel, _, sha) in &old.files {
        let kept = join_relative(&prev, rel);
        if is_the_exe(rel) && whole(&kept, sha) && whole(&join_relative(target, rel), sha) {
            fs::remove_file(&kept).map_err(|e| io_error("remove", &kept, &e))?;
        }
    }
    for name in [MANIFEST_NAME, CONFIG_NAME, COPY_PARTS[0], COPY_PARTS[1]] {
        let f = prev.join(name);
        if !is_link(&f) && f.is_file() {
            fs::remove_file(&f).map_err(|e| io_error("remove", &f, &e))?;
        }
    }
    let marker = prev.join(SWAP_MARKER);
    fs::remove_file(&marker).map_err(|e| io_error("remove", &marker, &e))?;
    if let Ok(entries) = crate::package::walk(&prev) {
        for (rel, kind) in entries.iter().rev() {
            if *kind == crate::package::Kind::Dir {
                let _ = fs::remove_dir(join_relative(&prev, rel));
            }
        }
    }
    let _ = fs::remove_dir(&prev);
    Ok(old)
}

/// `put_in_place`, which for a test stops after `stop_after` steps of the swap and leaves everything as it is then.
pub(crate) fn put_in_place_stopping(
    target: &Path,
    staged: &Path,
    stop_after: Option<usize>,
) -> Result<Manifest, PackError> {
    let Some(old) = installed_manifest(target)? else {
        return Err(PackError::Refused(format!(
            "No version of BALLFIGHT Studio is installed in {}, so nothing was put in place.",
            target.display()
        )));
    };
    match swap_state(target) {
        SwapState::Committed => {
            // Stopped after its last real step: only the marker (and perhaps empty folders) are left.
            let marker = target.join(".previous").join(SWAP_MARKER);
            fs::remove_file(&marker).map_err(|e| io_error("remove", &marker, &e))?;
            let _ = clear_leftover_staging(staged, Some(&old));
            Ok(old)
        }
        SwapState::Stopped => {
            // Some files may already have moved: each new one must be in `staged` or in place, with its bytes.
            let new = read_manifest(&staged.join(MANIFEST_NAME))?;
            for (rel, _, sha) in &new.files {
                let whole = |root: &Path| {
                    let f = join_relative(root, rel);
                    !is_link(&f) && f.is_file() && sha256_file(&f).is_ok_and(|now| &now == sha)
                };
                if !whole(staged) && !whole(target) {
                    return Err(PackError::Refused(format!(
                        "{rel} of version {} is missing from {} and from the program folder, so the update cannot be \
                         finished.",
                        new.version,
                        staged.display()
                    )));
                }
            }
            refuse_running_old(target, &old, &new)?;
            swap_in(target, &old, &new, staged, stop_after, AGAIN_UPDATE)?;
            Ok(new)
        }
        SwapState::Done => {
            if is_link(staged) || !staged.is_dir() {
                return Err(PackError::Refused(format!(
                    "{} is not a folder, so nothing was put in place.",
                    staged.display()
                )));
            }
            // A copy of `update.toml` kept with an earlier version (when it comes back by "Go back") is ours by its
            // text, and the one in place stays, so it goes.
            let copy = staged.join(CONFIG_NAME);
            if fs::symlink_metadata(&copy).is_ok() {
                if is_link(&copy) || crate::config::read_config(&copy).is_err() {
                    return Err(PackError::Refused(format!(
                        "{} is not an update.toml that BALLFIGHT Studio wrote, so nothing was put in place.",
                        copy.display()
                    )));
                }
                fs::remove_file(&copy).map_err(|e| io_error("remove", &copy, &e))?;
            }
            let new = check_package(staged)?;
            refuse_running_old(target, &old, &new)?;
            check_conflicts(target, &old, &new)?;
            let prev = target.join(".previous");
            let cleared_any = fs::symlink_metadata(&prev).is_ok();
            if cleared_any {
                let r = plan_clear(&prev)?;
                if let Some((p, why)) = crate::remove::remove_now(&r).first() {
                    return Err(PackError::Io(format!(
                        "A file of the copy kept from before, {}, could not be cleared away ({why}), so nothing was \
                         put in place; version {} is still installed, and the copy kept from before it may be \
                         incomplete.",
                        p.display(),
                        old.version
                    )));
                }
            }
            swap_in(target, &old, &new, staged, stop_after, AGAIN_UPDATE).map_err(|e| match e {
                PackError::Io(text) if cleared_any && text != STOPPED => {
                    PackError::Io(format!("{text} The copy kept from the version before it was cleared."))
                }
                other => other,
            })?;
            Ok(new)
        }
    }
}

/// `refuse_running` for an update: a program of the installed version that is still running stops it, unless the
/// file at that place already holds the new version's bytes (the new version, finishing its own swap).
fn refuse_running_old(target: &Path, old: &Manifest, new: &Manifest) -> Result<(), PackError> {
    let running = old
        .files
        .iter()
        .filter(|(p, _, _)| p.to_ascii_lowercase().ends_with(".exe"))
        .any(|(p, _, _)| {
            let f = join_relative(target, p);
            let replaced = new
                .find(p)
                .is_some_and(|(_, _, sha)| sha256_file(&f).is_ok_and(|now| &now == sha));
            in_use(&f) && !replaced
        });
    if running {
        Err(PackError::Refused(CLOSE_FIRST.to_string()))
    } else {
        Ok(())
    }
}

/// Makes the two shortcuts (Start menu and desktop) to the installed exe, with its icon and its folder as the
/// working folder. An existing shortcut of that name is replaced only when it already opens this exe; one that opens
/// something else is left as it is, and that is an error in words.
pub fn shortcuts(layout: &Layout) -> Result<(), PackError> {
    check_layout(layout)?;
    let bytes = lnk::bytes(&layout.exe(), &layout.programs, &layout.icon(), APP_NAME)?;
    let mut problems = Vec::new();
    for link in [layout.start_menu_link(), layout.desktop_link()] {
        let Some(dir) = link.parent() else { continue };
        if fs::symlink_metadata(&link).is_ok() {
            if !opens_the_app(&link, layout) {
                problems.push(format!(
                    "A shortcut named BALLFIGHT Studio is already in {} and opens something else, so it was left as \
                     it is.",
                    dir.display()
                ));
                continue;
            }
            if fs::read(&link).is_ok_and(|b| b == bytes) {
                continue;
            }
        }
        let written = fs::create_dir_all(dir)
            .map_err(|e| io_error("make the folder", dir, &e))
            .and_then(|()| {
                let part = dir.join(format!(".{LINK_NAME}.part"));
                let w = fs::write(&part, &bytes)
                    .map_err(|e| io_error("write", &part, &e))
                    .and_then(|()| fs::rename(&part, &link).map_err(|e| io_error("rename", &part, &e)));
                if w.is_err() {
                    let _ = fs::remove_file(&part);
                }
                w
            });
        if let Err(e) = written {
            problems.push(e.to_string());
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(PackError::Refused(problems.join(" ")))
    }
}

/// Makes the shortcuts after a finished install and says what happened, in words: never an error, since the install
/// itself has worked by then (a shortcut of that name that opens something else is a note).
pub fn shortcuts_said(layout: &Layout) -> String {
    match shortcuts(layout) {
        Ok(()) => format!(
            "Made the shortcuts {} and {}.",
            layout.start_menu_link().display(),
            layout.desktop_link().display()
        ),
        Err(note) => note.to_string(),
    }
}

/// Whether `p` is a full path on a local drive letter (`C:\...`), read from its text alone: no `\\host\share`, no
/// `\\?\` or `\\.\` form, nothing relative.
pub(crate) fn on_a_drive_letter(p: &Path) -> bool {
    let Some(text) = p.to_str() else { return false };
    let b = text.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\'
}

/// Whether the file at `link` is a shortcut that opens this layout's installed exe. A shortcut whose target is not a
/// path on a local drive letter is not ours, decided from its bytes before any lookup, so a planted shortcut to
/// `\\host\share\...` never makes install or removal contact that host.
pub fn opens_the_app(link: &Path, layout: &Layout) -> bool {
    if is_link(link) || !link.is_file() {
        return false;
    }
    let Ok(bytes) = fs::read(link) else {
        return false;
    };
    let Some(dir) = link.parent() else { return false };
    lnk::target_at(&bytes, dir)
        .filter(|t| on_a_drive_letter(t))
        .is_some_and(|t| names(&t).is_some() && names(&t) == names(&layout.exe()))
}

/// The line printed before the source is copied.
pub const COPYING_SOURCE: &str =
    "Copying the source from GitHub; this can take a few minutes. A GitHub sign-in window may open.";

/// The longest the copy of the source may take before it is stopped.
const CLONE_LIMIT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// The app's own copy of the repository for updates (route A), and the `update.toml` that records it. Reads the
/// owner's clone's `origin` (`git -C <from> remote get-url origin`), refuses an address with a user name or
/// password without repeating it, then (after `say(COPYING_SOURCE)`) clones it with `--no-checkout` into
/// `<data>\source` through `run::run` with a 30-minute limit, unless a finished copy of the same repository is
/// already there (kept). A folder there that is a copy of another repository, or not a finished copy, is refused in
/// words and never deleted. Git is only ever pointed at a folder that holds its own `.git`, so it never walks up to
/// a repository around it. Git handles its own sign-in.
///
/// Only an https or ssh address is taken (`check_remote`), and the clone may use only those two protocols.
pub fn source_copy(git: &Path, from: &Path, data: &Path, say: &dyn Fn(&str)) -> Result<UpdateConfig, PackError> {
    source_copy_with(git, from, data, say, false)
}

/// `source_copy`; `local` (tests only, through their own calls: no command or app path passes `true`) also takes a
/// local repository and lets the clone read it.
pub(crate) fn source_copy_with(
    git: &Path,
    from: &Path,
    data: &Path,
    say: &dyn Fn(&str),
    local: bool,
) -> Result<UpdateConfig, PackError> {
    let o = OsStr::new;
    let no_clone = |dir: &Path| {
        PackError::Refused(format!(
            "Git could not read where the copy of the source at {} comes from, so updates could not be set up.",
            dir.display()
        ))
    };
    if !has_own_git(from) {
        return Err(no_clone(from));
    }
    let remote = git_says(
        git,
        from,
        &[o("-C"), from.as_os_str(), o("remote"), o("get-url"), o("origin")],
        data,
    )
    .map_err(|_| no_clone(from))?;
    if local {
        crate::config::check_address(&remote)?;
    } else {
        check_remote(&remote)?;
    }
    let source = data.join("source");
    let unfinished = || {
        PackError::Refused(format!(
            "{} is not a finished copy of the source; delete it, then install again.",
            source.display()
        ))
    };
    if fs::symlink_metadata(&source).is_ok() {
        if is_link(&source) || !source.is_dir() || !has_own_git(&source) {
            return Err(unfinished());
        }
        let theirs = git_says(
            git,
            &source,
            &[o("-C"), source.as_os_str(), o("remote"), o("get-url"), o("origin")],
            data,
        );
        match theirs {
            Ok(t) if t == remote => {}
            Ok(_) => {
                return Err(PackError::Refused(format!(
                    "{} is a copy of another repository, so it was left as it is and updates were not set up. Move \
                     it elsewhere, then install again.",
                    source.display()
                )));
            }
            Err(()) => return Err(unfinished()),
        }
        // A finished clone has the remote's branches; one stopped part-way may have none.
        let refs = git_says(
            git,
            &source,
            &[
                o("-C"),
                source.as_os_str(),
                o("for-each-ref"),
                o("--count=1"),
                o("refs/remotes/origin"),
            ],
            data,
        );
        if !refs.is_ok_and(|r| !r.is_empty()) {
            return Err(unfinished());
        }
    } else {
        fs::create_dir_all(data).map_err(|e| io_error("make the folder", data, &e))?;
        say(COPYING_SOURCE);
        // `--` ends the options, so the address can never be read as one.
        let never = std::sync::atomic::AtomicBool::new(false);
        let args = [
            o("clone"),
            o("--no-checkout"),
            o("--quiet"),
            o("--"),
            o(&remote),
            source.as_os_str(),
        ];
        let hooks = no_hooks(data);
        let how = crate::run::GitRun {
            dir: data,
            no_hooks: &hooks,
            local,
            env: &[],
            limit: CLONE_LIMIT,
            cancel: &never,
        };
        let ran = crate::run::git(git, &args, &how);
        let failed = match ran {
            Ok(r) if r.status.success() => None,
            Ok(r) => Some(r.tail.last().cloned().unwrap_or_else(|| "no reason given".to_string())),
            Err(e) => Some(e),
        };
        if let Some(said) = failed {
            let mut text = format!(
                "Git could not copy the repository into {}, so updates could not be set up. Git said \"{}\".",
                source.display(),
                said.trim().trim_end_matches('.')
            );
            if fs::symlink_metadata(&source).is_ok() {
                text.push(' ');
                text.push_str(&unfinished().to_string());
            }
            return Err(PackError::Io(text));
        }
    }
    Ok(UpdateConfig {
        source,
        remote,
        tag_prefix: TAG_PREFIX.to_string(),
        build_dir: data.join("build"),
        downloads: data.join("downloads"),
    })
}

/// Whether `dir` holds its own `.git` (a folder or, for a worktree, a file), not reached through a link.
fn has_own_git(dir: &Path) -> bool {
    let g = dir.join(".git");
    !is_link(dir) && !is_link(&g) && fs::symlink_metadata(&g).is_ok()
}

/// The folder the install's git runs are told holds their hooks: `<data>\build\no-hooks`, the same empty folder the
/// update engine uses (a missing one means no hooks as well).
fn no_hooks(data: &Path) -> PathBuf {
    data.join("build").join("no-hooks")
}

/// What git printed, trimmed, when it ended well; run through `run::git`, the one way git is started.
fn git_says(git: &Path, dir: &Path, args: &[&OsStr], data: &Path) -> Result<String, ()> {
    let never = std::sync::atomic::AtomicBool::new(false);
    let hooks = no_hooks(data);
    let how = crate::run::GitRun {
        dir,
        no_hooks: &hooks,
        local: false,
        env: &[],
        limit: std::time::Duration::from_secs(120),
        cancel: &never,
    };
    let out = crate::run::git(git, args, &how).map_err(|_| ())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(())
    }
}

/// The protocols git may use to reach an update's repository: https and ssh only, and TLS checked.
pub const PROTOCOLS: [&str; 8] = crate::run::GIT_PROTOCOLS;
/// Git on this PC: the first `git.exe` on `PATH`, else `%ProgramFiles%\Git\cmd\git.exe`.
pub fn find_git() -> Option<PathBuf> {
    let on_path = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join("git.exe"))
            .find(|g| g.is_absolute() && g.is_file())
    });
    on_path.or_else(|| {
        let g = PathBuf::from(std::env::var_os("ProgramFiles")?)
            .join("Git")
            .join("cmd")
            .join("git.exe");
        g.is_file().then_some(g)
    })
}

/// Test only (reading links back through the shell object): Windows PowerShell,
/// `<drive>:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`, from `SystemRoot` checked as for curl (a
/// drive's `Windows` folder); never from `PATH`.
#[cfg(test)]
pub(crate) fn powershell() -> Result<PathBuf, PackError> {
    let root = std::env::var_os("SystemRoot")
        .ok_or_else(|| PackError::Refused("Windows' folder is not known (SystemRoot is not set).".to_string()))?;
    let text = root.to_string_lossy();
    let b = text.as_bytes();
    let drive_windows =
        b.len() == 10 && b[0].is_ascii_alphabetic() && b[1..3] == *b":\\" && text[3..].eq_ignore_ascii_case("Windows");
    if !drive_windows {
        return Err(PackError::Refused(format!(
            "Windows' folder is given as {text}, which is not a drive's Windows folder, so PowerShell is not used \
             from it."
        )));
    }
    let ps = PathBuf::from(&root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if ps.is_file() {
        Ok(ps)
    } else {
        Err(PackError::Refused(format!(
            "Windows PowerShell is missing from {}.",
            ps.display()
        )))
    }
}

/// `text` for PowerShell's `-EncodedCommand`: UTF-16 little-endian, in base64. A fixed script passed this way needs
/// no quoting at all, so nothing in it can be read as another argument.
#[cfg(test)]
pub(crate) fn encoded_command(text: &str) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(ABC[(n >> 18) as usize & 63] as char);
        out.push(ABC[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ABC[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ABC[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// A minimal writer and reader of Windows shell links (`.lnk`), after Microsoft's published format MS-SHLLINK.
///
/// Why a writer of our own: the alternatives are a PowerShell or `WScript` command (a script handed paths) or the
/// shell's `IShellLinkW` and `IPersistFile` COM interfaces, which `windows-sys` does not describe, so they would need
/// hand-written interface tables and a dozen unsafe calls through them. A link is a small binary record: this writer
/// is plain safe code, and tests read every link back through Windows' own shell object.
///
/// What it writes: the 76-byte header (the ShellLink class id; flags `HasLinkTargetIDList | HasLinkInfo | HasName |
/// HasWorkingDir | HasIconLocation | IsUnicode`; show normally); the target's item id list, which Windows itself makes
/// for the path (`SHParseDisplayName`, the one place with unsafe code, in `id_list`); a LinkInfo with a VolumeID (a
/// fixed drive, no label) and the target's full path (ANSI and Unicode); the description, working folder and icon
/// file as Unicode strings; the terminal block. The item id list is what the shell reads a link's target from
/// without searching; the LinkInfo path is what it falls back on, and what `target` reads.
pub mod lnk {
    use std::path::{Path, PathBuf};

    use crate::PackError;

    /// The header's size and the ShellLink class id {00021401-0000-0000-C000-000000000046}.
    const HEADER_SIZE: u32 = 0x4C;
    const CLSID: [u8; 16] = [
        0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
    ];
    const HAS_ID_LIST: u32 = 0x1;
    const HAS_LINK_INFO: u32 = 0x2;
    const HAS_NAME: u32 = 0x4;
    const HAS_WORKING_DIR: u32 = 0x10;
    const HAS_ICON_LOCATION: u32 = 0x40;
    const IS_UNICODE: u32 = 0x80;
    /// The longest path written (Windows' classic MAX_PATH less its end mark).
    const MAX_PATH_CHARS: usize = 259;

    /// The bytes of a link to `target` (which must exist), starting in `working_dir`, showing the first icon in
    /// `icon`, with `description` as its tooltip. Each path must be a full path with a drive letter and at most 259
    /// characters.
    pub fn bytes(target: &Path, working_dir: &Path, icon: &Path, description: &str) -> Result<Vec<u8>, PackError> {
        let ids = id_list(&full_path(target)?)?;
        bytes_with(Some(&ids), target, working_dir, icon, description)
    }

    /// `bytes` with a given item id list (or none: then the shell finds the target from the LinkInfo path when it
    /// opens the link). Plain code, for tests of the format.
    pub fn bytes_with(
        ids: Option<&[u8]>,
        target: &Path,
        working_dir: &Path,
        icon: &Path,
        description: &str,
    ) -> Result<Vec<u8>, PackError> {
        let target = full_path(target)?;
        let working_dir = full_path(working_dir)?;
        let icon = full_path(icon)?;
        if description.encode_utf16().count() > 1024 || description.chars().any(char::is_control) {
            return Err(PackError::Refused("A shortcut's description is too long.".to_string()));
        }
        let mut out = Vec::with_capacity(1024);
        // ShellLinkHeader (76 bytes).
        put32(&mut out, HEADER_SIZE);
        out.extend_from_slice(&CLSID);
        let ids_flag = if ids.is_some() { HAS_ID_LIST } else { 0 };
        put32(
            &mut out,
            ids_flag | HAS_LINK_INFO | HAS_NAME | HAS_WORKING_DIR | HAS_ICON_LOCATION | IS_UNICODE,
        );
        put32(&mut out, 0x20); // FILE_ATTRIBUTE_ARCHIVE, as for any program file.
        out.extend_from_slice(&[0; 24]); // Creation, access and write times: not recorded.
        put32(&mut out, 0); // The target's size: not recorded.
        put32(&mut out, 0); // Icon index: the first icon in the icon file.
        put32(&mut out, 1); // SW_SHOWNORMAL.
        out.extend_from_slice(&[0; 2]); // No hot key.
        out.extend_from_slice(&[0; 10]); // Reserved.
        debug_assert_eq!(out.len(), HEADER_SIZE as usize);

        // LinkTargetIDList: its size, then the list as Windows made it (ending in its two zero bytes).
        if let Some(ids) = ids {
            let ends_right = ids.len() >= 2 && ids[ids.len() - 2..] == [0, 0];
            let Ok(size) = u16::try_from(ids.len()) else {
                return Err(PackError::Refused("A shortcut's target list is too long.".to_string()));
            };
            if !ends_right {
                return Err(PackError::Refused(
                    "A shortcut's target list is not complete.".to_string(),
                ));
            }
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(ids);
        }

        // LinkInfo: a header of 0x24 bytes (so the Unicode offsets are there), a VolumeID, the path.
        let mut volume = Vec::new();
        put32(&mut volume, 17); // VolumeIDSize: these 16 bytes and an empty label.
        put32(&mut volume, 3); // DRIVE_FIXED.
        put32(&mut volume, 0); // No serial number.
        put32(&mut volume, 16); // The label starts right after these four numbers...
        volume.push(0); // ...and is empty.
        // The ANSI path: a character outside ASCII is written as '?', which no path can hold, so the ANSI copy can
        // never name another file; Windows reads the Unicode copy.
        let mut ansi: Vec<u8> = target
            .chars()
            .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
            .collect();
        ansi.push(0);
        let unicode = utf16_ended(&target);
        let header = 0x24u32;
        let volume_at = header;
        let base_at = volume_at + volume.len() as u32;
        let suffix_at = base_at + ansi.len() as u32;
        let base_unicode_at = suffix_at + 1;
        let suffix_unicode_at = base_unicode_at + unicode.len() as u32;
        let size = suffix_unicode_at + 2;
        for n in [
            size,
            header,
            1,
            volume_at,
            base_at,
            0,
            suffix_at,
            base_unicode_at,
            suffix_unicode_at,
        ] {
            put32(&mut out, n); // flags 1: VolumeIDAndLocalBasePath; 0: no network part.
        }
        out.extend_from_slice(&volume);
        out.extend_from_slice(&ansi);
        out.push(0); // The common path suffix, ANSI: empty.
        out.extend_from_slice(&unicode);
        out.extend_from_slice(&[0, 0]); // The common path suffix, Unicode: empty.

        // StringData, in the format's order: the description, the working folder, the icon file.
        for s in [description, working_dir.as_str(), icon.as_str()] {
            let units: Vec<u16> = s.encode_utf16().collect();
            out.extend_from_slice(&(units.len() as u16).to_le_bytes());
            for u in units {
                out.extend_from_slice(&u.to_le_bytes());
            }
        }
        put32(&mut out, 0); // TerminalBlock: no extra data.
        Ok(out)
    }

    /// The full path a link's LinkInfo names, or `None` when the bytes are not a link this reader understands.
    pub fn target(bytes: &[u8]) -> Option<PathBuf> {
        if get32(bytes, 0)? != HEADER_SIZE || bytes.get(4..20)? != CLSID {
            return None;
        }
        let flags = get32(bytes, 20)?;
        let mut at = HEADER_SIZE as usize;
        if flags & HAS_ID_LIST != 0 {
            at = at.checked_add(2 + usize::from(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?)))?;
        }
        if flags & HAS_LINK_INFO == 0 {
            return None;
        }
        let size = get32(bytes, at)? as usize;
        let info = bytes.get(at..at.checked_add(size)?)?;
        let header = get32(info, 4)?;
        if size < 0x1C || get32(info, 8)? & 1 == 0 {
            return None;
        }
        let path = if header >= 0x24 && get32(info, 28)? != 0 {
            let base = utf16_at(info, get32(info, 28)? as usize)?;
            let suffix = match get32(info, 32)? {
                0 => String::new(),
                n => utf16_at(info, n as usize)?,
            };
            base + &suffix
        } else {
            ascii_at(info, get32(info, 16)? as usize)? + &ascii_at(info, get32(info, 24)? as usize)?
        };
        (!path.is_empty()).then(|| PathBuf::from(path))
    }

    /// The target of the link file in `link_dir` with these bytes: when the link has a LinkInfo section, its local path
    /// (or `None`, as for a network link); only when it has none, its relative path (which Windows writes when it
    /// leaves the LinkInfo out, as it can for a path given partly in 8.3 short form) taken from `link_dir`.
    pub fn target_at(bytes: &[u8], link_dir: &Path) -> Option<PathBuf> {
        let flags = get32(bytes, 20)?;
        // A link with a LinkInfo is read from it alone (one without a local path, such as a network link, gives
        // nothing); the relative path is trusted only when the link has nothing else.
        if flags & HAS_LINK_INFO != 0 {
            return target(bytes);
        }
        let relative = string_data(bytes, flags)?.relative?;
        if relative.is_empty() {
            return None;
        }
        // `..` taken away name by name; one that would climb above the drive gives nothing.
        let mut out = PathBuf::new();
        for c in link_dir.join(relative).components() {
            match c {
                std::path::Component::ParentDir => {
                    if out.file_name().is_none() || !out.pop() {
                        return None;
                    }
                }
                std::path::Component::CurDir => {}
                other => out.push(other.as_os_str()),
            }
        }
        Some(out)
    }

    /// The strings a link holds after its item id list and LinkInfo.
    struct StringData {
        relative: Option<String>,
    }

    fn string_data(bytes: &[u8], flags: u32) -> Option<StringData> {
        const HAS_RELATIVE_PATH: u32 = 0x8;
        if get32(bytes, 0)? != HEADER_SIZE || bytes.get(4..20)? != CLSID {
            return None;
        }
        let mut at = HEADER_SIZE as usize;
        if flags & HAS_ID_LIST != 0 {
            at = at.checked_add(2 + usize::from(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?)))?;
        }
        if flags & HAS_LINK_INFO != 0 {
            at = at.checked_add(get32(bytes, at)? as usize)?;
        }
        let unicode = flags & IS_UNICODE != 0;
        let mut read = |present: bool| -> Option<Option<String>> {
            if !present {
                return Some(None);
            }
            let count = usize::from(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?));
            at += 2;
            let text = if unicode {
                let raw = bytes.get(at..at.checked_add(count * 2)?)?;
                at += count * 2;
                let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
                String::from_utf16(&units).ok()?
            } else {
                let raw = bytes.get(at..at.checked_add(count)?)?;
                at += count;
                raw.is_ascii().then(|| String::from_utf8_lossy(raw).into_owned())?
            };
            Some(Some(text))
        };
        read(flags & HAS_NAME != 0)?;
        let relative = read(flags & HAS_RELATIVE_PATH != 0)?;
        Some(StringData { relative })
    }

    /// Windows' item id list for the existing file at `path` (a full path), made by the shell itself.
    ///
    /// It runs on a thread of its own, which sets up COM for itself (single-threaded, as the shell expects) and takes
    /// it down again, so the calling thread's COM state is never touched.
    #[cfg(windows)]
    fn id_list(path: &str) -> Result<Vec<u8>, PackError> {
        use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
        use windows_sys::Win32::UI::Shell::Common::ITEMIDLIST;
        use windows_sys::Win32::UI::Shell::{ILFree, ILGetSize, SHParseDisplayName};

        // One at a time: making shortcuts is rare, and the shell is then never asked from two threads at once.
        static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
        let made = std::thread::scope(|s| {
            s.spawn(|| {
                // SAFETY: the reserved pointer must be null, and is; this thread is new and does nothing else with
                // COM. It is taken down below only when this call succeeded, as the pairing rule asks.
                let init = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
                let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
                let mut attrs = 0u32;
                // SAFETY: `wide` is a UTF-16 string ending in 0 that lives across the call; no bind context is
                // passed (null is allowed); `pidl` and `attrs` are valid places for the two answers.
                let parsed =
                    unsafe { SHParseDisplayName(wide.as_ptr(), std::ptr::null_mut(), &mut pidl, 0, &mut attrs) };
                let bytes = if parsed >= 0 && !pidl.is_null() {
                    // SAFETY: `pidl` is the list Windows just gave; ILGetSize reads only inside it and counts its
                    // end mark.
                    let size = unsafe { ILGetSize(pidl) } as usize;
                    // SAFETY: Windows says the list at `pidl` is `size` bytes long; it is copied out before it is
                    // freed.
                    let copy = unsafe { std::slice::from_raw_parts(pidl.cast::<u8>(), size) }.to_vec();
                    // SAFETY: `pidl` came from SHParseDisplayName, which hands it to the caller, and is freed once.
                    unsafe { ILFree(pidl) };
                    Some(copy)
                } else {
                    None
                };
                if init >= 0 {
                    // SAFETY: pairs the successful CoInitializeEx above, on the same thread.
                    unsafe { CoUninitialize() };
                }
                bytes
            })
            .join()
            .ok()
            .flatten()
        });
        made.ok_or_else(|| {
            PackError::Refused(format!(
                "Windows could not find {path} to make a shortcut to it, so no shortcut was made."
            ))
        })
    }

    #[cfg(not(windows))]
    fn id_list(_path: &str) -> Result<Vec<u8>, PackError> {
        Err(PackError::Refused("Shortcuts are made only on Windows.".to_string()))
    }

    /// A full path with a drive letter, of at most 259 characters, as text.
    fn full_path(p: &Path) -> Result<String, PackError> {
        let refuse = || {
            Err(PackError::Refused(format!(
                "A shortcut can only lead to a full path on a drive of at most {MAX_PATH_CHARS} characters, and {} \
                 is not one.",
                p.display()
            )))
        };
        let Some(s) = p.to_str() else { return refuse() };
        let b = s.as_bytes();
        let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\';
        if !drive || s.contains('/') || s.encode_utf16().count() > MAX_PATH_CHARS || s.chars().any(char::is_control) {
            return refuse();
        }
        Ok(s.to_string())
    }

    fn put32(out: &mut Vec<u8>, n: u32) {
        out.extend_from_slice(&n.to_le_bytes());
    }

    fn get32(b: &[u8], at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(b.get(at..at.checked_add(4)?)?.try_into().ok()?))
    }

    /// UTF-16 little-endian with its end mark.
    fn utf16_ended(s: &str) -> Vec<u8> {
        s.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
    }

    /// The UTF-16 text starting at `at`, up to its end mark.
    fn utf16_at(b: &[u8], at: usize) -> Option<String> {
        let mut units = Vec::new();
        let mut i = at;
        loop {
            let u = u16::from_le_bytes(b.get(i..i.checked_add(2)?)?.try_into().ok()?);
            if u == 0 {
                break;
            }
            units.push(u);
            i += 2;
        }
        String::from_utf16(&units).ok()
    }

    /// The ASCII text starting at `at`, up to its end mark; `None` for anything outside ASCII.
    fn ascii_at(b: &[u8], at: usize) -> Option<String> {
        let rest = b.get(at..)?;
        let end = rest.iter().position(|&c| c == 0)?;
        let text = &rest[..end];
        text.is_ascii().then(|| String::from_utf8_lossy(text).into_owned())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::package::tests::fake_package;
    use crate::testdir::TempDir;
    use std::io::Read;

    pub(crate) fn config_for(layout: &Layout) -> UpdateConfig {
        UpdateConfig {
            source: layout.data.join("source"),
            remote: "https://github.com/someone/InstrumentBattleMachine.git".to_string(),
            tag_prefix: TAG_PREFIX.to_string(),
            build_dir: layout.data.join("build"),
            downloads: layout.data.join("downloads"),
        }
    }

    pub(crate) fn names_in(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = match fs::read_dir(dir) {
            Ok(r) => r
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => Vec::new(),
        };
        v.sort();
        v
    }

    /// A package at `root\pkg-<version>` with the fake exe's bytes changed to name the version, so two versions
    /// differ in their exe.
    pub(crate) fn package_of(root: &Path, version: &str) -> PathBuf {
        let p = fake_package(root, &format!("pkg-{version}"), version);
        // Re-make it with an exe that names the version, through the real `package`.
        let inputs = root.join(format!("pkg-{version}-inputs"));
        let exe = inputs.join("built").join(EXE_NAME);
        fs::write(&exe, format!("pretend exe {version}")).unwrap();
        let again = root.join(format!("package-{version}"));
        crate::package::package(&exe, &inputs.join("tree"), &inputs.join("checked"), &again).unwrap();
        let _ = p;
        again
    }

    fn read(path: &Path) -> Vec<u8> {
        let mut v = Vec::new();
        fs::File::open(path).unwrap().read_to_end(&mut v).unwrap();
        v
    }

    #[test]
    fn install_into_a_fresh_folder() {
        let dir = TempDir::new("install-fresh");
        let pkg = package_of(dir.path(), "0.6.0");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&pkg, &layout, &config_for(&layout)).unwrap();
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        assert_eq!(
            names_in(&layout.programs),
            [MANIFEST_NAME, EXE_NAME, ICON_NAME, "ffmpeg", "guide", "update.toml"]
        );
        assert_eq!(
            names_in(&layout.programs.join("ffmpeg")),
            ["ABOUT.txt", "LICENSE.txt", "ffmpeg.exe", "ffprobe.exe"]
        );
        assert_eq!(
            crate::config::read_config(&layout.programs.join(CONFIG_NAME)).unwrap(),
            config_for(&layout)
        );
        assert_eq!(
            read_manifest(&layout.programs.join(MANIFEST_NAME)).unwrap().version,
            "0.6.0"
        );
        assert!(!layout.staging().exists(), "no staging folder is left");
        // The package folder is left as it was.
        check_package(&pkg).unwrap();
        // Nothing was written outside the program folder: no shortcuts, no data.
        assert!(!layout.start_menu.exists() && !layout.desktop.exists() && !layout.data.exists());
    }

    #[test]
    fn install_over_an_old_one_keeps_it_as_previous() {
        let dir = TempDir::new("install-over");
        let old = package_of(dir.path(), "0.6.0");
        let new = package_of(dir.path(), "0.6.1");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&old, &layout, &config_for(&layout)).unwrap();
        // A person's own file in the program folder.
        fs::write(layout.programs.join("my notes.txt"), b"mine").unwrap();
        install(&new, &layout, &config_for(&layout)).unwrap();
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1");
        assert_eq!(
            read_manifest(&layout.programs.join(MANIFEST_NAME)).unwrap().version,
            "0.6.1"
        );
        let prev = layout.programs.join(".previous");
        assert_eq!(read(&prev.join(EXE_NAME)), b"pretend exe 0.6.0");
        assert_eq!(read_manifest(&prev.join(MANIFEST_NAME)).unwrap().version, "0.6.0");
        assert!(prev.join("ffmpeg").join("ffmpeg.exe").is_file());
        assert!(prev.join(CONFIG_NAME).is_file());
        assert_eq!(
            read(&layout.programs.join("my notes.txt")),
            b"mine",
            "a person's file stays where it was"
        );
        assert!(!prev.join("my notes.txt").exists());
        assert!(!layout.staging().exists());

        // A third install replaces .previous with 0.6.1 and keeps going.
        let third = package_of(dir.path(), "0.6.2");
        install(&third, &layout, &config_for(&layout)).unwrap();
        assert_eq!(read(&prev.join(EXE_NAME)), b"pretend exe 0.6.1");
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.2");
        assert_eq!(read(&layout.programs.join("my notes.txt")), b"mine");
    }

    #[test]
    fn install_refuses_while_the_app_runs() {
        let dir = TempDir::new("install-running");
        let old = package_of(dir.path(), "0.6.0");
        let new = package_of(dir.path(), "0.6.1");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&old, &layout, &config_for(&layout)).unwrap();
        // A real running program in the exe's place: Windows' own ping, copied there, pinging this PC for ~8 s.
        let ping = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE");
        fs::remove_file(layout.exe()).unwrap();
        fs::copy(&ping, layout.exe()).unwrap();
        let mut child = crate::run::quiet(
            &layout.exe(),
            &[OsStr::new("-n"), OsStr::new("9"), OsStr::new("127.0.0.1")],
        )
        .spawn()
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let before = read(&layout.exe());
        let err = install(&new, &layout, &config_for(&layout));
        let _ = child.kill();
        let _ = child.wait();
        let err = err.unwrap_err().to_string();
        assert_eq!(err, CLOSE_FIRST);
        assert_eq!(read(&layout.exe()), before, "the running copy is untouched");
        assert!(!layout.programs.join(".previous").exists());
        assert!(!layout.staging().exists());
    }

    #[test]
    fn a_persons_file_in_the_way_stops_the_install() {
        let dir = TempDir::new("install-way");
        let old = package_of(dir.path(), "0.6.0");
        let new = package_of(dir.path(), "0.6.1");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&old, &layout, &config_for(&layout)).unwrap();
        // 0.6.1 is given a new file whose name a person's own file already has.
        let manifest_before = fs::read(layout.programs.join(MANIFEST_NAME)).unwrap();
        fs::write(layout.programs.join("guide").join("extra.html"), b"a person's page").unwrap();
        let inputs = dir.path().join("pkg-0.6.1-inputs");
        fs::write(inputs.join("tree/studio/packaging/guide/extra.html"), b"the new page").unwrap();
        let new2 = dir.path().join("package-0.6.1-extra");
        crate::package::package(
            &inputs.join("built").join(EXE_NAME),
            &inputs.join("tree"),
            &inputs.join("checked"),
            &new2,
        )
        .unwrap();
        let _ = new;
        let err = install(&new2, &layout, &config_for(&layout)).unwrap_err().to_string();
        assert!(
            err.contains("extra.html") && err.contains("not put there by BALLFIGHT Studio"),
            "{err}"
        );
        assert_eq!(
            read(&layout.programs.join("guide").join("extra.html")),
            b"a person's page"
        );
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        assert_eq!(fs::read(layout.programs.join(MANIFEST_NAME)).unwrap(), manifest_before);
        assert!(!layout.programs.join(".previous").exists());
        assert!(!layout.staging().exists());
    }

    #[test]
    fn a_folder_that_is_not_ours_is_never_installed_over() {
        let dir = TempDir::new("install-foreign");
        let pkg = package_of(dir.path(), "0.6.0");
        let layout = Layout::under(&dir.path().join("pc"));
        fs::create_dir_all(&layout.programs).unwrap();
        fs::write(layout.programs.join("something.txt"), b"not ours").unwrap();
        let err = install(&pkg, &layout, &config_for(&layout)).unwrap_err().to_string();
        assert!(err.contains("was not made by"), "{err}");
        assert_eq!(names_in(&layout.programs), ["something.txt"]);
        // An empty folder of that name is fine.
        fs::remove_file(layout.programs.join("something.txt")).unwrap();
        install(&pkg, &layout, &config_for(&layout)).unwrap();
        assert!(layout.exe().is_file());
    }

    #[test]
    fn a_broken_package_installs_nothing() {
        let dir = TempDir::new("install-broken");
        let old = package_of(dir.path(), "0.6.0");
        let new = package_of(dir.path(), "0.6.1");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&old, &layout, &config_for(&layout)).unwrap();
        fs::write(new.join("ffmpeg").join("ffprobe.exe"), b"changed after packaging").unwrap();
        assert!(install(&new, &layout, &config_for(&layout)).is_err());
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        assert!(!layout.programs.join(".previous").exists());
        assert!(!layout.staging().exists(), "the staging folder is cleared again");
    }

    #[test]
    fn a_failed_swap_puts_everything_back() {
        let dir = TempDir::new("install-swapfail");
        let old = package_of(dir.path(), "0.6.0");
        let new = package_of(dir.path(), "0.6.1");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&old, &layout, &config_for(&layout)).unwrap();
        // The old guide page is held open without sharing delete, so it cannot be moved into .previous.
        let held_path = layout.programs.join("guide").join("index.html");
        let held = hold(&held_path);
        let err = install(&new, &layout, &config_for(&layout)).unwrap_err().to_string();
        drop(held);
        assert!(err.contains("0.6.0 is still installed"), "{err}");
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        assert_eq!(
            read_manifest(&layout.programs.join(MANIFEST_NAME)).unwrap().version,
            "0.6.0"
        );
        check_installed(&layout.programs);
        assert!(!layout.staging().exists());
        // With the file free again, the install works.
        install(&new, &layout, &config_for(&layout)).unwrap();
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1");
    }

    /// A package of `version` that also holds a guide page the earlier fake versions do not have.
    pub(crate) fn package_with_extra(root: &Path, version: &str) -> PathBuf {
        let inputs = root.join(format!("extra-{version}-inputs"));
        let (exe, tree, checked) = crate::package::tests::fake_inputs(&inputs, version);
        fs::write(&exe, format!("pretend exe {version}")).unwrap();
        fs::write(tree.join("studio/packaging/guide/extra.html"), b"a new page").unwrap();
        let out = root.join(format!("package-extra-{version}"));
        crate::package::package(&exe, &tree, &checked, &out).unwrap();
        out
    }

    /// Installs `package`, stopping after `steps` steps of the swap with nothing undone, as a power cut would.
    pub(crate) fn install_stopped_after(package: &Path, layout: &Layout, steps: usize) -> Result<(), PackError> {
        install_stopping(package, layout, &config_for(layout), Some(steps))
    }

    #[test]
    fn a_stopped_install_is_finished_by_installing_again() {
        let mut stopped_ever = false;
        let mut finished_first_time = false;
        for n in 0..=27 {
            let dir = TempDir::new("install-stopped");
            let old = package_of(dir.path(), "0.6.0");
            let new = package_with_extra(dir.path(), "0.6.1");
            let layout = Layout::under(&dir.path().join("pc"));
            install(&old, &layout, &config_for(&layout)).unwrap();
            fs::write(layout.programs.join("my notes.txt"), b"mine").unwrap();
            let stopped = match install_stopped_after(&new, &layout, n) {
                Err(PackError::Io(t)) if t == STOPPED => {
                    stopped_ever = true;
                    // Stopped anywhere, the folder is still one of ours, and removal could still plan it.
                    assert!(read_manifest(&layout.programs.join(MANIFEST_NAME)).is_ok(), "step {n}");
                    assert!(crate::remove::removal_plan(&layout).is_ok(), "step {n}");
                    // And the shortcut always finds a whole exe, the earlier one or the new one.
                    let exe = read(&layout.exe());
                    assert!(exe == b"pretend exe 0.6.0" || exe == b"pretend exe 0.6.1", "step {n}");
                    true
                }
                Ok(()) => {
                    finished_first_time = true;
                    false
                }
                Err(e) => panic!("step {n}: {e}"),
            };
            install(&new, &layout, &config_for(&layout)).unwrap_or_else(|e| panic!("step {n}: {e}"));
            assert_eq!(
                read_manifest(&layout.programs.join(MANIFEST_NAME)).unwrap().version,
                "0.6.1"
            );
            check_installed(&layout.programs);
            assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1", "step {n}");
            assert_eq!(read(&layout.programs.join("guide/extra.html")), b"a new page");
            assert_eq!(read(&layout.programs.join("my notes.txt")), b"mine", "step {n}");
            let prev = layout.programs.join(".previous");
            // The kept copy is a whole version: the earlier one when the first install stopped, else (installed
            // twice) 0.6.1 itself.
            let kept = if stopped { "0.6.0" } else { "0.6.1" };
            assert_eq!(
                read_manifest(&prev.join(MANIFEST_NAME)).unwrap().version,
                kept,
                "step {n}"
            );
            check_installed(&prev);
            assert_eq!(
                read(&prev.join(EXE_NAME)),
                format!("pretend exe {kept}").as_bytes(),
                "step {n}"
            );
            assert!(prev.join(CONFIG_NAME).is_file());
            assert!(!layout.staging().exists(), "step {n}");
            assert_eq!(
                names_in(&layout.programs),
                [
                    ".previous",
                    MANIFEST_NAME,
                    EXE_NAME,
                    ICON_NAME,
                    "ffmpeg",
                    "guide",
                    "my notes.txt",
                    "update.toml"
                ],
                "step {n}"
            );
        }
        assert!(
            stopped_ever && finished_first_time,
            "the steps covered both a stop and a whole swap"
        );
    }

    #[test]
    fn a_finished_reinstall_is_not_taken_for_a_stopped_one() {
        let dir = TempDir::new("install-twice");
        let layout = Layout::under(&dir.path().join("pc"));
        let v061 = package_of(dir.path(), "0.6.1");
        install(&v061, &layout, &config_for(&layout)).unwrap();
        install(&v061, &layout, &config_for(&layout)).unwrap();
        let prev = layout.programs.join(".previous");
        assert!(!prev.join(SWAP_MARKER).exists(), "a finished swap leaves no marker");
        // A person edits a listed file; 0.6.2 drops one file.
        fs::write(layout.programs.join("guide").join("index.html"), b"a person's edit").unwrap();
        let inputs = dir.path().join("pkg-0.6.2-inputs");
        let (exe, tree, checked) = crate::package::tests::fake_inputs(&inputs, "0.6.2");
        fs::write(&exe, b"pretend exe 0.6.2").unwrap();
        fs::remove_file(tree.join("studio/packaging/guide/pictures/start.png")).unwrap();
        fs::remove_dir(tree.join("studio/packaging/guide/pictures")).unwrap();
        let v062 = dir.path().join("package-0.6.2-dropped");
        crate::package::package(&exe, &tree, &checked, &v062).unwrap();
        install(&v062, &layout, &config_for(&layout)).unwrap();
        check_installed(&layout.programs);
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.2");
        assert!(
            !layout.programs.join("guide").join("pictures").exists(),
            "no orphan of the dropped file"
        );
        assert_eq!(
            read(&layout.programs.join("guide").join("index.html")),
            b"<p>The guide.</p>"
        );
        // The kept copy is 0.6.1, with the person's edit in it.
        assert_eq!(read_manifest(&prev.join(MANIFEST_NAME)).unwrap().version, "0.6.1");
        assert_eq!(read(&prev.join("guide").join("index.html")), b"a person's edit");
        assert_eq!(read(&prev.join(EXE_NAME)), b"pretend exe 0.6.1");
        assert!(prev.join("guide").join("pictures").join("start.png").is_file());
        assert!(!prev.join(SWAP_MARKER).exists());
    }

    #[test]
    fn a_stop_right_after_the_manifest_step_is_finished() {
        let dir = TempDir::new("install-afterlast");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&package_of(dir.path(), "0.6.0"), &layout, &config_for(&layout)).unwrap();
        let new = package_with_extra(dir.path(), "0.6.1");
        // 24 steps: the last is taking the marker out; stop just before it.
        assert!(install_stopped_after(&new, &layout, 23).is_err());
        let prev = layout.programs.join(".previous");
        assert!(prev.join(SWAP_MARKER).is_file());
        assert_eq!(
            read_manifest(&layout.programs.join(MANIFEST_NAME)).unwrap().version,
            "0.6.1"
        );
        assert!(layout.staging().is_dir() && !layout.staging().join(MANIFEST_NAME).exists());
        install(&new, &layout, &config_for(&layout)).unwrap();
        assert!(!prev.join(SWAP_MARKER).exists() && !layout.staging().exists());
        assert_eq!(read_manifest(&prev.join(MANIFEST_NAME)).unwrap().version, "0.6.0");
        check_installed(&layout.programs);
        check_installed(&prev);
    }

    #[test]
    fn an_unreadable_kept_manifest_is_named() {
        let dir = TempDir::new("install-badprev");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&package_of(dir.path(), "0.6.0"), &layout, &config_for(&layout)).unwrap();
        install(&package_of(dir.path(), "0.6.1"), &layout, &config_for(&layout)).unwrap();
        let kept = layout.programs.join(".previous").join(MANIFEST_NAME);
        fs::write(&kept, b"BALLFIGHT Studio package mani").unwrap();
        let err = install(&package_of(dir.path(), "0.6.2"), &layout, &config_for(&layout))
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            format!(
                "{} cannot be read, so nothing was installed. Delete that file, then install again.",
                kept.display()
            )
        );
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1");
    }

    #[test]
    fn a_folder_that_holds_only_empty_folders_counts_as_empty() {
        let dir = TempDir::new("install-emptyprev");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&package_of(dir.path(), "0.6.0"), &layout, &config_for(&layout)).unwrap();
        fs::create_dir_all(layout.programs.join(".next").join("ffmpeg").join("deeper")).unwrap();
        install(&package_of(dir.path(), "0.6.1"), &layout, &config_for(&layout)).unwrap();
        assert!(!layout.programs.join(".next").exists());
        // A file in it is not ours, and stops the install.
        fs::create_dir_all(layout.programs.join(".next")).unwrap();
        fs::write(layout.programs.join(".next").join("x.txt"), b"x").unwrap();
        let err = install(&package_of(dir.path(), "0.6.2"), &layout, &config_for(&layout)).unwrap_err();
        assert!(err.to_string().contains("did not put there"), "{err}");
    }

    #[test]
    fn a_link_anywhere_on_a_path_is_seen() {
        let dir = TempDir::new("install-pathlink");
        let root = dir.path().join("root");
        fs::create_dir_all(root.join("real")).unwrap();
        fs::write(root.join("real").join("a.txt"), b"a").unwrap();
        assert!(!path_has_link(&root, "real/a.txt"));
        if !make_junction(&root.join("real"), &root.join("linked")) {
            eprintln!("note: could not make a junction here, so the link case was not tried");
            return;
        }
        assert!(path_has_link(&root, "linked/a.txt"));
        assert!(path_has_link(&root, "linked"));
    }

    /// Every file the installed manifest lists is there with its hash.
    fn check_installed(dir: &Path) {
        let m = read_manifest(&dir.join(MANIFEST_NAME)).unwrap();
        for (p, bytes, sha) in &m.files {
            let f = join_relative(dir, p);
            assert_eq!(fs::metadata(&f).unwrap().len(), *bytes, "{p}");
            assert_eq!(&sha256_file(&f).unwrap(), sha, "{p}");
        }
    }

    /// Opens a file so that nobody can move or delete it until the handle is dropped.
    pub(crate) fn hold(path: &Path) -> fs::File {
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // FILE_SHARE_READ only: no FILE_SHARE_DELETE, so a rename or delete fails while this is open.
            fs::OpenOptions::new().read(true).share_mode(0x1).open(path).unwrap()
        }
        #[cfg(not(windows))]
        {
            fs::File::open(path).unwrap()
        }
    }

    #[test]
    fn layouts_that_are_not_ours_are_refused() {
        let dir = TempDir::new("install-layout");
        let good = Layout::under(dir.path());
        check_layout(&good).unwrap();
        let mut bad = good.clone();
        bad.programs = dir.path().join("Programs").join("Something Else");
        assert!(
            check_layout(&bad)
                .unwrap_err()
                .to_string()
                .contains("not named BALLFIGHT Studio")
        );
        let mut bad = good.clone();
        bad.programs = PathBuf::from(r"C:\");
        assert!(check_layout(&bad).is_err());
        let mut bad = good.clone();
        bad.programs = PathBuf::from(r"relative\BALLFIGHT Studio");
        assert!(check_layout(&bad).is_err());
        // The data folder's own parent (named BALLFIGHT Studio on a real PC), or a folder around the data folder.
        let mut bad = good.clone();
        bad.data = dir.path().join(APP_NAME).join("data");
        bad.programs = dir.path().join(APP_NAME);
        assert!(check_layout(&bad).is_err());
        let mut bad = good.clone();
        bad.data = good.programs.join("data");
        assert!(check_layout(&bad).is_err());
        let mut bad = good.clone();
        bad.desktop = good.programs.join("Desktop");
        assert!(check_layout(&bad).is_err());
        let mut bad = good.clone();
        bad.programs = good.data.join(APP_NAME);
        assert!(check_layout(&bad).is_err());
        // Around the user's own folder.
        if let Some(home) = std::env::var_os("USERPROFILE") {
            let mut bad = good.clone();
            bad.programs = PathBuf::from(home).join(APP_NAME);
            check_layout(&bad).unwrap();
            let home = PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
            if let Some(parent) = home.parent() {
                let mut bad = good.clone();
                bad.programs = parent.to_path_buf();
                assert!(check_layout(&bad).is_err());
            }
        }
    }

    #[test]
    fn a_program_folder_that_is_a_link_is_refused() {
        let dir = TempDir::new("install-link");
        let layout = Layout::under(dir.path());
        let elsewhere = dir.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        fs::create_dir_all(layout.programs.parent().unwrap()).unwrap();
        if !make_junction(&elsewhere, &layout.programs) {
            eprintln!("note: could not make a junction here, so the link refusal was not tried");
            return;
        }
        let err = check_layout(&layout).unwrap_err().to_string();
        assert!(err.contains("link"), "{err}");
        let pkg = package_of(dir.path(), "0.6.0");
        assert!(install(&pkg, &layout, &config_for(&layout)).is_err());
        assert!(names_in(&elsewhere).is_empty());
    }

    /// Makes `link` a directory junction to `to` (no admin rights needed) with `mklink /J`, through the one way to
    /// start a program. Test-only. Gives false when that is not possible here.
    pub(crate) fn make_junction(to: &Path, link: &Path) -> bool {
        let Some(root) = std::env::var_os("SystemRoot") else {
            return false;
        };
        let cmd = PathBuf::from(root).join("System32").join("cmd.exe");
        let never = std::sync::atomic::AtomicBool::new(false);
        let args = [
            OsStr::new("/d"),
            OsStr::new("/c"),
            OsStr::new("mklink"),
            OsStr::new("/J"),
            link.as_os_str(),
            to.as_os_str(),
        ];
        crate::run::run(&cmd, &args, std::time::Duration::from_secs(20), &never).is_ok_and(|r| r.status.success())
            && is_link(link)
    }

    /// What Windows' own shell object reads from a link: target, icon location and working folder, one per line.
    fn read_back(link: &Path) -> Vec<String> {
        // A fixed script; the link's path comes in an environment variable, never in the script.
        const SCRIPT: &str = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:BF_TEST_LINK)\n\
             [Console]::Out.WriteLine($s.TargetPath)\n\
             [Console]::Out.WriteLine($s.IconLocation)\n\
             [Console]::Out.WriteLine($s.WorkingDirectory)\n\
             [Console]::Out.WriteLine($s.Description)\n";
        let ps = powershell().unwrap();
        let enc = encoded_command(SCRIPT);
        let out = crate::run::quiet(
            &ps,
            &[
                OsStr::new("-NoLogo"),
                OsStr::new("-NoProfile"),
                OsStr::new("-NonInteractive"),
                OsStr::new("-EncodedCommand"),
                OsStr::new(&enc),
            ],
        )
        .env("BF_TEST_LINK", link)
        .output()
        .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn the_shortcuts_point_at_the_exe() {
        let dir = TempDir::new("install-links");
        let pkg = package_of(dir.path(), "0.6.0");
        let layout = Layout::under(&dir.path().join("pc"));
        install(&pkg, &layout, &config_for(&layout)).unwrap();
        shortcuts(&layout).unwrap();
        for link in [layout.start_menu_link(), layout.desktop_link()] {
            // Windows may read a path back in its long form while the temp folder was given in short form (as on
            // a hosted build machine), so the paths are compared as places, not as text.
            let back = read_back(&link);
            assert!(same_place(Path::new(&back[0]), &layout.exe()), "{back:?}");
            let icon = back[1].strip_suffix(",0").unwrap_or_else(|| panic!("{back:?}"));
            assert!(same_place(Path::new(icon), &layout.icon()), "{back:?}");
            assert!(same_place(Path::new(&back[2]), &layout.programs), "{back:?}");
            assert_eq!(back[3], "BALLFIGHT Studio", "{back:?}");
            assert!(opens_the_app(&link, &layout));
        }
        assert_eq!(names_in(&layout.start_menu), [LINK_NAME]);
        assert_eq!(names_in(&layout.desktop), [LINK_NAME]);
        // Made again (the item id list holds folder times, so the bytes may differ), they still open the app.
        shortcuts(&layout).unwrap();
        assert!(opens_the_app(&layout.desktop_link(), &layout));
        assert!(same_place(
            Path::new(&read_back(&layout.desktop_link())[0]),
            &layout.exe()
        ));
    }

    #[test]
    fn a_shortcut_to_something_else_is_left_alone() {
        let dir = TempDir::new("install-otherlink");
        let layout = Layout::under(&dir.path().join("pc"));
        fs::create_dir_all(&layout.desktop).unwrap();
        fs::create_dir_all(&layout.programs).unwrap();
        fs::write(layout.exe(), b"pretend exe").unwrap();
        let other = lnk::bytes(
            Path::new(r"C:\Windows\notepad.exe"),
            Path::new(r"C:\Windows"),
            Path::new(r"C:\Windows\notepad.exe"),
            "Someone's own shortcut",
        )
        .unwrap();
        fs::write(layout.desktop_link(), &other).unwrap();
        let err = shortcuts(&layout).unwrap_err().to_string();
        assert!(err.contains("opens something else"), "{err}");
        assert_eq!(fs::read(layout.desktop_link()).unwrap(), other);
        // The other shortcut was still made.
        assert!(opens_the_app(&layout.start_menu_link(), &layout));
        // After an install this is a note, not a failure.
        let said = shortcuts_said(&layout);
        assert!(said.contains("opens something else") && said.ends_with('.'), "{said}");
        assert_eq!(fs::read(layout.desktop_link()).unwrap(), other);
    }

    /// Runs a fixed PowerShell script in a test, with its values in environment variables, never in the script.
    /// Gives what it printed.
    fn test_script(script: &str, env: &[(&str, &OsStr)]) -> String {
        let mut c = crate::run::quiet(
            &powershell().unwrap(),
            &[
                OsStr::new("-NoLogo"),
                OsStr::new("-NoProfile"),
                OsStr::new("-NonInteractive"),
                OsStr::new("-EncodedCommand"),
                OsStr::new(&encoded_command(script)),
            ],
        );
        for (k, v) in env {
            c.env(k, v);
        }
        let out = c.output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// Windows' own shell object writes a shortcut at `link` to `target`.
    fn windows_writes_link(link: &Path, target: &Path) {
        const SCRIPT: &str = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:BF_TEST_LINK)\n\
             $s.TargetPath = $env:BF_TEST_TARGET\n$s.Save()\n";
        test_script(
            SCRIPT,
            &[
                ("BF_TEST_LINK", link.as_os_str()),
                ("BF_TEST_TARGET", target.as_os_str()),
            ],
        );
    }

    /// The 8.3 short form of an existing folder, as Windows gives it.
    fn short_form(dir: &Path) -> PathBuf {
        const SCRIPT: &str = "$f = (New-Object -ComObject Scripting.FileSystemObject).GetFolder($env:BF_TEST_DIR)\n\
             [Console]::Out.WriteLine($f.ShortPath)\n";
        PathBuf::from(test_script(SCRIPT, &[("BF_TEST_DIR", dir.as_os_str())]))
    }

    /// Whether two paths are the same place, in whichever form each is written.
    fn same_place(a: &Path, b: &Path) -> bool {
        same_or_inside(a, b) && same_or_inside(b, a)
    }

    #[test]
    fn a_link_windows_wrote_reads_too() {
        // If Windows ever rewrites one of our shortcuts, removal must still know it.
        let dir = TempDir::new("install-winlink");
        let layout = Layout::under(&dir.path().join("pc"));
        fs::create_dir_all(&layout.programs).unwrap();
        fs::create_dir_all(&layout.desktop).unwrap();
        fs::write(layout.exe(), b"pretend exe").unwrap();
        windows_writes_link(&layout.desktop_link(), &layout.exe());
        let bytes = fs::read(layout.desktop_link()).unwrap();
        // Windows writes the long form, whatever form the temp folder was given in.
        assert!(same_place(&lnk::target(&bytes).unwrap(), &layout.exe()));
        assert!(opens_the_app(&layout.desktop_link(), &layout));
    }

    /// Needs a volume that makes 8.3 short names (the system drive normally does). On one that does not, there is
    /// nothing to compare, so it passes early: the reason is printed on the error output, which the test runner
    /// shows only with `--nocapture` or when a test fails.
    #[test]
    fn a_path_in_short_form_is_the_same_place() {
        let dir = TempDir::new("install-short");
        let long_root = dir.path().join("a folder with a long name");
        fs::create_dir_all(&long_root).unwrap();
        let short_root = short_form(&long_root);
        let short_name = short_root.file_name().unwrap().to_string_lossy().into_owned();
        if short_name.eq_ignore_ascii_case("a folder with a long name") {
            eprintln!("note: this volume makes no 8.3 short names, so the short-form case cannot be tried here");
            return;
        }
        assert!(short_name.contains('~'), "{}", short_root.display());
        assert_eq!(long_form(&short_root), long_form(&long_root));
        assert!(same_place(
            &short_root.join("not there yet").join("x.exe"),
            &long_root.join("not there yet").join("x.exe")
        ));

        // Installed through the short form, as a program given a short %TEMP% would.
        let short = Layout::under(&short_root);
        let long = Layout::under(&long_root);
        install(&package_of(dir.path(), "0.6.0"), &short, &config_for(&short)).unwrap();
        shortcuts(&short).unwrap();
        // Read back by Windows in long form, the shortcut is still ours, under either layout.
        assert!(same_place(Path::new(&read_back(&short.desktop_link())[0]), &long.exe()));
        for layout in [&short, &long] {
            assert!(opens_the_app(&short.start_menu_link(), layout));
        }
        // A shortcut Windows itself wrote is ours too, so removal takes it. (With a path partly in short form,
        // Windows may write it with no LinkInfo, only its item id list and a relative path.)
        windows_writes_link(&short.desktop_link(), &long.exe());
        assert!(opens_the_app(&short.desktop_link(), &short));
        let plan = crate::remove::removal_plan(&short).unwrap();
        assert!(
            plan.files.iter().any(|(f, _)| f == &short.desktop_link()),
            "{:?}",
            plan.kept
        );
        assert!(plan.kept.is_empty(), "{:?}", plan.kept);
        // The installed exe named in long form is the installed one: it may remove the install made in short form.
        let temp = dir.path().join("temp");
        fs::create_dir_all(&temp).unwrap();
        let done = crate::remove::remove_app_with(&short, &long.exe(), &temp, &|a, b| fs::rename(a, b)).unwrap();
        assert!(done.moved_exe.is_some(), "{done:?}");
        assert!(!long.programs.exists() && !long.desktop_link().exists() && !long.start_menu_link().exists());
    }

    #[test]
    fn a_link_without_link_info_is_read_by_its_relative_path() {
        // The shape Windows writes when it leaves the LinkInfo out: header, item id list, then the strings, with a
        // name and a relative path (flags HasLinkTargetIDList | HasName | HasRelativePath | IsUnicode).
        let mut b = Vec::new();
        b.extend_from_slice(&0x4Cu32.to_le_bytes());
        b.extend_from_slice(&[
            0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        b.extend_from_slice(&(0x1u32 | 0x4 | 0x8 | 0x80).to_le_bytes());
        b.resize(76, 0);
        b.extend_from_slice(&[6, 0, 4, 0, 0xAA, 0xBB, 0, 0]);
        for s in ["BALLFIGHT Studio", r"..\Programs\BALLFIGHT Studio\ballfight-studio.exe"] {
            let units: Vec<u16> = s.encode_utf16().collect();
            b.extend_from_slice(&(units.len() as u16).to_le_bytes());
            b.extend(units.iter().flat_map(|u| u.to_le_bytes()));
        }
        b.extend_from_slice(&[0; 4]);
        assert!(lnk::target(&b).is_none());
        assert_eq!(
            lnk::target_at(&b, Path::new(r"C:\Users\Someone\Desktop")).unwrap(),
            PathBuf::from(r"C:\Users\Someone\Programs\BALLFIGHT Studio\ballfight-studio.exe")
        );
        // Climbing above the drive gives nothing; cut-short bytes give nothing.
        assert!(lnk::target_at(&b, Path::new(r"C:\")).is_none());
        assert!(lnk::target_at(&b[..b.len() - 20], Path::new(r"C:\x")).is_none());
    }

    /// A link put together by hand: the header with `flags`, then `link_info` as given, then Unicode `strings`.
    fn hand_link(flags: u32, link_info: &[u8], strings: &[&str]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0x4Cu32.to_le_bytes());
        b.extend_from_slice(&[
            0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        b.extend_from_slice(&flags.to_le_bytes());
        b.resize(76, 0);
        b.extend_from_slice(link_info);
        for s in strings {
            let units: Vec<u16> = s.encode_utf16().collect();
            b.extend_from_slice(&(units.len() as u16).to_le_bytes());
            b.extend(units.iter().flat_map(|u| u.to_le_bytes()));
        }
        b.extend_from_slice(&[0; 4]);
        b
    }

    /// A LinkInfo whose local path (Unicode) is `path`.
    fn link_info_with_local_path(path: &str) -> Vec<u8> {
        let unicode: Vec<u8> = path.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
        let (vol, base, suffix, base_u) = (0x24u32, 0x24 + 17, 0x24 + 18, 0x24 + 19);
        let suffix_u = base_u + unicode.len() as u32;
        let mut li = Vec::new();
        for n in [suffix_u + 2, 0x24, 1, vol, base, 0, suffix, base_u, suffix_u] {
            li.extend_from_slice(&n.to_le_bytes());
        }
        for n in [17u32, 3, 0, 16] {
            li.extend_from_slice(&n.to_le_bytes());
        }
        li.extend_from_slice(&[0, 0, 0]); // empty label, empty ANSI path, empty ANSI suffix
        li.extend_from_slice(&unicode);
        li.extend_from_slice(&[0, 0]);
        li
    }

    #[test]
    fn a_relative_path_is_used_only_when_there_is_no_link_info() {
        const LINK_INFO_NAME_RELATIVE_UNICODE: u32 = 0x2 | 0x4 | 0x8 | 0x80;
        // A network-only LinkInfo (no local base path) plus a relative path that would lead somewhere local.
        let mut network_only = Vec::new();
        for n in [0x1Cu32, 0x1C, 2, 0, 0, 0x1C, 0] {
            network_only.extend_from_slice(&n.to_le_bytes());
        }
        let b = hand_link(
            LINK_INFO_NAME_RELATIVE_UNICODE,
            &network_only,
            &["BALLFIGHT Studio", r"..\Programs\BALLFIGHT Studio\ballfight-studio.exe"],
        );
        assert_eq!(lnk::target_at(&b, Path::new(r"C:\Users\Someone\Desktop")), None);
        // With a local LinkInfo path, that path is the answer, not the relative one.
        let local = link_info_with_local_path(r"C:\Apps\ballfight-studio.exe");
        let b = hand_link(
            LINK_INFO_NAME_RELATIVE_UNICODE,
            &local,
            &["x", r"..\Elsewhere\other.exe"],
        );
        assert_eq!(
            lnk::target_at(&b, Path::new(r"C:\Users\Someone\Desktop")),
            Some(PathBuf::from(r"C:\Apps\ballfight-studio.exe"))
        );
    }

    #[test]
    fn a_shortcut_to_another_computer_is_not_ours_without_asking_it() {
        let dir = TempDir::new("install-unc");
        let layout = Layout::under(&dir.path().join("pc"));
        fs::create_dir_all(&layout.programs).unwrap();
        fs::create_dir_all(&layout.desktop).unwrap();
        fs::write(layout.exe(), b"pretend exe").unwrap();
        // `.invalid` names no host anywhere; a lookup of it would take seconds to fail.
        for target in [
            r"\\bf-test-host.invalid\share\ballfight-studio.exe",
            r"\\?\UNC\bf-test-host.invalid\share\ballfight-studio.exe",
            r"\\.\C:\ballfight-studio.exe",
        ] {
            let b = hand_link(0x2 | 0x80, &link_info_with_local_path(target), &[]);
            assert_eq!(lnk::target(&b), Some(PathBuf::from(target)));
            fs::write(layout.desktop_link(), &b).unwrap();
            let started = std::time::Instant::now();
            assert!(!opens_the_app(&layout.desktop_link(), &layout), "{target}");
            assert!(
                started.elapsed() < std::time::Duration::from_millis(500),
                "{target}: {:?}",
                started.elapsed()
            );
        }
        for (p, ok) in [
            (r"C:\x.exe", true),
            (r"c:\x", true),
            (r"\\host\share\x.exe", false),
            (r"\\?\C:\x.exe", false),
            (r"C:x.exe", false),
            (r"x.exe", false),
        ] {
            assert_eq!(on_a_drive_letter(Path::new(p)), ok, "{p}");
        }
    }

    #[test]
    fn a_link_reads_back_through_our_reader() {
        let b = lnk::bytes_with(
            None,
            Path::new(r"C:\Users\Someone\AppData\Local\Programs\BALLFIGHT Studio\ballfight-studio.exe"),
            Path::new(r"C:\Users\Someone\AppData\Local\Programs\BALLFIGHT Studio"),
            Path::new(r"C:\Users\Someone\AppData\Local\Programs\BALLFIGHT Studio\ballfight.ico"),
            "BALLFIGHT Studio",
        )
        .unwrap();
        assert_eq!(&b[..4], &[0x4C, 0, 0, 0]);
        assert_eq!(
            lnk::target(&b).unwrap(),
            PathBuf::from(r"C:\Users\Someone\AppData\Local\Programs\BALLFIGHT Studio\ballfight-studio.exe")
        );
        // With an item id list in front, the reader steps over it.
        let ids = [4u8, 0, 0xAA, 0xBB, 0, 0];
        let with = lnk::bytes_with(
            Some(&ids),
            Path::new(r"C:\Apps\BALLFIGHT Studio\ballfight-studio.exe"),
            Path::new(r"C:\Apps\BALLFIGHT Studio"),
            Path::new(r"C:\Apps\BALLFIGHT Studio\ballfight.ico"),
            "BALLFIGHT Studio",
        )
        .unwrap();
        assert_eq!(&with[76..78], &[6, 0]);
        assert_eq!(
            lnk::target(&with).unwrap(),
            PathBuf::from(r"C:\Apps\BALLFIGHT Studio\ballfight-studio.exe")
        );
        assert!(
            lnk::bytes_with(
                Some(&[4, 0, 1, 2]),
                Path::new(r"C:\a.exe"),
                Path::new(r"C:\"),
                Path::new(r"C:\a.ico"),
                "x"
            )
            .is_err()
        );
        assert!(lnk::target(&b[..60]).is_none());
        assert!(lnk::target(b"not a link at all, just some text bytes in a file that is long enough").is_none());
        assert!(
            lnk::bytes(
                Path::new("relative.exe"),
                Path::new(r"C:\"),
                Path::new(r"C:\x.ico"),
                "x"
            )
            .is_err()
        );
        let long = format!(r"C:\{}\x.exe", "a".repeat(300));
        assert!(lnk::bytes(Path::new(&long), Path::new(r"C:\"), Path::new(r"C:\x.ico"), "x").is_err());
    }

    #[test]
    fn base64_is_right() {
        // "A" in UTF-16LE is 41 00.
        assert_eq!(encoded_command("A"), "QQA=");
        assert_eq!(encoded_command("ab"), "YQBiAA==");
        assert_eq!(encoded_command("abc"), "YQBiAGMA");
    }

    /// Runs git with fixed arguments in a test (local repositories only, no network).
    fn git(git: &Path, args: &[&OsStr]) {
        let never = std::sync::atomic::AtomicBool::new(false);
        // Every GIT_ variable of the test's own environment goes, so a test run from a git hook never acts on the
        // repository around it.
        let how = crate::run::Family {
            scrub_git: true,
            ..Default::default()
        };
        let ran =
            crate::run::run_family(git, args, &how, std::time::Duration::from_secs(60), &never, &mut |_| {}).unwrap();
        assert!(ran.status.success(), "git {args:?}: {:?}", ran.tail);
    }

    #[test]
    fn the_source_copy_follows_the_owners_clone() {
        let Some(g) = find_git() else {
            eprintln!("note: git is not on this PC, so the source copy was not tried");
            return;
        };
        let dir = TempDir::new("install-source");
        let upstream = dir.path().join("upstream");
        let clone = dir.path().join("owner clone");
        let o = |s: &'static str| OsStr::new(s);
        git(&g, &[o("init"), o("-q"), upstream.as_os_str()]);
        fs::write(upstream.join("a.txt"), b"a").unwrap();
        git(&g, &[o("-C"), upstream.as_os_str(), o("add"), o("a.txt")]);
        git(
            &g,
            &[
                o("-C"),
                upstream.as_os_str(),
                o("-c"),
                o("user.name=Test"),
                o("-c"),
                o("user.email=test@example.invalid"),
                o("commit"),
                o("-q"),
                o("-m"),
                o("one"),
            ],
        );
        git(&g, &[o("clone"), o("-q"), upstream.as_os_str(), clone.as_os_str()]);
        let data = dir.path().join("data");
        let said = std::cell::RefCell::new(Vec::new());
        let say = |l: &str| said.borrow_mut().push(l.to_string());
        let c = source_copy_with(&g, &clone, &data, &say, true).unwrap();
        assert_eq!(*said.borrow(), [COPYING_SOURCE]);
        assert_eq!(c.source, data.join("source"));
        assert_eq!(c.build_dir, data.join("build"));
        assert_eq!(c.downloads, data.join("downloads"));
        assert_eq!(c.tag_prefix, "studio-v");
        assert!(same_place(Path::new(&c.remote), &upstream), "{}", c.remote);
        assert!(data.join("source").join(".git").is_dir());
        assert!(!data.join("source").join("a.txt").exists(), "no checkout");
        // Again: the copy is kept.
        assert_eq!(source_copy_with(&g, &clone, &data, &say, true).unwrap(), c);
        assert_eq!(said.borrow().len(), 1, "nothing is copied again");
        // A copy of another repository there is refused and left alone.
        let other = dir.path().join("other");
        git(&g, &[o("init"), o("-q"), other.as_os_str()]);
        let data2 = dir.path().join("data2");
        git(
            &g,
            &[o("clone"), o("-q"), other.as_os_str(), data2.join("source").as_os_str()],
        );
        let err = source_copy_with(&g, &clone, &data2, &say, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("another repository"), "{err}");

        // A source folder without its own .git (git would walk up to the repository around it), and one with the
        // right origin but no branches (a clone stopped part-way): not finished copies, and left as they are.
        let data3 = dir.path().join("data3");
        fs::create_dir_all(data3.join("source")).unwrap();
        fs::write(data3.join("source").join("half.txt"), b"x").unwrap();
        let err = source_copy_with(&g, &clone, &data3, &say, true)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("is not a finished copy of the source; delete it, then install again."),
            "{err}"
        );
        assert!(data3.join("source").join("half.txt").is_file());
        let data4 = dir.path().join("data4");
        let half = data4.join("source");
        git(&g, &[o("init"), o("-q"), half.as_os_str()]);
        git(
            &g,
            &[
                o("-C"),
                half.as_os_str(),
                o("remote"),
                o("add"),
                o("origin"),
                upstream.as_os_str(),
            ],
        );
        let err = source_copy_with(&g, &clone, &data4, &say, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("is not a finished copy"), "{err}");
        assert!(half.join(".git").is_dir());
        // A clone that fails says why and leaves nothing it did not make.
        let gone = dir.path().join("no such repository");
        git(
            &g,
            &[
                o("-C"),
                clone.as_os_str(),
                o("remote"),
                o("set-url"),
                o("origin"),
                gone.as_os_str(),
            ],
        );
        let data5 = dir.path().join("data5");
        let err = source_copy_with(&g, &clone, &data5, &say, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Git could not copy the repository"), "{err}");
        // The folder the clone was owed: no source copy.
        assert!(!data5.join("source").join(".git").exists());
        // The owner's clone itself must have its own .git.
        // Outside tests, a local repository is never taken as where updates come from.
        let err = source_copy(&g, &clone, &dir.path().join("data-real"), &say).unwrap_err();
        assert_eq!(
            err,
            PackError::Refused(crate::config::REMOTE_NOT_HTTPS_OR_SSH.to_string())
        );
        assert!(!dir.path().join("data-real").exists());
        let plain = dir.path().join("plain folder");
        fs::create_dir_all(&plain).unwrap();
        assert!(source_copy_with(&g, &plain, &dir.path().join("data6"), &say, true).is_err());
    }

    /// A copy of the folder `from` (files and folders) at `to`.
    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy_tree(&e.path(), &to.join(e.file_name()));
            } else {
                fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }

    /// Version `old` installed under `root\pc` with a person's file in it, and version `new` staged in its `.next`
    /// (as the update engine stages one). Gives the layout.
    fn installed_with_next(root: &Path, old: &str, new: &str) -> Layout {
        let layout = Layout::under(&root.join("pc"));
        install(&package_of(root, old), &layout, &config_for(&layout)).unwrap();
        fs::write(layout.programs.join("my notes.txt"), b"mine").unwrap();
        copy_tree(&package_of(root, new), &layout.programs.join(".next"));
        layout
    }

    #[test]
    fn an_update_is_put_in_place_and_the_earlier_one_kept() {
        let dir = TempDir::new("update-swap");
        let layout = installed_with_next(dir.path(), "0.6.0", "0.6.1");
        let p = &layout.programs;
        let config_before = read(&p.join(CONFIG_NAME));
        assert!(!swap_unfinished(p));
        let m = put_in_place(p, &p.join(".next")).unwrap();
        assert_eq!(m.version, "0.6.1");
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1");
        check_installed(p);
        assert_eq!(read(&p.join(CONFIG_NAME)), config_before, "update.toml stays as it is");
        assert_eq!(read(&p.join("my notes.txt")), b"mine");
        assert!(!p.join(".next").exists(), "the staged folder is used up");
        let prev = p.join(".previous");
        assert_eq!(read(&prev.join(EXE_NAME)), b"pretend exe 0.6.0");
        check_installed(&prev);
        assert!(prev.join(CONFIG_NAME).is_file() && !prev.join(SWAP_MARKER).exists());
        assert!(!swap_unfinished(p));

        // "Go back": the kept version comes back as the next one, by the same swap.
        fs::rename(&prev, p.join(".next")).unwrap();
        let m = put_in_place(p, &p.join(".next")).unwrap();
        assert_eq!(m.version, "0.6.0");
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        check_installed(p);
        assert_eq!(read(&p.join(CONFIG_NAME)), config_before);
        assert_eq!(read(&prev.join(EXE_NAME)), b"pretend exe 0.6.1");
        assert!(!p.join(".next").exists());
        assert_eq!(read(&p.join("my notes.txt")), b"mine");
        // Removal still knows every file.
        let r = crate::remove::removal_plan(&layout).unwrap();
        assert!(r.kept.iter().all(|(f, _)| f.ends_with("my notes.txt")), "{:?}", r.kept);
    }

    /// The exe at the shortcut's target is there, and is the whole earlier or the whole new one.
    fn exe_is_whole(layout: &Layout, n: usize) {
        let now = read(&layout.exe());
        assert!(
            now == b"pretend exe 0.6.0" || now == b"pretend exe 0.6.1",
            "n = {n}: the exe is {:?}",
            String::from_utf8_lossy(&now)
        );
    }

    #[test]
    fn an_update_stopped_anywhere_is_finished_by_trying_again() {
        let dir = TempDir::new("update-stops");
        let mut stops = 0;
        for n in 0..30 {
            let root = dir.path().join(format!("n{n}"));
            let layout = installed_with_next(&root, "0.6.0", "0.6.1");
            let p = &layout.programs;
            match put_in_place_stopping(p, &p.join(".next"), Some(n)) {
                Err(PackError::Io(t)) if t == STOPPED => {
                    stops += 1;
                    // The folder is always one of ours, removal can always plan it, and the exe is always there.
                    assert!(read_manifest(&p.join(MANIFEST_NAME)).is_ok(), "n = {n}");
                    crate::remove::removal_plan(&layout).unwrap();
                    exe_is_whole(&layout, n);
                    // Stopped after making `.previous` and before its marker, nothing has changed yet.
                    if n > 1 {
                        assert!(swap_unfinished(p), "n = {n}");
                    }
                    let m = put_in_place(p, &p.join(".next")).unwrap();
                    assert_eq!(m.version, "0.6.1", "n = {n}");
                }
                Ok(_) => {}
                Err(e) => panic!("n = {n}: {e}"),
            }
            assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1", "n = {n}");
            check_installed(p);
            check_installed(&p.join(".previous"));
            assert_eq!(read(&p.join(".previous").join(EXE_NAME)), b"pretend exe 0.6.0");
            assert!(!swap_unfinished(p), "n = {n}");
            assert!(!p.join(".next").exists(), "n = {n}");
            assert_eq!(read(&p.join("my notes.txt")), b"mine");
        }
        assert!(stops >= 15, "{stops}");
    }

    #[test]
    fn an_update_stopped_anywhere_can_be_taken_back() {
        let dir = TempDir::new("update-undo");
        let mut undone = 0;
        for n in 2..30 {
            let root = dir.path().join(format!("n{n}"));
            let layout = installed_with_next(&root, "0.6.0", "0.6.1");
            let p = &layout.programs;
            let before = names_in(p);
            match put_in_place_stopping(p, &p.join(".next"), Some(n)) {
                Err(PackError::Io(t)) if t == STOPPED && swap_state(p) == SwapState::Stopped => {
                    let lock = swap_lock(p).unwrap();
                    let m = undo_stopped(p, &p.join(".next"), &lock).unwrap();
                    drop(lock);
                    undone += 1;
                    assert_eq!(m.version, "0.6.0");
                    assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0", "n = {n}");
                    check_installed(p);
                    assert!(!swap_unfinished(p), "n = {n}");
                    assert_eq!(names_in(p), before, "n = {n}");
                    check_package(&p.join(".next")).unwrap();
                    assert_eq!(read(&p.join("my notes.txt")), b"mine");
                    // And the update can then go ahead as if nothing happened.
                    put_in_place(p, &p.join(".next")).unwrap();
                    assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1");
                }
                _ => {}
            }
        }
        assert!(undone >= 10, "{undone}");
    }

    fn ping_exe() -> PathBuf {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE")
    }

    /// Version 0.6.0 installed (a pretend exe) and version 0.6.1 staged in `.next` whose exe is a real program (a
    /// copy of Windows' ping), so a test can run "the new version".
    fn installed_with_runnable_next(root: &Path) -> Layout {
        let layout = Layout::under(&root.join("pc"));
        install(&package_of(root, "0.6.0"), &layout, &config_for(&layout)).unwrap();
        let inputs = root.join("runnable-inputs");
        let (exe, tree, checked) = crate::package::tests::fake_inputs(&inputs, "0.6.1");
        fs::copy(ping_exe(), &exe).unwrap();
        crate::package::package(&exe, &tree, &checked, &layout.programs.join(".next")).unwrap();
        layout
    }

    /// Runs the program at `exe` (a copy of ping) for about half a minute.
    fn run(exe: &Path) -> std::process::Child {
        let child = crate::run::quiet(exe, &[OsStr::new("-n"), OsStr::new("30"), OsStr::new("127.0.0.1")])
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        child
    }

    /// The set-aside running exes in `.previous`.
    fn set_aside_exes(p: &Path) -> Vec<String> {
        names_in(&p.join(".previous"))
            .into_iter()
            .filter(|n| crate::package::is_set_aside_exe(n))
            .collect()
    }

    /// Whether the program `child` still runs.
    fn still_runs(child: &mut std::process::Child) -> bool {
        child.try_wait().is_ok_and(|s| s.is_none())
    }

    // Each take-back runs twice: as Windows 10 and 11 on NTFS behave (a replacing rename may replace a running
    // program's file), and as Windows behaves where it may not (the running exe is then set aside by name).

    #[test]
    fn a_failed_update_is_taken_back_while_the_new_exe_runs() {
        for refuse in [false, true] {
            NO_REPLACE_WHILE_RUNNING.with(|n| n.set(refuse));
            let dir = TempDir::new("update-undo-running");
            let layout = installed_with_runnable_next(dir.path());
            let p = &layout.programs;
            let next = p.join(".next");
            let ping_bytes = read(&ping_exe());
            // The new version runs from .next (as the finishing program does); its exe moves into place first, then
            // a later new file cannot move (held), so the swap is taken back with the new exe running at its place.
            let mut finisher = run(&next.join(EXE_NAME));
            let held = hold(&next.join("ffmpeg").join("ffprobe.exe"));
            let err = put_in_place(p, &next).unwrap_err().to_string();
            drop(held);
            assert!(still_runs(&mut finisher), "the new program ran all along");
            assert!(
                err.contains("So nothing changed: version 0.6.0 is still installed."),
                "{refuse}: {err}"
            );
            assert_eq!(
                read(&layout.exe()),
                b"pretend exe 0.6.0",
                "the earlier exe is back at its place"
            );
            assert!(!swap_unfinished(p));
            check_installed(p);
            assert_eq!(set_aside_exes(p).len(), usize::from(refuse), "{refuse}");
            let _ = finisher.kill();
            let _ = finisher.wait();
            // Once it has ended, the next swap clears it by its name and goes ahead.
            put_in_place(p, &next).unwrap();
            assert_eq!(read(&layout.exe()), ping_bytes);
            assert!(set_aside_exes(p).is_empty());
            check_installed(p);
        }
        NO_REPLACE_WHILE_RUNNING.with(|n| n.set(false));
    }

    #[test]
    fn a_stopped_update_is_taken_back_while_the_new_exe_runs_at_its_place() {
        let ping_bytes = read(&ping_exe());
        for refuse in [false, true] {
            NO_REPLACE_WHILE_RUNNING.with(|n| n.set(refuse));
            let dir = TempDir::new("update-undo-place");
            let mut ran_new = 0;
            for n in 2..30 {
                let root = dir.path().join(format!("n{n}"));
                let layout = installed_with_runnable_next(&root);
                let p = &layout.programs;
                let next = p.join(".next");
                let stopped = matches!(put_in_place_stopping(p, &next, Some(n)), Err(PackError::Io(t)) if t == STOPPED);
                if !stopped || swap_state(p) != SwapState::Stopped {
                    continue;
                }
                // Where the new exe is already at its place, it is the one running (the new version, started by a
                // double-click, taking the update back).
                let new_running = read(&layout.exe()) == ping_bytes;
                let mut child = new_running.then(|| run(&layout.exe()));
                let lock = swap_lock(p).unwrap();
                let undone = undo_stopped(p, &next, &lock);
                drop(lock);
                if let Some(c) = &mut child {
                    assert!(still_runs(c), "n = {n}: the new program ran all along");
                    let _ = c.kill();
                    let _ = c.wait();
                    ran_new += 1;
                }
                let m = undone.unwrap_or_else(|e| panic!("{refuse} n = {n}: {e}"));
                assert_eq!(m.version, "0.6.0");
                assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0", "{refuse} n = {n}");
                check_installed(p);
                assert!(!swap_unfinished(p), "n = {n}");
                check_package(&next).unwrap();
                assert_eq!(
                    set_aside_exes(p).len(),
                    usize::from(refuse && new_running),
                    "{refuse} n = {n}"
                );
                // Removal plans every file of ours, the set-aside ones included.
                let r = crate::remove::removal_plan(&layout).unwrap();
                assert!(r.kept.is_empty(), "{refuse} n = {n}: {:?}", r.kept);
                // And the update can go ahead afterwards; the set-aside file goes with it.
                put_in_place(p, &next).unwrap_or_else(|e| panic!("{refuse} n = {n}: {e}"));
                assert_eq!(read(&layout.exe()), ping_bytes);
                assert!(set_aside_exes(p).is_empty(), "n = {n}");
            }
            assert!(ran_new >= 5, "{ran_new}");
        }
        NO_REPLACE_WHILE_RUNNING.with(|n| n.set(false));
    }

    #[test]
    fn a_take_back_that_cannot_finish_keeps_the_marker_and_says_so() {
        let dir = TempDir::new("update-undo-stuck");
        let layout = installed_with_runnable_next(dir.path());
        let p = &layout.programs;
        let next = p.join(".next");
        let held_new = hold(&next.join("ffmpeg").join("ffprobe.exe"));
        FAIL_EXE_BACK.with(|f| f.set(true));
        let err = put_in_place(p, &next).unwrap_err().to_string();
        FAIL_EXE_BACK.with(|f| f.set(false));
        drop(held_new);
        // Never "nothing changed" here: the words say a start finishes or takes it back, and the marker says so too.
        assert!(!err.contains("nothing changed"), "{err}");
        assert!(err.contains(AGAIN_UPDATE), "{err}");
        assert!(swap_unfinished(p));
        let exe = read(&layout.exe());
        assert!(exe == read(&ping_exe()) || exe == b"pretend exe 0.6.0");
        // A start takes it back.
        let lock = swap_lock(p).unwrap();
        assert_eq!(undo_stopped(p, &next, &lock).unwrap().version, "0.6.0");
        drop(lock);
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        assert!(!swap_unfinished(p));
        check_installed(p);
    }
    #[test]
    fn two_swaps_never_run_at_once() {
        let dir = TempDir::new("update-lock");
        let layout = installed_with_next(dir.path(), "0.6.0", "0.6.1");
        let p = &layout.programs;
        let held = swap_lock(p).unwrap();
        assert_eq!(swap_lock(p).err(), Some(PackError::Refused(SWAP_BUSY.to_string())));
        assert_eq!(
            put_in_place(p, &p.join(".next")).unwrap_err(),
            PackError::Refused(SWAP_BUSY.to_string())
        );
        let pkg = package_of(dir.path(), "0.6.2");
        assert_eq!(
            install(&pkg, &layout, &config_for(&layout)).unwrap_err(),
            PackError::Refused(SWAP_BUSY.to_string())
        );
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0", "nothing moved");
        drop(held);
        put_in_place(p, &p.join(".next")).unwrap();
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1");
    }
    #[test]
    fn a_failed_update_puts_everything_back() {
        let dir = TempDir::new("update-fail");
        let layout = installed_with_next(dir.path(), "0.6.0", "0.6.1");
        let p = &layout.programs;
        // An earlier file held open without sharing delete cannot move into .previous.
        let held = hold(&p.join("ffmpeg").join("ffprobe.exe"));
        let err = put_in_place(p, &p.join(".next")).unwrap_err().to_string();
        drop(held);
        assert!(err.contains("0.6.0 is still installed"), "{err}");
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        check_installed(p);
        assert!(!swap_unfinished(p));
        // The staged version is whole again, so it can be tried again.
        check_package(&p.join(".next")).unwrap();
        put_in_place(p, &p.join(".next")).unwrap();
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.1");
    }

    #[test]
    fn an_update_waits_for_the_earlier_version_to_close() {
        let dir = TempDir::new("update-running");
        let layout = installed_with_next(dir.path(), "0.6.0", "0.6.1");
        let p = &layout.programs;
        let ping = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE");
        // The earlier version's exe is a real running program (a copy of ping): the update is refused.
        let old_manifest = read_manifest(&p.join(MANIFEST_NAME)).unwrap();
        fs::remove_file(layout.exe()).unwrap();
        fs::copy(&ping, layout.exe()).unwrap();
        let ping_sha = sha256_file(&layout.exe()).unwrap();
        let mut m = old_manifest.clone();
        for f in &mut m.files {
            if f.0 == EXE_NAME {
                f.1 = fs::metadata(layout.exe()).unwrap().len();
                f.2 = ping_sha.clone();
            }
        }
        fs::write(p.join(MANIFEST_NAME), m.text()).unwrap();
        let mut child = crate::run::quiet(
            &layout.exe(),
            &[OsStr::new("-n"), OsStr::new("9"), OsStr::new("127.0.0.1")],
        )
        .spawn()
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let err = put_in_place(p, &p.join(".next"));
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(err.unwrap_err().to_string(), CLOSE_FIRST);
        assert_eq!(
            sha256_file(&layout.exe()).unwrap(),
            ping_sha,
            "the running copy is untouched"
        );
        assert!(!p.join(".previous").exists());
        check_package(&p.join(".next")).unwrap();
    }

    #[test]
    fn the_new_version_finishing_its_own_swap_is_not_refused() {
        let dir = TempDir::new("update-own");
        let root = dir.path();
        let layout = Layout::under(&root.join("pc"));
        install(&package_of(root, "0.6.0"), &layout, &config_for(&layout)).unwrap();
        // Version 0.6.1's exe is a real program (a copy of ping).
        let inputs = root.join("ping-inputs");
        let (exe, tree, checked) = crate::package::tests::fake_inputs(&inputs, "0.6.1");
        let ping = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE");
        fs::copy(&ping, &exe).unwrap();
        let next = layout.programs.join(".next");
        crate::package::package(&exe, &tree, &checked, &next).unwrap();
        // Stopped right after the new exe moved in (marker, two copies, the earlier files, then the exe).
        let p = &layout.programs;
        let old_files = read_manifest(&p.join(MANIFEST_NAME)).unwrap().files.len();
        let err = put_in_place_stopping(p, &next, Some(4 + old_files + 1)).unwrap_err();
        assert_eq!(err, PackError::Io(STOPPED.to_string()));
        assert_eq!(sha256_file(&layout.exe()).unwrap(), sha256_file(&ping).unwrap());
        // The new exe runs from its place (as the new version does while it finishes the swap): not refused.
        let mut child = crate::run::quiet(
            &layout.exe(),
            &[OsStr::new("-n"), OsStr::new("9"), OsStr::new("127.0.0.1")],
        )
        .spawn()
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let done = put_in_place(p, &next);
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(done.unwrap().version, "0.6.1");
        check_installed(p);
        assert!(!swap_unfinished(p));
    }

    #[test]
    fn an_update_needs_a_whole_staged_package() {
        let dir = TempDir::new("update-whole");
        let layout = installed_with_next(dir.path(), "0.6.0", "0.6.1");
        let p = &layout.programs;
        fs::write(p.join(".next").join("ballfight.ico"), b"changed").unwrap();
        assert!(put_in_place(p, &p.join(".next")).is_err());
        assert_eq!(read(&layout.exe()), b"pretend exe 0.6.0");
        assert!(!p.join(".previous").exists());
        // Nothing installed: refused in words.
        let empty = dir.path().join("none").join(APP_NAME);
        let err = put_in_place(&empty, &p.join(".next")).unwrap_err().to_string();
        assert!(err.contains("No version of BALLFIGHT Studio is installed"), "{err}");
    }
}
