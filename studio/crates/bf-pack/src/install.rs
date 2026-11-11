//! The install and the shortcuts (plan M6, Task 11; decision D2: a plain per-user folder and two shortcuts, no
//! installer, no admin rights, no registry).
//!
//! Every function takes its folders from a `Layout`. Only `Layout::system()` names the real ones
//! (`%LOCALAPPDATA%\Programs\BALLFIGHT Studio`, the Start menu's `Programs` folder, the desktop); tests use
//! `Layout::under(<temp folder>)`.
//!
//! **The install never destroys a working copy.** Files are copied into `BALLFIGHT Studio.installing` beside the
//! program folder and checked against the package's manifest. Only then are they put in place: a fresh install is
//! one rename of the whole folder; over an earlier install, each file the earlier manifest lists moves into
//! `.previous` (as an update does) and each new file moves in, and if any move fails every move is undone. A file in
//! the program folder that the earlier manifest does not list (a person's own) is never replaced or moved: if a new
//! file would land on it, the install stops in words before anything changes.
//!
//! **Shortcuts** are written by `lnk`, a small writer of Windows' shell link format (MS-SHLLINK) in this file: no
//! shell command line, no COM, no unsafe code. Tests read every link back through Windows' own shell object.

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::config::{CONFIG_NAME, TAG_PREFIX, UpdateConfig, check_remote, write_config};
use crate::package::{
    EXE_NAME, ICON_NAME, MANIFEST_NAME, Manifest, check_package, is_link, join_relative, read_manifest,
};
use crate::sha::sha256_file;
use crate::{PackError, io_error};

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

/// The names of a path, lower-cased, without `.` parts; `None` for a relative path or one with `..`.
fn names(p: &Path) -> Option<Vec<String>> {
    if !p.is_absolute() {
        return None;
    }
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
/// `update.toml`. See the module notes for how a working copy is kept.
pub fn install(package: &Path, layout: &Layout, config: &UpdateConfig) -> Result<(), PackError> {
    check_layout(layout)?;
    let new = check_package(package)?;
    let target = &layout.programs;
    let parent = target
        .parent()
        .ok_or_else(|| PackError::Refused("The program folder has no parent folder.".to_string()))?;
    fs::create_dir_all(parent).map_err(|e| io_error("make the folder", parent, &e))?;
    let old = installed_manifest(target)?;

    // Everything that could stop the install is checked before anything is written.
    let mut housekeeping = Vec::new();
    if let Some(old) = &old {
        refuse_running(target, old)?;
        check_conflicts(target, old, &new)?;
        for name in [".previous", ".next"] {
            let d = target.join(name);
            if fs::symlink_metadata(&d).is_ok() {
                housekeeping.push(plan_clear(&d)?);
            }
        }
    }
    let staging = layout.staging();
    clear_leftover_staging(&staging)?;
    if let Err(e) = stage(package, &new, &staging, config) {
        let _ = clear_leftover_staging(&staging);
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
                        "An earlier version's file {} could not be cleared away ({why}), so nothing was installed; \
                         version {} is still installed.",
                        p.display(),
                        old.version
                    )));
                    break;
                }
            }
            cleared.and_then(|()| swap_in(target, old, &new, &staging))
        }
    };
    if done.is_err() {
        let _ = clear_leftover_staging(&staging);
    }
    done
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
/// left alone and the install stops in words.
fn plan_clear(dir: &Path) -> Result<crate::remove::Removal, PackError> {
    let mut r = crate::remove::Removal::default();
    let empty = dir.is_dir() && !is_link(dir) && fs::read_dir(dir).map(|mut d| d.next().is_none()).unwrap_or(false);
    if empty {
        r.dirs.push(dir.to_path_buf());
        return Ok(r);
    }
    let foreign = |what: &Path| {
        PackError::Refused(format!(
            "{} holds {}, which BALLFIGHT Studio did not put there, so nothing was installed. Move it elsewhere first.",
            dir.display(),
            what.display()
        ))
    };
    if is_link(dir) || !dir.is_dir() || !dir.join(MANIFEST_NAME).is_file() {
        return Err(foreign(dir));
    }
    crate::remove::plan_folder(dir, &mut r)?;
    if let Some((p, _)) = r.kept.first() {
        return Err(foreign(p));
    }
    Ok(r)
}

