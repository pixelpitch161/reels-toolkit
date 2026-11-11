//! Removing the app from this PC (plan M6, Task 11; "Remove BALLFIGHT Studio from this PC" in Settings, Task 13).
//!
//! **Only what an install wrote goes**, file by file, from the manifest the install wrote: each file `MANIFEST.txt`
//! lists whose SHA-256 still matches, the manifest itself, the install's `update.toml`, the same inside `.previous`
//! and `.next` by their own manifests, the two shortcuts when they open the installed exe, and then each folder
//! that is left empty. Nothing is ever removed by deleting a folder with what is in it.
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
//! The app cannot remove its own running exe, so `schedule_removal` starts one hidden Windows PowerShell with a
//! fixed script (`REMOVAL_SCRIPT`, passed with `-EncodedCommand`), which waits for the app to end and then works
//! through a list file this module wrote: it checks each file's SHA-256 again just before removing it, never
//! follows a link, and removes a folder only when it is empty. The paths never appear in the script or on its
//! command line; they come from the list file named in an environment variable.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{CONFIG_NAME, read_config};
use crate::install::{Layout, check_layout, opens_the_app};
use crate::package::{MANIFEST_NAME, is_link, join_relative, read_manifest};
use crate::sha::sha256_file;
use crate::{PackError, io_error};

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

/// The first line of a removal list file.
pub const LIST_HEADER: &str = "BALLFIGHT Studio removal list 1";

/// The removal program: one fixed Windows PowerShell script. It reads the after-pid and the list file's path from
/// `BF_REMOVE_AFTER_PID` and `BF_REMOVE_LIST`, waits up to `BF_REMOVE_WAIT` seconds (60) for that process to end
/// (if it has not, it removes only its own list file and stops), then for each `F<tab>sha256<tab>path` line
/// removes the file only if it is a plain file (not a folder, not a link) whose SHA-256 is still that, and for each
/// `D<tab>path` line removes the folder only if it is a real folder and empty (`Directory.Delete` without recursion
/// fails on any other). Last it removes the list file. It writes nothing anywhere else. Exit 2: bad values; 3: the
/// app did not end; 4: not our list.
pub const REMOVAL_SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$list = $env:BF_REMOVE_LIST
$id = 0
$wait = 0
if (-not [int]::TryParse($env:BF_REMOVE_AFTER_PID, [ref]$id)) { exit 2 }
if (-not [int]::TryParse($env:BF_REMOVE_WAIT, [ref]$wait)) { exit 2 }
try { Wait-Process -Id $id -Timeout $wait -ErrorAction SilentlyContinue } catch { }
if (Get-Process -Id $id -ErrorAction SilentlyContinue) { [System.IO.File]::Delete($list); exit 3 }
$lines = [System.IO.File]::ReadAllLines($list, [System.Text.Encoding]::UTF8)
if ($lines.Length -lt 1 -or $lines[0] -cne 'BALLFIGHT Studio removal list 1') { exit 4 }
$reparse = [System.IO.FileAttributes]::ReparsePoint
for ($i = 1; $i -lt $lines.Length; $i++) {
  $parts = $lines[$i].Split([char]9)
  try {
    if ($parts.Length -eq 3 -and $parts[0] -ceq 'F') {
      $item = Get-Item -LiteralPath $parts[2] -Force
      $plain = (-not $item.PSIsContainer) -and (($item.Attributes -band $reparse) -eq 0)
      if ($plain -and ((Get-FileHash -LiteralPath $parts[2] -Algorithm SHA256).Hash -eq $parts[1])) {
        [System.IO.File]::Delete($parts[2])
      }
    } elseif ($parts.Length -eq 2 -and $parts[0] -ceq 'D') {
      $item = Get-Item -LiteralPath $parts[1] -Force
      if ($item.PSIsContainer -and (($item.Attributes -band $reparse) -eq 0)) {
        [System.IO.Directory]::Delete($parts[1], $false)
      }
    }
  } catch { }
}
[System.IO.File]::Delete($list)
exit 0
"#;

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
    plan_folder(p, &mut r)?;
    Ok(r)
}

