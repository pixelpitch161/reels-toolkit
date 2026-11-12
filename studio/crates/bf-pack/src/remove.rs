//! Removing the app from this PC (plan M6, Task 11; "Remove BALLFIGHT Studio from this PC" in Settings, Task 13).
//!
//! **Only what an install wrote goes**, file by file, from the manifest the install wrote: each file `MANIFEST.txt`
//! lists whose SHA-256 still matches, the manifest itself, the install's `update.toml`, the same inside `.previous`
//! and `.next` by their own manifests, what a stopped install left (its staging folder, by its own manifest, and new
//! files already in place that match it), the shortcuts that open the installed exe, and then each folder that is
//! left empty. Nothing is ever removed by deleting a folder with what is in it.
//!
//! Refused, with nothing changed: a program folder without a manifest of ours, one not named `BALLFIGHT Studio`, a
//! relative path, a drive root, the user's own folder or anything around it, the data folder or anything around it,
//! and a program folder that is a link. Inside the folder, a link is never followed: what it leads to stays, and so
//! does the link.
//!
//! **What stays** (this follows the controller's rule for this task, which keeps more than decision D7 f): every
//! file a person added to the program folder, any of our files that has changed since it was installed, the
//! settings and the queue (`%APPDATA%\BALLFIGHT Studio`), the whole of `%LOCALAPPDATA%\BALLFIGHT Studio` (the
//! shortlist cache, fault reports and logs, and the rebuildable `data\source`, `data\build` and `data\downloads`,
//! none of which an install's manifest lists), and every video. No registry value is written or removed: the
//! install writes none (D2).
//!
//! **In the app's own process** (ruling M6-R2): `remove_app` runs on the person's click. Windows lets a running exe
//! be renamed but not deleted, so the app's own exe (once its SHA-256 matches the manifest) is first moved out of the
//! program folder to a new name in the temp folder, when that is on the same drive; then everything planned is
//! removed at once, and the words say what went, what stayed, and where the moved exe is. No other program is
//! started.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::PackError;
use crate::config::{CONFIG_NAME, read_config};
use crate::install::{Layout, check_layout, opens_the_app, same_or_inside};
use crate::package::io_error;
use crate::package::{MANIFEST_NAME, Manifest, is_link, join_relative, read_manifest};
use crate::sha::sha256_file;

/// What a removal will do: the files that go (each with the SHA-256 it must still have), the folders that go if they
/// are empty by then (deepest first), and what stays, each with why.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Removal {
    pub files: Vec<(PathBuf, String)>,
    pub dirs: Vec<PathBuf>,
    pub kept: Vec<(PathBuf, String)>,
}

impl Removal {
    /// Everything that goes, in order: the files (shortcuts first), then the folders.
    pub fn goes(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .map(|(p, _)| p.clone())
            .chain(self.dirs.iter().cloned())
            .collect()
    }
}

/// What a removal left: what went, what stayed (each with why), and what became of the running exe.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Removed {
    /// The program folder that was removed from.
    pub programs: PathBuf,
    /// Every planned file and folder that is gone now.
    pub went: Vec<PathBuf>,
    /// Everything that stayed, with why.
    pub stayed: Vec<(PathBuf, String)>,
    /// Where the running exe was moved to, in the temp folder.
    pub moved_exe: Option<PathBuf>,
    /// The running exe, when it could not be moved out (so it, and its folder, stay).
    pub exe_left: Option<PathBuf>,
}

/// What would be removed from this layout, and what would stay. Refuses (with nothing changed) a layout that is
/// not ours; see the module notes.
pub fn removal_plan(layout: &Layout) -> Result<Removal, PackError> {
    check_layout(layout)?;
    let p = &layout.programs;
    if fs::symlink_metadata(p).is_err() {
        return Err(PackError::Refused(format!(
            "BALLFIGHT Studio is not there in {}, so there is nothing to remove.",
            p.display()
        )));
    }
    let manifest = p.join(MANIFEST_NAME);
    if !p.is_dir() || is_link(&manifest) || !manifest.is_file() || read_manifest(&manifest).is_err() {
        return Err(PackError::Refused(format!(
            "The folder {} was not made by BALLFIGHT Studio's install, so nothing is removed.",
            p.display()
        )));
    }
    let mut r = Removal::default();
    for link in [layout.start_menu_link(), layout.desktop_link()] {
        if fs::symlink_metadata(&link).is_err() {
            continue;
        }
        if opens_the_app(&link, layout) {
            let sha = sha256_file(&link).map_err(|e| io_error("read", &link, &e))?;
            r.files.push((link, sha));
        } else {
            r.kept.push((
                link,
                "it is a shortcut of that name that opens something else".to_string(),
            ));
        }
    }
    // A stopped install leaves its staging folder beside the program folder, and maybe some new files already in
    // place: both are ours by the staging folder's own manifest.
    let staging = layout.staging();
    let staged = (!is_link(&staging) && staging.is_dir())
        .then(|| read_manifest(&staging.join(MANIFEST_NAME)).ok())
        .flatten();
    if staged.is_some() {
        plan_folder(&staging, &mut r, None)?;
    }
    plan_folder(p, &mut r, staged.as_ref())?;
    Ok(r)
}