/// Clears what a stopped install left in the staging folder: only the names its own manifest lists (whatever their
/// bytes, since a copy may have stopped part-way), its manifest and `update.toml`, then its empty folders. Anything
/// else in it stops the install in words, with nothing removed.
fn clear_leftover_staging(staging: &Path) -> Result<(), PackError> {
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
            || manifest
                .as_ref()
                .is_some_and(|m| m.files.iter().any(|(p, _, _)| p == rel))
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
fn make_parents(path: &Path, root: &Path, made: &mut Vec<PathBuf>) -> Result<(), PackError> {
    let Some(dir) = path.parent() else { return Ok(()) };
    let mut missing: Vec<&Path> = dir
        .ancestors()
        .take_while(|a| *a != root && same_or_inside(a, root))
        .filter(|a| fs::symlink_metadata(a).is_err())
        .collect();
    missing.reverse();
    for d in missing {
        fs::create_dir(d).map_err(|e| io_error("make the folder", d, &e))?;
        made.push(d.to_path_buf());
    }
    Ok(())
}

/// Puts the staged files in place over an earlier install: each file the earlier manifest lists (and its manifest
/// and `update.toml`) moves into a new `.previous`, then each staged file moves in, the manifest last. If any move
/// fails, every move is undone in reverse and the earlier version is as it was.
fn swap_in(target: &Path, old: &Manifest, new: &Manifest, staging: &Path) -> Result<(), PackError> {
    let prev = target.join(".previous");
    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut made: Vec<PathBuf> = Vec::new();
    let mut old_names: Vec<&str> = old.files.iter().map(|(p, _, _)| p.as_str()).collect();
    old_names.extend([CONFIG_NAME, MANIFEST_NAME]);
    let mut new_names: Vec<&str> = new.files.iter().map(|(p, _, _)| p.as_str()).collect();
    new_names.extend([CONFIG_NAME, MANIFEST_NAME]);

    let moved = (|| {
        fs::create_dir(&prev).map_err(|e| io_error("make the folder", &prev, &e))?;
        made.push(prev.clone());
        for rel in &old_names {
            let from = join_relative(target, rel);
            if is_link(&from) || !from.is_file() {
                continue;
            }
            let to = join_relative(&prev, rel);
            make_parents(&to, &prev, &mut made)?;
            fs::rename(&from, &to).map_err(|e| io_error("move the earlier version's", &from, &e))?;
            moves.push((from, to));
        }
        for rel in &new_names {
            let from = join_relative(staging, rel);
            let to = join_relative(target, rel);
            make_parents(&to, target, &mut made)?;
            fs::rename(&from, &to).map_err(|e| io_error("put in place", &to, &e))?;
            moves.push((from, to));
        }
        Ok::<(), PackError>(())
    })();

    if let Err(e) = moved {
        let mut not_back = Vec::new();
        for (from, to) in moves.iter().rev() {
            if fs::rename(to, from).is_err() {
                not_back.push(from.display().to_string());
            }
        }
        for d in made.iter().rev() {
            let _ = fs::remove_dir(d);
        }
        let mut text = format!("{e} So nothing changed: version {} is still installed.", old.version);
        if !not_back.is_empty() {
            text = format!(
                "{e} Putting the earlier version back failed for {}; the rest of version {} is still installed.",
                not_back.join(", "),
                old.version
            );
        }
        return Err(PackError::Io(text));
    }

    // Folders the earlier version had and the new one does not: removed when empty, deepest first.
    let mut old_dirs: Vec<PathBuf> = old_names
        .iter()
        .flat_map(|rel| {
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
    let _ = clear_leftover_staging(staging);
    Ok(())
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

/// Whether the file at `link` is a shortcut that opens this layout's installed exe.
pub fn opens_the_app(link: &Path, layout: &Layout) -> bool {
    if is_link(link) || !link.is_file() {
        return false;
    }
    let Ok(bytes) = fs::read(link) else {
        return false;
    };
    lnk::target(&bytes).is_some_and(|t| names(&t).is_some() && names(&t) == names(&layout.exe()))
}

/// The app's own copy of the repository for updates (route A), and the `update.toml` that records it. Reads the
/// owner's clone's `origin` (`git -C <from> remote get-url origin`), refuses an address with a user name or
/// password without repeating it, then clones it with `--no-checkout` into `<data>\source`, unless a copy of the
/// same repository is already there (kept). A folder there that is a copy of another repository is refused. Git
/// handles its own sign-in.
pub fn source_copy(git: &Path, from: &Path, data: &Path) -> Result<UpdateConfig, PackError> {
    let o = OsStr::new;
    let remote = git_says(
        git,
        &[o("-C"), from.as_os_str(), o("remote"), o("get-url"), o("origin")],
    )
    .map_err(|_| {
        PackError::Refused(format!(
            "Git could not read where the copy of the source at {} comes from, so updates could not be set up.",
            from.display()
        ))
    })?;
    check_remote(&remote)?;
    let source = data.join("source");
    if fs::symlink_metadata(&source).is_ok() {
        let same = !is_link(&source)
            && source.is_dir()
            && git_says(
                git,
                &[o("-C"), source.as_os_str(), o("remote"), o("get-url"), o("origin")],
            )
            .is_ok_and(|theirs| theirs == remote);
        if !same {
            return Err(PackError::Refused(format!(
                "{} is a copy of another repository, so it was left as it is and updates were not set up. Move it \
                 elsewhere, then install again.",
                source.display()
            )));
        }
    } else {
        fs::create_dir_all(data).map_err(|e| io_error("make the folder", data, &e))?;
        // `--` ends the options, so the address can never be read as one.
        let out = git_output(
            git,
            &[
                o("clone"),
                o("--no-checkout"),
                o("--quiet"),
                o("--"),
                o(&remote),
                source.as_os_str(),
            ],
        )
        .map_err(|e| PackError::Io(format!("Git could not be started: {e}.")))?;
        if !out.status.success() {
            let said = String::from_utf8_lossy(&out.stderr);
            let last = said
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("no reason given")
                .trim();
            return Err(PackError::Io(format!(
                "Git could not copy the repository into {}, so updates could not be set up. Git said \"{last}\".",
                source.display()
            )));
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

/// Git run with its arguments through the one way to start a program and waited for, never waiting on a console
/// prompt (git's own sign-in window may still open for the owner).
fn git_output(git: &Path, args: &[&OsStr]) -> std::io::Result<std::process::Output> {
    crate::run::quiet(git, args).env("GIT_TERMINAL_PROMPT", "0").output()
}

/// What git printed, trimmed, when it ended well.
fn git_says(git: &Path, args: &[&OsStr]) -> Result<String, ()> {
    let out = git_output(git, args).map_err(|_| ())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(())
    }
}

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

/// Windows PowerShell, `<drive>:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`, from `SystemRoot` checked
/// as for curl (a drive's `Windows` folder); never from `PATH`.
pub fn powershell() -> Result<PathBuf, PackError> {
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
            let back = read_back(&link);
            assert_eq!(back[0], layout.exe().display().to_string(), "{back:?}");
            assert_eq!(back[1], format!("{},0", layout.icon().display()), "{back:?}");
            assert_eq!(back[2], layout.programs.display().to_string(), "{back:?}");
            assert_eq!(back[3], "BALLFIGHT Studio", "{back:?}");
            assert!(opens_the_app(&link, &layout));
        }
        assert_eq!(names_in(&layout.start_menu), [LINK_NAME]);
        assert_eq!(names_in(&layout.desktop), [LINK_NAME]);
        // Made again (the item id list holds folder times, so the bytes may differ), they still open the app.
        shortcuts(&layout).unwrap();
        assert!(opens_the_app(&layout.desktop_link(), &layout));
        assert_eq!(read_back(&layout.desktop_link())[0], layout.exe().display().to_string());
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
    }

    #[test]
    fn a_link_windows_wrote_reads_too() {
        // If Windows ever rewrites one of our shortcuts, removal must still know it. A link written by Windows' own
        // shell object in a temp folder (the paths come in environment variables, never in the script).
        const SCRIPT: &str = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:BF_TEST_LINK)\n\
             $s.TargetPath = $env:BF_TEST_TARGET\n$s.Save()\n";
        let dir = TempDir::new("install-winlink");
        let layout = Layout::under(&dir.path().join("pc"));
        fs::create_dir_all(&layout.programs).unwrap();
        fs::create_dir_all(&layout.desktop).unwrap();
        fs::write(layout.exe(), b"pretend exe").unwrap();
        let out = crate::run::quiet(
            &powershell().unwrap(),
            &[
                OsStr::new("-NoLogo"),
                OsStr::new("-NoProfile"),
                OsStr::new("-NonInteractive"),
                OsStr::new("-EncodedCommand"),
                OsStr::new(&encoded_command(SCRIPT)),
            ],
        )
        .env("BF_TEST_LINK", layout.desktop_link())
        .env("BF_TEST_TARGET", layout.exe())
        .output()
        .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let bytes = fs::read(layout.desktop_link()).unwrap();
        assert_eq!(lnk::target(&bytes).unwrap(), layout.exe());
        assert!(opens_the_app(&layout.desktop_link(), &layout));
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
        let ran = crate::run::run(git, args, std::time::Duration::from_secs(60), &never).unwrap();
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
        let c = source_copy(&g, &clone, &data).unwrap();
        assert_eq!(c.source, data.join("source"));
        assert_eq!(c.build_dir, data.join("build"));
        assert_eq!(c.downloads, data.join("downloads"));
        assert_eq!(c.tag_prefix, "studio-v");
        assert_eq!(PathBuf::from(&c.remote), upstream);
        assert!(data.join("source").join(".git").is_dir());
        assert!(!data.join("source").join("a.txt").exists(), "no checkout");
        // Again: the copy is kept.
        assert_eq!(source_copy(&g, &clone, &data).unwrap(), c);
        // A copy of another repository there is refused and left alone.
        let other = dir.path().join("other");
        git(&g, &[o("init"), o("-q"), other.as_os_str()]);
        let data2 = dir.path().join("data2");
        git(
            &g,
            &[o("clone"), o("-q"), other.as_os_str(), data2.join("source").as_os_str()],
        );
        let err = source_copy(&g, &clone, &data2).unwrap_err().to_string();
        assert!(err.contains("another repository"), "{err}");
    }
}