/// Plans the removal of one folder that holds a manifest of ours (the program folder, `.previous`, `.next`),
/// adding to `r`: each listed file whose SHA-256 still matches, `update.toml` when it is one of ours, the manifest,
/// then the folders deepest first and `dir` itself. Everything else in it is added to `r.kept`, and links are never
/// entered.
pub(crate) fn plan_folder(dir: &Path, r: &mut Removal) -> Result<(), PackError> {
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
                if !is_link(&own) && own.is_file() && plan_folder(&path, &mut inner).is_ok() {
                    r.files.append(&mut inner.files);
                    r.dirs.append(&mut inner.dirs);
                    r.kept.append(&mut inner.kept);
                } else {
                    for (inner_rel, inner_kind) in walk(&path)? {
                        if inner_kind != Kind::Dir {
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
            Kind::File => {
                let ours_config = rel == CONFIG_NAME && read_config(&path).is_ok();
                let listed = m.files.iter().find(|(p, _, _)| p == &rel);
                let got = || sha256_file(&path).map_err(|e| io_error("read", &path, &e));
                match (ours_config, listed) {
                    (true, _) => r.files.push((path.clone(), got()?)),
                    (false, Some((_, _, sha))) => {
                        let now = got()?;
                        if &now == sha {
                            r.files.push((path, now));
                        } else {
                            r.kept.push((path, "it has changed since it was installed".to_string()));
                        }
                    }
                    (false, None) => r.kept.push((path, "BALLFIGHT Studio did not put it there".to_string())),
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

/// Removes what `r` plans, here and now (for folders whose files no running program holds), checking each file's
/// SHA-256 again just before removing it. Gives what stayed, with why.
pub fn remove_now(r: &Removal) -> Vec<(PathBuf, String)> {
    let mut stayed = r.kept.clone();
    for (f, sha) in &r.files {
        if fs::symlink_metadata(f).is_err() {
            continue;
        }
        if is_link(f) || !f.is_file() {
            stayed.push((f.clone(), "it is no longer a plain file".to_string()));
            continue;
        }
        match sha256_file(f) {
            Ok(now) if &now == sha => {
                if let Err(e) = fs::remove_file(f) {
                    stayed.push((f.clone(), format!("it could not be removed: {e}")));
                }
            }
            _ => stayed.push((f.clone(), "it has changed since the removal was planned".to_string())),
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
    plan_folder(dir, &mut r)?;
    Ok(remove_now(&r))
}

/// Removes the app from this layout after the process `after_pid` (the app itself) has ended: plans the removal,
/// writes the list file in the temp folder, and starts the removal program, which goes on by itself. The app should
/// close right after this returns `Ok`.
pub fn schedule_removal(layout: &Layout, after_pid: u32) -> Result<(), PackError> {
    let r = removal_plan(layout)?;
    // Handed off on purpose: the removal program outlives the app, which is what it waits for.
    let child = start_removal(&r, after_pid, &std::env::temp_dir())?;
    drop(child);
    Ok(())
}

/// The list file's text: the header, then `F<tab>sha256<tab>path` for each file and `D<tab>path` for each folder.
fn list_text(r: &Removal) -> Result<String, PackError> {
    let mut s = format!("{LIST_HEADER}\r\n");
    let text = |p: &Path| {
        p.to_str()
            .filter(|t| !t.chars().any(char::is_control))
            .map(str::to_string)
            .ok_or_else(|| PackError::Refused(format!("{} has a name the removal cannot write down.", p.display())))
    };
    for (f, sha) in &r.files {
        s.push_str(&format!("F\t{sha}\t{}\r\n", text(f)?));
    }
    for d in &r.dirs {
        s.push_str(&format!("D\t{}\r\n", text(d)?));
    }
    Ok(s)
}

/// Writes the list file for `r` in `list_dir` and starts the removal program, which waits up to `wait_secs` for
/// `after_pid` to end. Gives the running program (not waited for by `schedule_removal`).
pub(crate) fn start_removal_waiting(
    r: &Removal,
    after_pid: u32,
    list_dir: &Path,
    wait_secs: u32,
) -> Result<std::process::Child, PackError> {
    let ps = crate::install::powershell()?;
    let text = list_text(r)?;
    // A new file each time (create_new), named after this process and the time, so nothing is written over.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let list = list_dir.join(format!("BALLFIGHT Studio removal {}-{stamp}.txt", std::process::id()));
    {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&list)
            .map_err(|e| io_error("make", &list, &e))?;
        f.write_all(text.as_bytes())
            .and_then(|()| f.sync_all())
            .map_err(|e| io_error("write", &list, &e))?;
    }
    let script = crate::install::encoded_command(REMOVAL_SCRIPT);
    let o = std::ffi::OsStr::new;
    let mut c = crate::run::quiet(
        &ps,
        &[
            o("-NoLogo"),
            o("-NoProfile"),
            o("-NonInteractive"),
            o("-EncodedCommand"),
            o(&script),
        ],
    );
    // Nothing is read from it, and it may outlive the app: no pipes to a process that is gone.
    c.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .env("BF_REMOVE_LIST", &list)
        .env("BF_REMOVE_AFTER_PID", after_pid.to_string())
        .env("BF_REMOVE_WAIT", wait_secs.to_string());
    c.spawn().map_err(|e| {
        let _ = fs::remove_file(&list);
        PackError::Io(format!("The removal could not be started: {e}."))
    })
}

/// `start_removal_waiting` with the 60 s wait.
pub(crate) fn start_removal(r: &Removal, after_pid: u32, list_dir: &Path) -> Result<std::process::Child, PackError> {
    start_removal_waiting(r, after_pid, list_dir, 60)
}

/// What Settings says before a removal (Task 13): what goes, what stays, and anything of the person's that stays in
/// the program folder. Whole plain sentences.
pub fn removal_words(layout: &Layout, r: &Removal) -> Vec<String> {
    let mut words = vec![
        format!(
            "Removing takes away the program in {} and its two shortcuts, once BALLFIGHT Studio has closed.",
            layout.programs.display()
        ),
        "Your videos, your settings, the queue, fault reports and the app's other files in your user folder all \
         stay on this PC."
            .to_string(),
    ];
    if let Some(data_root) = layout.data.parent() {
        words.push(format!(
            "The app's downloads and update builds in {} stay too; delete that folder yourself to free the space.",
            data_root.display()
        ));
    }
    let names: Vec<String> = r
        .kept
        .iter()
        .filter(|(p, _)| p.starts_with(&layout.programs))
        .filter_map(|(p, _)| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    if !names.is_empty() {
        words.push(format!(
            "These files in the program folder were not put there by BALLFIGHT Studio, or have changed, so they \
             stay: {}.",
            names.join(", ")
        ));
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
    use std::time::{Duration, Instant};

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

    /// The pid of a program that has already ended.
    fn an_ended_pid() -> u32 {
        let ping = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE");
        let mut child = crate::run::quiet(&ping, &[OsStr::new("-n"), OsStr::new("1"), OsStr::new("127.0.0.1")])
            .spawn()
            .unwrap();
        let id = child.id();
        child.wait().unwrap();
        id
    }

    #[test]
    fn the_removal_program_takes_only_the_plan() {
        let dir = TempDir::new("remove-script");
        let layout = a_pc(dir.path());
        let r = removal_plan(&layout).unwrap();
        // Changed after the plan was made: the program checks the hash again, so this one stays.
        fs::write(layout.programs.join("ballfight.ico"), b"changed after the plan").unwrap();
        // Held open: it cannot be removed, the program goes on with the rest.
        let held = hold(&layout.programs.join("guide").join("index.html"));
        let list_dir = dir.path().join("lists");
        fs::create_dir_all(&list_dir).unwrap();
        let mut child = start_removal(&r, an_ended_pid(), &list_dir).unwrap();
        let status = child.wait().unwrap();
        drop(held);
        assert!(status.success(), "{status}");
        assert_eq!(names_in(&layout.programs), ["ballfight.ico", "guide", "my notes.txt"]);
        assert_eq!(names_in(&layout.programs.join("guide")), ["index.html"]);
        assert_eq!(
            fs::read(layout.programs.join("ballfight.ico")).unwrap(),
            b"changed after the plan"
        );
        assert!(!layout.start_menu_link().exists() && !layout.desktop_link().exists());
        everything_else_stays(dir.path(), &layout);
        assert!(names_in(&list_dir).is_empty(), "the list file is removed at the end");
    }

    #[test]
    fn the_removal_program_waits_for_a_program_that_does_not_end() {
        let dir = TempDir::new("remove-wait");
        let layout = a_pc(dir.path());
        let r = removal_plan(&layout).unwrap();
        let list_dir = dir.path().join("lists");
        fs::create_dir_all(&list_dir).unwrap();
        // The list names a pid that is running (this test itself): with a 1 s wait the program gives up and
        // removes nothing.
        let mut child = start_removal_waiting(&r, std::process::id(), &list_dir, 1).unwrap();
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(3));
        assert!(
            names_in(&list_dir).is_empty(),
            "the list file is removed when nothing is done"
        );
        assert!(layout.exe().is_file() && layout.desktop_link().is_file());
    }

    #[test]
    #[ignore = "starts a program that runs for about 3 s, then waits up to 10 s; run by name"]
    fn removal_happens_after_the_app_ends() {
        let dir = TempDir::new("remove-after");
        let layout = a_pc(dir.path());
        let ping = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE");
        // Stands in for the app: about 2 s.
        let mut app = crate::run::quiet(&ping, &[OsStr::new("-n"), OsStr::new("3"), OsStr::new("127.0.0.1")])
            .spawn()
            .unwrap();
        let r = removal_plan(&layout).unwrap();
        let list_dir = dir.path().join("lists");
        fs::create_dir_all(&list_dir).unwrap();
        let mut removal = start_removal(&r, app.id(), &list_dir).unwrap();
        std::thread::sleep(Duration::from_millis(1000));
        assert!(layout.exe().is_file(), "nothing goes while the app runs");
        app.wait().unwrap();
        let ended = Instant::now();
        while layout.exe().exists() && ended.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(!layout.exe().exists(), "the program folder's files went within 10 s");
        assert!(removal.wait().unwrap().success());
        assert_eq!(names_in(&layout.programs), ["my notes.txt"]);
        everything_else_stays(dir.path(), &layout);
        assert!(dir.path().join("pc").join("config").join("settings.toml").is_file());
    }

    #[test]
    fn the_words_say_what_goes_and_what_stays() {
        let dir = TempDir::new("remove-words");
        let layout = a_pc(dir.path());
        let r = removal_plan(&layout).unwrap();
        let words = removal_words(&layout, &r);
        let all = words.join(" ");
        assert!(all.contains("two shortcuts"), "{all}");
        assert!(all.contains("videos") && all.contains("settings"), "{all}");
        assert!(all.contains("my notes.txt"), "{all}");
        for w in &words {
            assert!(w.ends_with('.'), "{w}");
            assert!(w.chars().next().unwrap().is_uppercase(), "{w}");
        }
        assert!(!all.contains(LINK_NAME) || all.contains("BALLFIGHT Studio"));
    }
}