/// Plans the removal of one folder that holds a manifest of ours (the program folder, `.previous`, `.next`, a
/// staging folder), adding to `r`: each file its manifest (or `also`) lists whose SHA-256 still matches,
/// `update.toml` when it is one of ours, the manifest, then the folders deepest first and `dir` itself. Everything
/// else in it is added to `r.kept`, and links are never entered.
pub(crate) fn plan_folder(dir: &Path, r: &mut Removal, also: Option<&Manifest>) -> Result<(), PackError> {
    use crate::package::{Kind, walk};
    let manifest_path = dir.join(MANIFEST_NAME);
    let m = read_manifest(&manifest_path)?;
    let mut dirs = Vec::new();
    for (rel, kind) in walk(dir)? {
        let path = join_relative(dir, &rel);
        let top = rel.split('/').next().unwrap_or("");
        let nested = top == ".previous" || top == ".next";
        if nested && rel != top {
            continue; // Planned below by its own manifest.
        }
        match kind {
            Kind::Link => r.kept.push((path, "it is a link, which is never followed".to_string())),
            Kind::Dir if nested => {
                let mut inner = Removal::default();
                let own = path.join(MANIFEST_NAME);
                if !is_link(&own) && own.is_file() && plan_folder(&path, &mut inner, None).is_ok() {
                    r.files.append(&mut inner.files);
                    r.dirs.append(&mut inner.dirs);
                    r.kept.append(&mut inner.kept);
                } else {
                    let only_folders = walk(&path)?;
                    // Without a manifest, a swap's own files (its marker, its copies' temp names, a running exe it
                    // set aside) are still ours by name.
                    let by_name = |rel: &str| {
                        rel == crate::install::SWAP_MARKER
                            || crate::install::COPY_PARTS.contains(&rel)
                            || crate::package::is_set_aside_exe(rel)
                    };
                    let ours = |(rel, k): &(String, Kind)| *k == Kind::Dir || (*k == Kind::File && by_name(rel));
                    if only_folders.iter().all(ours) {
                        for (inner_rel, inner_kind) in &only_folders {
                            if *inner_kind == Kind::File {
                                let f = join_relative(&path, inner_rel);
                                let now = sha256_file(&f).map_err(|e| io_error("read", &f, &e))?;
                                r.files.push((f, now));
                            }
                        }
                        r.dirs.extend(
                            only_folders
                                .iter()
                                .rev()
                                .filter(|(_, k)| *k == Kind::Dir)
                                .map(|(d, _)| join_relative(&path, d)),
                        );
                        r.dirs.push(path.clone());
                    }
                    for (inner_rel, inner_kind) in only_folders {
                        if inner_kind != Kind::Dir && !by_name(&inner_rel) {
                            r.kept.push((
                                join_relative(&path, &inner_rel),
                                "BALLFIGHT Studio cannot tell that it put it there".to_string(),
                            ));
                        }
                    }
                }
            }
            Kind::Dir => dirs.push(path),
            Kind::File if rel == MANIFEST_NAME => {}
            // A swap's own marker and copy names (see `install::swap_in`) are ours by name, whatever they hold.
            // ... and so is a running exe a taken-back swap set aside (`ballfight-studio.exe.running-N`).
            Kind::File
                if rel == crate::install::SWAP_MARKER
                    || crate::install::COPY_PARTS.contains(&rel.as_str())
                    || crate::package::is_set_aside_exe(&rel) =>
            {
                let now = sha256_file(&path).map_err(|e| io_error("read", &path, &e))?;
                r.files.push((path, now));
            }
            Kind::File => {
                let ours_config = rel == CONFIG_NAME && read_config(&path).is_ok();
                let wants: Vec<&String> = std::iter::once(&m)
                    .chain(also)
                    .filter_map(|man| man.files.iter().find(|(p, _, _)| p == &rel).map(|(_, _, sha)| sha))
                    .collect();
                if ours_config {
                    let now = sha256_file(&path).map_err(|e| io_error("read", &path, &e))?;
                    r.files.push((path, now));
                } else if wants.is_empty() {
                    r.kept.push((path, "BALLFIGHT Studio did not put it there".to_string()));
                } else {
                    let now = sha256_file(&path).map_err(|e| io_error("read", &path, &e))?;
                    if wants.iter().any(|w| **w == now) {
                        r.files.push((path, now));
                    } else {
                        r.kept.push((path, "it has changed since it was installed".to_string()));
                    }
                }
            }
        }
    }
    let sha = sha256_file(&manifest_path).map_err(|e| io_error("read", &manifest_path, &e))?;
    r.files.push((manifest_path, sha));
    // The walk lists a folder before what it holds, so the reverse lists each folder after everything in it.
    r.dirs.extend(dirs.into_iter().rev());
    r.dirs.push(dir.to_path_buf());
    Ok(())
}

/// Removes what `r` plans, here and now, checking each file's SHA-256 again just before removing it. Gives what
/// stayed, with why.
pub fn remove_now(r: &Removal) -> Vec<(PathBuf, String)> {
    let mut stayed = r.kept.clone();
    let is_manifest = |f: &Path| f.file_name().is_some_and(|n| n == MANIFEST_NAME);
    // Every other file first; then a folder's manifest only when nothing planned in that folder stayed, so that
    // the removal (or an install) can simply be run again over what is left.
    let mut failed: Vec<PathBuf> = Vec::new();
    for pass_manifests in [false, true] {
        for (f, sha) in r.files.iter().filter(|(f, _)| is_manifest(f) == pass_manifests) {
            if fs::symlink_metadata(f).is_err() {
                continue;
            }
            if pass_manifests && f.parent().is_some_and(|dir| failed.iter().any(|x| x.starts_with(dir))) {
                stayed.push((f.clone(), "it is kept so that removing can be run again".to_string()));
                continue;
            }
            let why = if is_link(f) || !f.is_file() {
                Some("it is no longer a plain file".to_string())
            } else {
                match sha256_file(f) {
                    Ok(now) if &now == sha => fs::remove_file(f).err().map(|e| {
                        format!(
                            "it could not be removed ({})",
                            crate::package::plain_reason(&e).to_lowercase()
                        )
                    }),
                    _ => Some("it has changed since the removal was planned".to_string()),
                }
            };
            if let Some(why) = why {
                failed.push(f.clone());
                stayed.push((f.clone(), why));
            }
        }
    }
    for d in &r.dirs {
        if is_link(d) || !d.is_dir() {
            continue;
        }
        // Only an empty folder can go: `remove_dir` never removes what is in a folder.
        if fs::remove_dir(d).is_err() {
            stayed.push((d.clone(), "it still holds files".to_string()));
        }
    }
    stayed
}

/// Removes a folder that holds a manifest of ours (such as a package made in a temp folder) file by file, now. Gives
/// what stayed; a folder without our manifest is left as it is, and that is an error.
pub fn remove_by_manifest(dir: &Path) -> Result<Vec<(PathBuf, String)>, PackError> {
    if is_link(dir) {
        return Err(PackError::Refused(format!(
            "{} is a link, so nothing was removed.",
            dir.display()
        )));
    }
    let mut r = Removal::default();
    plan_folder(dir, &mut r, None)?;
    Ok(remove_now(&r))
}

/// The drive (or share) a full path is on, in lower case.
fn volume(p: &Path) -> Option<String> {
    match p.components().next()? {
        Component::Prefix(x) => Some(x.as_os_str().to_string_lossy().to_lowercase()),
        _ => None,
    }
}

/// Removes BALLFIGHT Studio from this layout now, from inside the running app (Task 13 calls this on the person's
/// click with `std::env::current_exe()`, then shows `removed_words` and closes). Refused, with nothing changed,
/// unless `running_exe` is the installed exe `layout.exe()` itself (not a build elsewhere, not the copy in
/// `.previous` or staging); and refusals are those of `removal_plan`.
pub fn remove_app(layout: &Layout, running_exe: &Path) -> Result<Removed, PackError> {
    remove_app_with(layout, running_exe, &std::env::temp_dir(), &|from, to| {
        fs::rename(from, to)
    })
}

/// `remove_app` with the temp folder and the rename step given, so tests can try each branch.
pub(crate) fn remove_app_with(
    layout: &Layout,
    running_exe: &Path,
    temp: &Path,
    rename: &dyn Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<Removed, PackError> {
    // Only the installed copy itself may remove the install: never a developer's build, a test program, the
    // packaging tool, or the exe kept in `.previous` or being staged.
    let installed = layout.exe();
    let is_installed = same_or_inside(running_exe, &installed) && same_or_inside(&installed, running_exe);
    if !is_installed {
        return Err(PackError::Refused(format!(
            "This copy of BALLFIGHT Studio is not the installed one in {}, so nothing was removed.",
            layout.programs.display()
        )));
    }
    let mut plan = removal_plan(layout)?;
    let mut done = Removed {
        programs: layout.programs.clone(),
        ..Removed::default()
    };
    let same = |a: &Path, b: &Path| a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy());
    if let Some(at) = plan.files.iter().position(|(f, _)| same(f, running_exe)) {
        let (exe, sha) = plan.files.remove(at);
        let matches = sha256_file(&exe).is_ok_and(|now| now == sha);
        let same_drive = volume(&exe).is_some() && volume(&exe) == volume(temp);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let to = temp.join(format!("BALLFIGHT Studio removed {}-{stamp}.exe", std::process::id()));
        if !matches {
            done.stayed
                .push((exe, "it has changed since it was installed".to_string()));
        } else if same_drive && fs::symlink_metadata(&to).is_err() && rename(&exe, &to).is_ok() {
            done.went.push(exe);
            done.moved_exe = Some(to);
        } else {
            done.exe_left = Some(exe);
        }
    }
    let goes = plan.goes();
    done.stayed.extend(remove_now(&plan));
    // The empty swap lock file beside the program folder goes too, once the folder is gone and nobody holds it.
    let lock = crate::install::swap_lock_path(&layout.programs);
    if fs::symlink_metadata(&layout.programs).is_err()
        && !is_link(&lock)
        && fs::metadata(&lock).is_ok_and(|m| m.is_file() && m.len() == 0)
        && crate::install::swap_lock(&layout.programs).is_ok()
    {
        let _ = fs::remove_file(&lock);
    }
    done.went
        .extend(goes.into_iter().filter(|p| fs::symlink_metadata(p).is_err()));
    Ok(done)
}

/// What Settings says before a removal (Task 13): what goes, what stays, and anything of the person's that stays in
/// the program folder. Whole plain sentences.
pub fn removal_words(layout: &Layout, r: &Removal) -> Vec<String> {
    let going = |link: &Path| r.files.iter().any(|(f, _)| f == link);
    let shortcuts = match (going(&layout.start_menu_link()), going(&layout.desktop_link())) {
        (true, true) => " and its Start menu and desktop shortcuts",
        (true, false) => " and its Start menu shortcut",
        (false, true) => " and its desktop shortcut",
        (false, false) => "",
    };
    let mut words = vec![
        format!(
            "Removing takes away the program in {}{shortcuts}, and BALLFIGHT Studio closes.",
            layout.programs.display()
        ),
        "Your settings, the queue, fault reports and logs, and your videos are kept in folders of their own, and \
         they stay on this PC."
            .to_string(),
    ];
    let rebuildable: Vec<String> = ["build", "downloads"]
        .iter()
        .map(|n| layout.data.join(n))
        .filter(|d| d.is_dir())
        .map(|d| d.display().to_string())
        .collect();
    if !rebuildable.is_empty() {
        words.push(format!(
            "The update builds and downloads stay too. To free that space, delete {} yourself afterwards.",
            rebuildable.join(" and ")
        ));
    }
    let names = kept_names(&layout.programs, &r.kept);
    if !names.is_empty() {
        words.push(format!(
            "These files in the program folder were not put there by BALLFIGHT Studio, or have changed, so they \
             stay: {}.",
            names.join(", ")
        ));
    }
    words
}

/// The names of the files (not folders) that stay in the program folder.
fn kept_names(programs: &Path, kept: &[(PathBuf, String)]) -> Vec<String> {
    kept.iter()
        .filter(|(p, _)| p.starts_with(programs) && !p.is_dir())
        .filter_map(|(p, _)| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}

/// What the app says after `remove_app`, before it closes: what went, what stayed, and the one program file left
/// behind, with where it is. Whole plain sentences.
pub fn removed_words(r: &Removed) -> Vec<String> {
    let mut words = Vec::new();
    if fs::symlink_metadata(&r.programs).is_err() {
        words.push(format!("BALLFIGHT Studio was removed from {}.", r.programs.display()));
    } else {
        words.push(format!(
            "BALLFIGHT Studio was removed, but the folder {} stays because something is still in it.",
            r.programs.display()
        ));
    }
    if let Some(moved) = &r.moved_exe {
        words.push(format!(
            "The program file that was running was moved to {}. Windows clears it by itself, or delete it.",
            moved.display()
        ));
    }
    if let Some(left) = &r.exe_left {
        words.push(format!(
            "The program file {} could not be moved away while it runs. Delete it yourself after BALLFIGHT Studio \
             has closed.",
            left.display()
        ));
    }
    let names = kept_names(&r.programs, &r.stayed);
    if !names.is_empty() {
        words.push(format!("These files stayed: {}.", names.join(", ")));
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::tests::{config_for, hold, make_junction, names_in, package_of};
    use crate::install::{LINK_NAME, install, shortcuts};
    use crate::package::EXE_NAME;
    use crate::testdir::TempDir;
    use std::ffi::OsStr;

    /// A PC under `root`: version 0.6.0 then 0.6.1 installed (so `.previous` holds 0.6.0), both shortcuts, a
    /// person's own file in the program folder, the app's data and settings, and a video.
    fn a_pc(root: &Path) -> Layout {
        let layout = Layout::under(&root.join("pc"));
        let old = package_of(root, "0.6.0");
        let new = package_of(root, "0.6.1");
        install(&old, &layout, &config_for(&layout)).unwrap();
        install(&new, &layout, &config_for(&layout)).unwrap();
        shortcuts(&layout).unwrap();
        fs::write(layout.programs.join("my notes.txt"), b"mine").unwrap();
        for (p, bytes) in [
            (layout.data.join("logs").join("start.txt"), &b"started"[..]),
            (layout.data.join("logs").join("crash-20261111-120000.txt"), b"a fault"),
            (
                layout.data.join("source").join(".git").join("HEAD"),
                b"ref: refs/heads/main",
            ),
            (layout.data.join("build").join("release").join("x.rlib"), b"built"),
            (layout.data.join("downloads").join("ffmpeg.zip"), b"zip"),
            (root.join("pc").join("cache").join("shortlists").join("a.json"), b"{}"),
            (root.join("pc").join("config").join("settings.toml"), b"format = 1"),
            (root.join("pc").join("config").join("queue.json"), b"{}"),
            (
                root.join("pc").join("Videos").join("BALLFIGHT").join("fight.mp4"),
                b"a video",
            ),
            (layout.desktop.join("another shortcut.lnk"), b"someone else's"),
        ] {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, bytes).unwrap();
        }
        layout
    }

    fn rel(layout: &Layout, p: &Path) -> String {
        let root = layout.programs.parent().unwrap().parent().unwrap();
        p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn removal_plans_only_the_apps_own_paths() {
        let dir = TempDir::new("remove-plan");
        let layout = a_pc(dir.path());
        let r = removal_plan(&layout).unwrap();
        let goes: Vec<String> = r.goes().iter().map(|p| rel(&layout, p)).collect();
        let p = "Programs/BALLFIGHT Studio";
        let mut want = vec![
            "Start Menu/Programs/BALLFIGHT Studio.lnk".to_string(),
            "Desktop/BALLFIGHT Studio.lnk".to_string(),
        ];
        for prefix in [format!("{p}/.previous"), format!("{p}/.next"), p.to_string()] {
            if prefix.ends_with(".next") {
                continue;
            }
            for f in [
                "ballfight-studio.exe",
                "ballfight.ico",
                "ffmpeg/ABOUT.txt",
                "ffmpeg/LICENSE.txt",
                "ffmpeg/ffmpeg.exe",
                "ffmpeg/ffprobe.exe",
                "guide/index.html",
                "guide/pictures/start.png",
                "update.toml",
                "MANIFEST.txt",
            ] {
                want.push(format!("{prefix}/{f}"));
            }
        }
        for d in [
            format!("{p}/.previous/guide/pictures"),
            format!("{p}/.previous/guide"),
            format!("{p}/.previous/ffmpeg"),
            format!("{p}/.previous"),
            format!("{p}/guide/pictures"),
            format!("{p}/guide"),
            format!("{p}/ffmpeg"),
            p.to_string(),
        ] {
            want.push(d);
        }
        assert_eq!(goes, want);
        // Nothing of the person's, the settings, the data, the cache or the videos.
        for g in &goes {
            for never in ["data/", "config/", "cache/", "Videos/", "my notes", "another shortcut"] {
                assert!(!g.contains(never), "{g}");
            }
        }
        let kept: Vec<String> = r.kept.iter().map(|(p, _)| rel(&layout, p)).collect();
        assert_eq!(kept, [format!("{p}/my notes.txt")]);
        // Planning changed nothing.
        assert!(layout.exe().is_file() && layout.desktop_link().is_file());
    }

    #[test]
    fn a_folder_without_our_manifest_is_refused() {
        let dir = TempDir::new("remove-nomanifest");
        let layout = Layout::under(dir.path());
        // Missing altogether.
        assert!(removal_plan(&layout).unwrap_err().to_string().contains("is not there"));
        // There, but nothing of ours in it.
        fs::create_dir_all(&layout.programs).unwrap();
        fs::write(layout.programs.join(EXE_NAME), b"someone's").unwrap();
        let err = removal_plan(&layout).unwrap_err().to_string();
        assert!(err.contains("was not made by"), "{err}");
        // A MANIFEST.txt of another kind.
        fs::write(
            layout.programs.join(MANIFEST_NAME),
            "Some other program's manifest\nversion 1\n",
        )
        .unwrap();
        let err = removal_plan(&layout).unwrap_err().to_string();
        assert!(err.contains("was not made by"), "{err}");
        assert!(layout.programs.join(EXE_NAME).is_file());
    }

    #[test]
    fn a_layout_that_is_not_ours_is_refused() {
        let dir = TempDir::new("remove-layout");
        let layout = a_pc(dir.path());
        let mut other = layout.clone();
        other.programs = layout.programs.with_file_name("Something");
        assert!(
            removal_plan(&other)
                .unwrap_err()
                .to_string()
                .contains("not named BALLFIGHT Studio")
        );
        let mut root = layout.clone();
        root.programs = PathBuf::from(r"C:\");
        assert!(removal_plan(&root).is_err());
        if let Some(home) = std::env::var_os("USERPROFILE") {
            let mut profile = layout.clone();
            profile.programs = PathBuf::from(home);
            assert!(removal_plan(&profile).is_err());
        }
        let mut around = layout.clone();
        around.programs = dir.path().join("pc").join(crate::install::APP_NAME);
        around.data = around.programs.join("data");
        assert!(removal_plan(&around).is_err());
    }

    #[test]
    fn links_are_never_followed() {
        let dir = TempDir::new("remove-links");
        let layout = a_pc(dir.path());
        // The guide folder is swapped for a junction to a folder elsewhere holding files of the same names.
        let elsewhere = dir.path().join("elsewhere");
        fs::create_dir_all(elsewhere.join("pictures")).unwrap();
        fs::copy(layout.programs.join("guide/index.html"), elsewhere.join("index.html")).unwrap();
        fs::copy(
            layout.programs.join("guide/pictures/start.png"),
            elsewhere.join("pictures/start.png"),
        )
        .unwrap();
        fs::remove_file(layout.programs.join("guide/pictures/start.png")).unwrap();
        fs::remove_dir(layout.programs.join("guide/pictures")).unwrap();
        fs::remove_file(layout.programs.join("guide/index.html")).unwrap();
        fs::remove_dir(layout.programs.join("guide")).unwrap();
        if !make_junction(&elsewhere, &layout.programs.join("guide")) {
            eprintln!("note: could not make a junction here, so the link case was not tried");
            return;
        }
        let r = removal_plan(&layout).unwrap();
        for g in r.goes() {
            assert!(!g.starts_with(layout.programs.join("guide")), "{}", g.display());
            assert!(!g.starts_with(&elsewhere), "{}", g.display());
        }
        assert!(
            r.kept
                .iter()
                .any(|(p, why)| p == &layout.programs.join("guide") && why.contains("link"))
        );
        let stayed = remove_now(&r);
        assert!(elsewhere.join("index.html").is_file() && elsewhere.join("pictures/start.png").is_file());
        assert!(is_link(&layout.programs.join("guide")));
        assert!(stayed.iter().any(|(p, _)| p == &layout.programs));

        // A program folder that is itself a link is refused outright.
        let dir2 = TempDir::new("remove-links-top");
        let l2 = Layout::under(dir2.path());
        let target = dir2.path().join("real");
        fs::create_dir_all(&target).unwrap();
        fs::write(
            target.join(MANIFEST_NAME),
            "BALLFIGHT Studio package manifest 1\nversion 0.6.0\n",
        )
        .unwrap();
        fs::create_dir_all(l2.programs.parent().unwrap()).unwrap();
        if make_junction(&target, &l2.programs) {
            assert!(removal_plan(&l2).unwrap_err().to_string().contains("link"));
            assert!(target.join(MANIFEST_NAME).is_file());
        }
    }

    #[test]
    fn a_changed_file_and_a_foreign_shortcut_stay() {
        let dir = TempDir::new("remove-changed");
        let layout = a_pc(dir.path());
        fs::write(layout.programs.join("ffmpeg").join("ABOUT.txt"), b"edited by a person").unwrap();
        let other = crate::install::lnk::bytes(
            Path::new(r"C:\Windows\notepad.exe"),
            Path::new(r"C:\Windows"),
            Path::new(r"C:\Windows\notepad.exe"),
            "x",
        )
        .unwrap();
        fs::write(layout.desktop_link(), &other).unwrap();
        let r = removal_plan(&layout).unwrap();
        let kept: Vec<String> = r.kept.iter().map(|(p, _)| rel(&layout, p)).collect();
        assert!(
            kept.contains(&"Programs/BALLFIGHT Studio/ffmpeg/ABOUT.txt".to_string()),
            "{kept:?}"
        );
        assert!(kept.contains(&"Desktop/BALLFIGHT Studio.lnk".to_string()), "{kept:?}");
        let stayed = remove_now(&r);
        assert_eq!(fs::read(layout.desktop_link()).unwrap(), other);
        assert_eq!(
            fs::read(layout.programs.join("ffmpeg/ABOUT.txt")).unwrap(),
            b"edited by a person"
        );
        assert!(!layout.start_menu_link().exists());
        assert!(!layout.exe().exists());
        assert_eq!(names_in(&layout.programs), ["ffmpeg", "my notes.txt"]);
        assert!(stayed.iter().any(|(p, _)| p == &layout.programs.join("ffmpeg")));
    }

    /// The data, settings, cache, videos and the other shortcut are all as `a_pc` left them.
    fn everything_else_stays(root: &Path, layout: &Layout) {
        for (p, bytes) in [
            (layout.data.join("logs").join("start.txt"), &b"started"[..]),
            (layout.data.join("logs").join("crash-20261111-120000.txt"), b"a fault"),
            (
                layout.data.join("source").join(".git").join("HEAD"),
                b"ref: refs/heads/main",
            ),
            (layout.data.join("build").join("release").join("x.rlib"), b"built"),
            (layout.data.join("downloads").join("ffmpeg.zip"), b"zip"),
            (root.join("pc").join("cache").join("shortlists").join("a.json"), b"{}"),
            (root.join("pc").join("config").join("settings.toml"), b"format = 1"),
            (root.join("pc").join("config").join("queue.json"), b"{}"),
            (
                root.join("pc").join("Videos").join("BALLFIGHT").join("fight.mp4"),
                b"a video",
            ),
            (layout.desktop.join("another shortcut.lnk"), b"someone else's"),
            (layout.programs.join("my notes.txt"), b"mine"),
        ] {
            assert_eq!(fs::read(&p).unwrap_or_default(), bytes, "{}", p.display());
        }
    }

    #[test]
    fn removing_now_takes_only_the_plan() {
        let dir = TempDir::new("remove-now");
        let layout = a_pc(dir.path());
        let r = removal_plan(&layout).unwrap();
        let stayed = remove_now(&r);
        assert_eq!(names_in(&layout.programs), ["my notes.txt"]);
        assert!(!layout.start_menu_link().exists() && !layout.desktop_link().exists());
        assert!(layout.start_menu.is_dir(), "the Start menu folder itself stays");
        everything_else_stays(dir.path(), &layout);
        assert!(stayed.iter().any(|(p, _)| p == &layout.programs.join("my notes.txt")));
    }

    /// `a_pc` without the person's own file, so a removal can take the whole program folder.
    fn a_clean_pc(root: &Path) -> Layout {
        let layout = a_pc(root);
        fs::remove_file(layout.programs.join("my notes.txt")).unwrap();
        layout
    }

    fn plain_rename(from: &Path, to: &Path) -> std::io::Result<()> {
        fs::rename(from, to)
    }

    #[test]
    fn the_app_removes_itself_and_its_exe_goes_to_the_temp_folder() {
        let dir = TempDir::new("remove-app");
        let layout = a_clean_pc(dir.path());
        let exe_bytes = fs::read(layout.exe()).unwrap();
        let temp = dir.path().join("temp");
        fs::create_dir_all(&temp).unwrap();
        let done = remove_app_with(&layout, &layout.exe(), &temp, &plain_rename).unwrap();
        assert!(!layout.programs.exists(), "{:?}", done.stayed);
        assert!(!layout.start_menu_link().exists() && !layout.desktop_link().exists());
        let moved = done.moved_exe.clone().unwrap();
        assert!(moved.starts_with(&temp));
        assert_eq!(fs::read(&moved).unwrap(), exe_bytes);
        assert_eq!(names_in(&temp).len(), 1);
        assert!(done.exe_left.is_none());
        assert!(done.went.contains(&layout.exe()) && done.went.contains(&layout.programs));
        everything_else_stays_but_my_notes(dir.path(), &layout);
        let words = removed_words(&done).join(" ");
        assert!(words.contains("was removed from"), "{words}");
        assert!(words.contains(&moved.display().to_string()), "{words}");
        assert!(words.contains("Windows clears it by itself, or delete it."), "{words}");
    }

    #[test]
    fn on_another_drive_the_exe_stays_and_is_named() {
        let dir = TempDir::new("remove-app-drive");
        let layout = a_clean_pc(dir.path());
        // A temp folder on another drive: Windows cannot rename across drives, so it is not tried.
        let other = if layout.exe().to_string_lossy().starts_with(['Z', 'z']) {
            r"Y:\temp"
        } else {
            r"Z:\temp"
        };
        let tried = std::cell::Cell::new(false);
        let rename = |_: &Path, _: &Path| {
            tried.set(true);
            Err(std::io::Error::other("not tried"))
        };
        let done = remove_app_with(&layout, &layout.exe(), Path::new(other), &rename).unwrap();
        assert!(!tried.get());
        assert_eq!(done.exe_left.as_deref(), Some(layout.exe().as_path()));
        assert!(layout.exe().is_file(), "the running exe stays");
        assert_eq!(names_in(&layout.programs), [EXE_NAME]);
        let words = removed_words(&done).join(" ");
        assert!(words.contains("stays because something is still in it"), "{words}");
        assert!(words.contains(&layout.exe().display().to_string()), "{words}");
        assert!(
            words.contains("Delete it yourself after BALLFIGHT Studio has closed."),
            "{words}"
        );
    }

    #[test]
    fn a_failed_rename_leaves_the_exe_and_names_it() {
        let dir = TempDir::new("remove-app-fail");
        let layout = a_clean_pc(dir.path());
        let temp = dir.path().join("temp");
        fs::create_dir_all(&temp).unwrap();
        let rename = |_: &Path, _: &Path| Err(std::io::Error::other("refused"));
        let done = remove_app_with(&layout, &layout.exe(), &temp, &rename).unwrap();
        assert_eq!(done.exe_left.as_deref(), Some(layout.exe().as_path()));
        assert!(done.moved_exe.is_none() && names_in(&temp).is_empty());
        assert_eq!(names_in(&layout.programs), [EXE_NAME]);
        assert!(!layout.desktop_link().exists());
    }

    /// Every file under `root` with its bytes, and every folder.
    fn snapshot(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
        crate::package::walk(root)
            .unwrap()
            .into_iter()
            .map(|(rel, kind)| {
                let bytes = (kind == crate::package::Kind::File).then(|| fs::read(join_relative(root, &rel)).unwrap());
                (rel, bytes)
            })
            .collect()
    }

    #[test]
    fn only_the_installed_copy_can_remove_the_install() {
        let dir = TempDir::new("remove-notme");
        let layout = a_pc(dir.path());
        let temp = dir.path().join("temp");
        fs::create_dir_all(&temp).unwrap();
        // A copy beside the install's staging folder, as a stopped install leaves one.
        let staged = layout.staging().join(EXE_NAME);
        fs::create_dir_all(layout.staging()).unwrap();
        fs::copy(layout.exe(), &staged).unwrap();
        let before = snapshot(dir.path());
        let others = [
            dir.path().join("studio").join("target").join("release").join(EXE_NAME),
            dir.path().join("ballfight-pack.exe"),
            layout.programs.join(".previous").join(EXE_NAME),
            staged.clone(),
            PathBuf::from(EXE_NAME),
            layout.programs.clone(),
        ];
        for other in others {
            let err = remove_app_with(&layout, &other, &temp, &plain_rename)
                .unwrap_err()
                .to_string();
            assert_eq!(
                err,
                format!(
                    "This copy of BALLFIGHT Studio is not the installed one in {}, so nothing was removed.",
                    layout.programs.display()
                ),
                "{}",
                other.display()
            );
        }
        assert_eq!(snapshot(dir.path()), before, "every file is as it was, byte for byte");
        assert!(names_in(&temp).is_empty());
    }

    #[test]
    fn a_removal_that_could_not_finish_can_be_run_again() {
        let dir = TempDir::new("remove-again");
        let layout = a_clean_pc(dir.path());
        let held_path = layout.programs.join(".previous").join("guide").join("index.html");
        let held = hold(&held_path);
        let stayed = remove_now(&removal_plan(&layout).unwrap());
        drop(held);
        assert!(stayed.iter().any(|(p, _)| p == &held_path), "{stayed:?}");
        // The manifests of the folders that still hold something of ours stay, so the plan works again.
        assert!(layout.programs.join(MANIFEST_NAME).is_file());
        assert!(layout.programs.join(".previous").join(MANIFEST_NAME).is_file());
        let again = removal_plan(&layout).unwrap();
        assert!(again.kept.is_empty(), "{:?}", again.kept);
        assert!(remove_now(&again).is_empty());
        assert!(!layout.programs.exists());
    }

    #[test]
    fn a_changed_exe_or_a_held_file_stays() {
        let dir = TempDir::new("remove-app-changed");
        let layout = a_clean_pc(dir.path());
        let temp = dir.path().join("temp");
        fs::create_dir_all(&temp).unwrap();
        fs::write(layout.exe(), b"changed since it was installed").unwrap();
        let held = hold(&layout.programs.join("guide").join("index.html"));
        let done = remove_app_with(&layout, &layout.exe(), &temp, &plain_rename).unwrap();
        drop(held);
        assert!(done.moved_exe.is_none() && done.exe_left.is_none() && names_in(&temp).is_empty());
        assert_eq!(fs::read(layout.exe()).unwrap(), b"changed since it was installed");
        assert!(layout.programs.join("guide").join("index.html").is_file());
        let words = removed_words(&done).join(" ");
        assert!(
            words.contains("ballfight-studio.exe") && words.contains("index.html"),
            "{words}"
        );
        // The refusals are those of the plan: nothing is touched.
        let mut other = layout.clone();
        other.programs = layout.programs.with_file_name("Something");
        assert!(remove_app_with(&other, &other.exe(), &temp, &plain_rename).is_err());
    }

    /// What `a_pc` made, except the person's file (which `a_clean_pc` took away), is as it was.
    fn everything_else_stays_but_my_notes(root: &Path, layout: &Layout) {
        fs::create_dir_all(&layout.programs).unwrap();
        fs::write(layout.programs.join("my notes.txt"), b"mine").unwrap();
        everything_else_stays(root, layout);
        fs::remove_file(layout.programs.join("my notes.txt")).unwrap();
        fs::remove_dir(&layout.programs).unwrap();
    }

    /// The child side of `a_running_app_removes_itself`: runs only when that test starts this program from the
    /// installed folder.
    #[test]
    #[ignore = "the child side of a_running_app_removes_itself"]
    fn child_side_of_removing_itself() {
        let (Some(root), Some(temp)) = (
            std::env::var_os("BF_CHILD_REMOVE_ROOT"),
            std::env::var_os("BF_CHILD_REMOVE_TEMP"),
        ) else {
            return;
        };
        let layout = Layout::under(Path::new(&root));
        let me = std::env::current_exe().unwrap();
        let done = remove_app_with(&layout, &me, Path::new(&temp), &plain_rename).unwrap();
        for w in removed_words(&done) {
            println!("{w}");
        }
    }

    #[test]
    fn a_running_app_removes_itself() {
        let dir = TempDir::new("remove-self");
        // A package whose exe is a copy of this test program, so the installed exe can really run.
        let inputs = dir.path().join("inputs");
        let (exe, tree, checked) = crate::package::tests::fake_inputs(&inputs, "0.6.0");
        fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        let pkg = dir.path().join("package");
        crate::package::package(&exe, &tree, &checked, &pkg).unwrap();
        let layout = Layout::under(&dir.path().join("pc"));
        install(&pkg, &layout, &config_for(&layout)).unwrap();
        shortcuts(&layout).unwrap();
        let temp = dir.path().join("temp");
        fs::create_dir_all(&temp).unwrap();
        // Started through the one way to start a program, with a bounded wait: stopped (and failed) after 2 minutes.
        let root = dir.path().join("pc");
        let never = std::sync::atomic::AtomicBool::new(false);
        let ran = crate::run::run_with_env(
            &layout.exe(),
            &[
                OsStr::new("--exact"),
                OsStr::new("remove::tests::child_side_of_removing_itself"),
                OsStr::new("--ignored"),
                OsStr::new("--nocapture"),
                OsStr::new("--test-threads=1"),
            ],
            &[
                ("BF_CHILD_REMOVE_ROOT", root.as_os_str()),
                ("BF_CHILD_REMOVE_TEMP", temp.as_os_str()),
            ],
            std::time::Duration::from_secs(120),
            &never,
        )
        .unwrap_or_else(|e| panic!("the child: {e}"));
        let said = String::from_utf8_lossy(&ran.stdout);
        assert!(ran.status.success(), "{said}\n{:?}", ran.tail);
        assert!(said.contains("BALLFIGHT Studio was removed from"), "{said}");
        assert!(said.contains("was moved to"), "{said}");
        assert!(!layout.programs.exists(), "the running app's whole folder went");
        assert!(!layout.start_menu_link().exists() && !layout.desktop_link().exists());
        let moved = names_in(&temp);
        assert_eq!(moved.len(), 1, "{moved:?}");
        assert!(moved[0].starts_with("BALLFIGHT Studio removed "), "{moved:?}");
    }

    #[test]
    fn the_words_say_what_goes_and_what_stays() {
        let dir = TempDir::new("remove-words");
        let layout = a_pc(dir.path());
        let r = removal_plan(&layout).unwrap();
        let words = removal_words(&layout, &r);
        let all = words.join(" ");
        assert!(all.contains("its Start menu and desktop shortcuts"), "{all}");
        assert!(
            all.contains("videos") && all.contains("settings") && all.contains("logs"),
            "{all}"
        );
        let build = layout.data.join("build").display().to_string();
        let downloads = layout.data.join("downloads").display().to_string();
        assert!(
            all.contains(&format!("delete {build} and {downloads} yourself")),
            "{all}"
        );
        assert!(all.contains("my notes.txt"), "{all}");
        for w in &words {
            assert!(w.ends_with('.'), "{w}");
            assert!(w.chars().next().unwrap().is_uppercase(), "{w}");
        }
        assert!(!all.contains(LINK_NAME));
        // A shortcut that opens something else is not one of "its" shortcuts.
        let other = crate::install::lnk::bytes(
            Path::new(r"C:\Windows\notepad.exe"),
            Path::new(r"C:\Windows"),
            Path::new(r"C:\Windows\notepad.exe"),
            "x",
        )
        .unwrap();
        fs::write(layout.desktop_link(), other).unwrap();
        let all = removal_words(&layout, &removal_plan(&layout).unwrap()).join(" ");
        assert!(all.contains("and its Start menu shortcut,"), "{all}");
        fs::remove_file(layout.start_menu_link()).unwrap();
        let all = removal_words(&layout, &removal_plan(&layout).unwrap()).join(" ");
        assert!(!all.contains("shortcut"), "{all}");
    }

    #[test]
    fn a_stopped_install_is_still_removed_by_its_own_manifests() {
        let dir = TempDir::new("remove-stopped");
        let layout = Layout::under(&dir.path().join("pc"));
        let old = package_of(dir.path(), "0.6.0");
        let new = crate::install::tests::package_with_extra(dir.path(), "0.6.1");
        install(&old, &layout, &config_for(&layout)).unwrap();
        // Stopped after the earlier files moved out and some new ones moved in.
        assert!(crate::install::tests::install_stopped_after(&new, &layout, 16).is_err());
        assert!(layout.staging().exists());
        let r = removal_plan(&layout).unwrap();
        assert!(r.kept.is_empty(), "{:?}", r.kept);
        let stayed = remove_now(&r);
        assert!(stayed.is_empty(), "{stayed:?}");
        assert!(!layout.programs.exists() && !layout.staging().exists());
    }
}
