//! The swap on restart (plan M6, Task 12). "Restart now" starts the staged exe, `.next\ballfight-studio.exe
//! --finish-update <pid>`, and the app closes; that program (`finish`, which Task 9 calls from `main`) waits for the
//! old app to end, puts `.next` in place with `bf_pack::install::put_in_place` (the install's own swap: the marker,
//! only the files the installed manifest lists move into a fresh `.previous`, the new files in, the new manifest
//! last; any failed step undoes every step), and starts the app again from the program folder. "Go back" turns
//! `.previous` into `.next` and does the same. Every swap, and every start's look at a stopped one, holds the swap
//! lock beside the program folder, so two never run at once.
//!
//! **The exe is always at the shortcut's target**, whole: the swap keeps a link of the earlier exe in `.previous`
//! and puts the new one over it in one replacing rename (see `bf_pack::install`). So a double-click always starts
//! either the earlier version or the new one, and that start (`at_start`) finishes or takes back the update.
//!
//! **A power cut, step by step** (what is on disk, and what a double-click on the shortcut does then):
//! - while `.next` is being made: `.next` holds `PACKING.txt`; the running version is untouched. A double-click starts
//!   the earlier version as usual; the next build clears `.next` by that list (`stage::clear_next`).
//! - after `.next` is made, before "Restart now": nothing has changed. A double-click starts the earlier version, and
//!   its look offers "Restart now" again after checking `.next` once more.
//! - during the swap, before the new exe is in place: the earlier exe starts; `at_start` sees the marker, cannot
//!   finish under its own running exe, and returns `Resume` once: `main` starts `.next`'s exe to finish and closes, and
//!   the new version starts. If that finish fails too, the next start takes the update back (`undo_stopped`) and
//!   says so: the earlier version runs, nothing lost.
//! - during the swap, after the new exe is in place: the new exe starts; `at_start` finishes the swap in this program
//!   and says so. If it cannot, it takes the update back and says so.
//! - after the swap's last step, before the marker is gone: the new exe starts; `at_start` removes the marker.
//! - during "Go back", between turning `.previous` into `.next` and the swap: the current exe starts; `at_start` turns
//!   `.next` back into `.previous`.
//! - while another program holds the swap lock (a swap is under way right now): `at_start` gives `Busy`, and `main`
//!   says `UPDATE_BUSY` and closes.

use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, Instant};

use bf_pack::PackError;
use bf_pack::install::{
    CLOSE_FIRST, SWAP_BUSY, put_in_place, put_in_place_held, swap_lock, swap_unfinished, undo_stopped,
};
use bf_pack::package::{EXE_NAME, MANIFEST_NAME, check_kept_package, read_manifest};
use bf_pack::version::parse_version;

use super::stage::{NEXT, check_named, clear_next};
use super::{Installed, restart_failed, sentence, swap_failed};

/// The option the app's `main` answers by running `finish` (Task 9): `--finish-update <pid>`.
pub const FINISH_FLAG: &str = "--finish-update";
/// The folder the version before the last update is kept in.
pub const PREVIOUS: &str = ".previous";
/// The note a failed swap leaves in the logs folder for the next start to show.
pub const NOTICE_NAME: &str = "update-notice.txt";
/// The note that a start has already handed a stopped swap to `.next`'s exe once, so it is not handed over again.
pub const RESUMED_NAME: &str = "update-resumed.txt";
/// What a start says, and then closes, while another program is putting a version in place.
pub const UPDATE_BUSY: &str = SWAP_BUSY;
/// How long `finish` waits for the old app to end.
const OLD_APP_WAIT: Duration = Duration::from_secs(30);

/// Starts `.next`'s exe with `--finish-update <this program's id>` in the program folder, after checking `.next`
/// is a whole package. The app saves and closes right after (Task 13 sees `UpdateView::close`).
pub fn launch_finish(inst: &Installed) -> Result<(), String> {
    let next = inst.dir.join(NEXT);
    check_kept_package(&next).map_err(|e| restart_failed(&e.to_string()))?;
    let pid = std::process::id().to_string();
    bf_pack::run::start_detached(
        &next.join(EXE_NAME),
        &[OsStr::new(FINISH_FLAG), OsStr::new(&pid)],
        &inst.dir,
    )
    .map(|_| ())
    .map_err(|e| restart_failed(&e))
}

/// `--finish-update <old_pid>`, run by the new version from `<dir>\.next`: the swap, then the app started again from
/// `dir`. On failure the earlier version is back in place and started, and `logs\update-notice.txt` holds the
/// sentence the next start shows. Task 9 exits with 0 for `Ok` and 1 for `Err`.
pub fn finish(dir: &Path, old_pid: u32) -> Result<(), String> {
    let logs = crate::store::Paths::system().map(|p| p.logs);
    finish_with(dir, old_pid, logs.as_deref(), OLD_APP_WAIT, &|exe| {
        bf_pack::run::start_detached(exe, &[], dir).map(|_| ())
    })
}

/// Leaves `notice` for the next start (best effort).
fn leave_notice(logs: Option<&Path>, notice: &str, why: &str) {
    if let Some(logs) = logs {
        let _ = std::fs::create_dir_all(logs);
        let _ = std::fs::write(logs.join(NOTICE_NAME), format!("{notice}\r\n{why}\r\n"));
    }
}

/// `finish`, with the logs folder, the wait and the start of the app given (tests).
///
/// When the old app has not ended within `wait` (or its files stay held, or another swap holds the lock), nothing is
/// put in place and **nothing is started**: the old app is still there (its window stays as it was), `.next` stays
/// staged, and the next start shows why and offers "Restart now" again.
pub fn finish_with(
    dir: &Path,
    old_pid: u32,
    logs: Option<&Path>,
    wait: Duration,
    start: &dyn Fn(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let next = dir.join(NEXT);
    let version_of = |d: &Path| {
        read_manifest(&d.join(MANIFEST_NAME))
            .map(|m| m.version)
            .unwrap_or_else(|_| "unknown".to_string())
    };
    let (old, new) = (version_of(dir), version_of(&next));
    let still_open = || {
        format!(
            "Version {old} did not close in time, so the update to version {new} was not put in place yet. Close \
             BALLFIGHT Studio, start it again, and choose Restart now."
        )
    };
    let deadline = Instant::now() + wait;
    if !bf_pack::run::wait_for_exit(old_pid, wait) {
        let notice = still_open();
        leave_notice(logs, &notice, "The earlier version was still running.");
        return Err(notice);
    }
    // The old app's files may stay held a moment after it ends (or by a copy started meanwhile): try again until
    // the wait is over.
    let put = loop {
        match put_in_place(dir, &next) {
            Err(PackError::Refused(t)) if (t == CLOSE_FIRST || t == SWAP_BUSY) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(250));
            }
            other => break other,
        }
    };
    let exe = dir.join(EXE_NAME);
    match put {
        Ok(_) => start(&exe).map_err(|e| {
            format!(
                "Version {new} is in place, but it could not be started. {} Start BALLFIGHT Studio from its \
                 shortcut.",
                sentence(&e)
            )
        }),
        Err(PackError::Refused(t)) if t == CLOSE_FIRST || t == SWAP_BUSY => {
            let notice = still_open();
            leave_notice(logs, &notice, &t);
            Err(notice)
        }
        Err(e) => {
            let notice = swap_failed(&new, &old);
            leave_notice(logs, &notice, &e.to_string());
            let _ = start(&exe);
            Err(format!("{notice} {e}"))
        }
    }
}

/// What the start-up should do about updates (Task 9 calls `at_start` early in `main`, for an installed copy).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtStart {
    /// Nothing to do.
    Nothing,
    /// Show this sentence as a notice.
    Notice(String),
    /// A swap is unfinished and this program (the earlier version) holds its exe: call `resume` and exit. Given at
    /// most once for a stopped swap; the start after that takes the update back.
    Resume,
    /// Another program is putting a version in place right now: show `UPDATE_BUSY` and exit.
    Busy,
}

/// Puts right what a stopped swap or "Go back" left (see the module notes), removes an empty `.next`, and gives the
/// notice a failed swap left in `logs`, once.
pub fn at_start(inst: &Installed, logs: Option<&Path>) -> AtStart {
    let dir = &inst.dir;
    let next = dir.join(NEXT);
    let Ok(lock) = swap_lock(dir) else {
        return AtStart::Busy;
    };
    let resumed = logs.map(|l| l.join(RESUMED_NAME));
    let forget_resume = || {
        if let Some(r) = &resumed {
            let _ = std::fs::remove_file(r);
        }
    };
    if swap_unfinished(dir) {
        let version_of = |d: &Path| read_manifest(&d.join(MANIFEST_NAME)).map(|m| m.version).ok();
        let new = version_of(&next).unwrap_or_else(|| "unknown".to_string());
        match put_in_place_held(dir, &next, &lock) {
            Ok(m) => {
                forget_resume();
                return AtStart::Notice(format!("The update to version {} is finished.", m.version));
            }
            Err(PackError::Refused(t))
                if t == CLOSE_FIRST
                    && next.join(EXE_NAME).is_file()
                    && resumed.as_ref().is_some_and(|r| !r.exists()) =>
            {
                if let Some(r) = &resumed {
                    let _ = std::fs::create_dir_all(r.parent().unwrap_or(r));
                    if std::fs::write(r, format!("{new}\r\n")).is_ok() {
                        return AtStart::Resume;
                    }
                }
            }
            Err(_) => {}
        }
        // It could not be finished (or it was handed over once already): take it back.
        return match undo_stopped(dir, &next, &lock) {
            Ok(old) => {
                forget_resume();
                AtStart::Notice(format!(
                    "The update to version {new} could not be finished, so it was taken back: this is still version \
                     {}. Nothing was lost.",
                    old.version
                ))
            }
            Err(e) => AtStart::Notice(format!(
                "An update was stopped part-way and could not be finished or taken back yet. {} Start BALLFIGHT \
                 Studio again to try once more.",
                sentence(&e.to_string())
            )),
        };
    }
    forget_resume();
    if std::fs::symlink_metadata(&next).is_ok() {
        // Only empty folders: gone. (`clear_unfinished` removes nothing else without its own list.)
        let empty = walk_is_only_folders(&next);
        if empty {
            let _ = bf_pack::package::clear_unfinished(&next);
        } else if std::fs::symlink_metadata(dir.join(PREVIOUS)).is_err()
            && bf_pack::config::read_config(&next.join(bf_pack::config::CONFIG_NAME)).is_ok()
        {
            // "Go back" stopped before its swap (only a kept version carries a copy of update.toml; a staged
            // update never does): the earlier version is kept again.
            let _ = std::fs::rename(&next, dir.join(PREVIOUS));
        }
    }
    drop(lock);
    match logs.and_then(take_notice) {
        Some(n) => AtStart::Notice(n),
        None => AtStart::Nothing,
    }
}

/// `AtStart::Resume`: starts `.next`'s exe again to finish the swap; the app then exits.
pub fn resume(inst: &Installed) -> Result<(), String> {
    let pid = std::process::id().to_string();
    bf_pack::run::start_detached(
        &inst.dir.join(NEXT).join(EXE_NAME),
        &[OsStr::new(FINISH_FLAG), OsStr::new(&pid)],
        &inst.dir,
    )
    .map(|_| ())
}

fn walk_is_only_folders(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.into_iter().all(|e| {
        e.is_ok_and(|e| {
            let p = e.path();
            !bf_pack::package::is_link(&p) && p.is_dir() && walk_is_only_folders(&p)
        })
    })
}

/// The sentence a failed swap left in `logs` (its first line), removed once read.
pub fn take_notice(logs: &Path) -> Option<String> {
    let path = logs.join(NOTICE_NAME);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    text.lines().next().map(str::to_string).filter(|l| !l.is_empty())
}

/// The version kept in `.previous`, when one is kept whole (no swap under way): older than the running one after an
/// update ("Go back"), newer after going back ("Go forward").
pub fn previous_version(dir: &Path) -> Option<String> {
    if swap_unfinished(dir) {
        return None;
    }
    read_manifest(&dir.join(PREVIOUS).join(MANIFEST_NAME))
        .ok()
        .map(|m| m.version)
}

/// The words of the button for the kept version: "Go back to version W" when it is older than `running`, "Go
/// forward to version W" when it is newer (after going back).
pub fn kept_button(kept: &str, running: &str) -> String {
    match (parse_version(kept), parse_version(running)) {
        (Some(k), Some(r)) if k > r => format!("Go forward to version {kept}"),
        _ => format!("Go back to version {kept}"),
    }
}

/// "Go back to version W" (or forward, after going back): `.previous` becomes `.next` (a staged update waiting is
/// given up first), its exe must say its version, then `launch_finish`. If any step fails, `.previous` is put back
/// and nothing changed. The kept version is used as it is: nothing is built again.
pub fn go_back(inst: &Installed) -> Result<(), String> {
    go_back_with(inst, &launch_finish)
}

/// `go_back`, with the launch given (tests).
pub fn go_back_with(inst: &Installed, launch: &dyn Fn(&Installed) -> Result<(), String>) -> Result<(), String> {
    let dir = &inst.dir;
    let failed = |why: &str| {
        format!(
            "Switching to the kept version did not start, so nothing changed. {}",
            sentence(why)
        )
    };
    let lock = swap_lock(dir).map_err(|e| failed(&e.to_string()))?;
    if swap_unfinished(dir) {
        return Err(failed(
            "An update is still being put in place; start BALLFIGHT Studio again first",
        ));
    }
    let prev = dir.join(PREVIOUS);
    let next = dir.join(NEXT);
    let kept = read_manifest(&prev.join(MANIFEST_NAME)).map_err(|_| failed("No other version is kept"))?;
    clear_next(dir).map_err(|e| failed(&e))?;
    std::fs::rename(&prev, &next).map_err(|e| failed(&format!("{} could not be renamed: {e}", prev.display())))?;
    let put_back = || {
        let _ = std::fs::rename(&next, &prev);
    };
    // It keeps its copy of update.toml until the swap (which keeps the one in place): that copy is what tells a
    // version brought back from a staged update, should the PC stop before the swap.
    if let Err(e) = check_named(&next, &kept.version, None, true) {
        put_back();
        return Err(failed(&e));
    }
    // The finishing program takes the lock itself once this app has closed.
    drop(lock);
    if let Err(e) = launch(inst) {
        put_back();
        return Err(failed(&e));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::stage::tests::{installed, snapshot};
    use super::super::testkit::TempDir;
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    fn ping() -> PathBuf {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE")
    }

    /// Stages version `version` (its exe a copy of `exe`) into the install's `.next`, as `stage` does.
    fn staged(root: &Path, inst: &Installed, version: &str, exe: &Path) {
        let tree = root.join(format!("tree-{version}"));
        super::super::git::standin::write(&tree, version, &super::super::git::standin::Kind::default());
        let checked = super::super::git::standin::checked_ffmpeg(&root.join("ff"));
        super::super::stage::stage_with(inst, exe, &tree, &|_| Ok(checked.clone())).unwrap();
    }

    fn pretend(root: &Path, text: &str) -> PathBuf {
        let p = root.join(format!("pretend-{}", text.len())).join(EXE_NAME);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    type Seen = Mutex<Vec<PathBuf>>;

    /// A start that only records what it was asked to start.
    fn recorder() -> (Seen, impl Fn(&Seen, &Path) -> Result<(), String>) {
        (Mutex::new(Vec::new()), |seen: &Seen, p: &Path| {
            seen.lock().unwrap().push(p.to_path_buf());
            Ok(())
        })
    }
    /// A process id that is not running (this test program's parent may be; a fresh ping that has ended is not).
    fn ended_pid() -> u32 {
        let pid = bf_pack::run::start_detached(
            &ping(),
            &[OsStr::new("-n"), OsStr::new("1"), OsStr::new("127.0.0.1")],
            &std::env::temp_dir(),
        )
        .unwrap();
        assert!(bf_pack::run::wait_for_exit(pid, Duration::from_secs(10)));
        pid
    }

    #[test]
    fn the_swap_moves_and_keeps_previous() {
        let dir = TempDir::new("swap-moves");
        let inst = installed(dir.path(), "0.6.0", None);
        std::fs::write(inst.dir.join("my notes.txt"), b"mine").unwrap();
        let config_before = std::fs::read(inst.dir.join("update.toml")).unwrap();
        staged(dir.path(), &inst, "0.6.1", &pretend(dir.path(), "pretend exe 0.6.1"));
        let logs = dir.path().join("logs");
        let (seen, rec) = recorder();
        finish_with(&inst.dir, ended_pid(), Some(&logs), Duration::from_secs(5), &|p| {
            rec(&seen, p)
        })
        .unwrap();
        assert_eq!(std::fs::read(inst.dir.join(EXE_NAME)).unwrap(), b"pretend exe 0.6.1");
        assert_eq!(read_manifest(&inst.dir.join(MANIFEST_NAME)).unwrap().version, "0.6.1");
        assert_eq!(std::fs::read(inst.dir.join("update.toml")).unwrap(), config_before);
        assert_eq!(std::fs::read(inst.dir.join("my notes.txt")).unwrap(), b"mine");
        assert!(!inst.dir.join(NEXT).exists());
        assert_eq!(previous_version(&inst.dir).as_deref(), Some("0.6.0"));
        assert_eq!(
            std::fs::read(inst.dir.join(PREVIOUS).join(EXE_NAME)).unwrap(),
            b"pretend exe 0.6.0"
        );
        check_package_but_config(&inst.dir.join(PREVIOUS));
        assert_eq!(
            *seen.lock().unwrap(),
            [inst.dir.join(EXE_NAME)],
            "the app is started again from its folder"
        );
        assert!(!logs.join(NOTICE_NAME).exists());
        assert_eq!(at_start(&inst, Some(&logs)), AtStart::Nothing);
    }

    /// `.previous` holds a whole package plus the copy of update.toml kept with it.
    fn check_package_but_config(dir: &Path) {
        let m = read_manifest(&dir.join(MANIFEST_NAME)).unwrap();
        for (rel, bytes, sha) in &m.files {
            let f = bf_pack::package::join_relative(dir, rel);
            assert_eq!(std::fs::metadata(&f).unwrap().len(), *bytes, "{rel}");
            assert_eq!(&bf_pack::sha::sha256_file(&f).unwrap(), sha, "{rel}");
        }
    }

    #[test]
    fn a_failed_swap_puts_everything_back() {
        let dir = TempDir::new("swap-fails");
        let inst = installed(dir.path(), "0.6.0", None);
        staged(dir.path(), &inst, "0.6.1", &pretend(dir.path(), "pretend exe 0.6.1"));
        let before = snapshot(&inst.dir);
        let logs = dir.path().join("logs");
        // A file of the installed version held open by the test (as a scanner might), so it cannot move.
        let held_path = inst.dir.join("ffmpeg").join("ffprobe.exe");
        let held = hold(&held_path);
        let (seen, rec) = recorder();
        let err = finish_with(&inst.dir, ended_pid(), Some(&logs), Duration::from_secs(2), &|p| {
            rec(&seen, p)
        })
        .unwrap_err();
        drop(held);
        assert!(
            err.starts_with(
                "The update to version 0.6.1 could not be put in place, so this is still version 0.6.0. Nothing was \
                 lost."
            ),
            "{err}"
        );
        assert_eq!(
            snapshot(&inst.dir),
            before,
            "everything is back as it was, .next included"
        );
        assert_eq!(
            *seen.lock().unwrap(),
            [inst.dir.join(EXE_NAME)],
            "the earlier version is started again"
        );
        // The next start shows the notice, once.
        assert_eq!(
            at_start(&inst, Some(&logs)),
            AtStart::Notice(
                "The update to version 0.6.1 could not be put in place, so this is still version 0.6.0. Nothing was \
                 lost."
                    .to_string()
            )
        );
        assert_eq!(at_start(&inst, Some(&logs)), AtStart::Nothing);
    }

    fn hold(path: &Path) -> std::fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_SHARE_READ only: no rename or delete while this is open.
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0x1)
            .open(path)
            .unwrap()
    }

    #[test]
    fn go_back_swaps_previous_in() {
        let dir = TempDir::new("swap-back");
        // Real programs that answer --version as the app does: the stand-in, built once for each version.
        let Some(exes) = standin_exes(dir.path(), &["0.6.0", "0.6.1"]) else {
            return;
        };
        let inst = installed(dir.path(), "0.6.0", Some(&exes[0]));
        staged(dir.path(), &inst, "0.6.1", &exes[1]);
        let (seen, rec) = recorder();
        finish_with(&inst.dir, ended_pid(), None, Duration::from_secs(5), &|p| rec(&seen, p)).unwrap();
        assert_eq!(previous_version(&inst.dir).as_deref(), Some("0.6.0"));
        // Go back: .previous becomes .next and the finishing program is started; the swap then runs as an update.
        let launched = Mutex::new(0);
        go_back_with(&inst, &|_| {
            *launched.lock().unwrap() += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(*launched.lock().unwrap(), 1);
        assert!(!inst.dir.join(PREVIOUS).exists());
        check_kept_package(&inst.dir.join(NEXT)).unwrap();
        finish_with(&inst.dir, ended_pid(), None, Duration::from_secs(5), &|p| rec(&seen, p)).unwrap();
        assert_eq!(read_manifest(&inst.dir.join(MANIFEST_NAME)).unwrap().version, "0.6.0");
        assert_eq!(
            std::fs::read(inst.dir.join(EXE_NAME)).unwrap(),
            std::fs::read(&exes[0]).unwrap()
        );
        assert_eq!(previous_version(&inst.dir).as_deref(), Some("0.6.1"));
        assert!(!inst.dir.join(NEXT).exists());

        // A launch that fails puts .previous back.
        let before = snapshot(&inst.dir);
        let err = go_back_with(&inst, &|_| Err("it would not start".into())).unwrap_err();
        assert_eq!(
            err,
            "Switching to the kept version did not start, so nothing changed. It would not start."
        );
        assert_eq!(snapshot(&inst.dir), before);
        // A "Go back" stopped after the rename (a power cut) is turned back at the next start.
        std::fs::rename(inst.dir.join(PREVIOUS), inst.dir.join(NEXT)).unwrap();
        assert_eq!(at_start(&inst, None), AtStart::Nothing);
        assert_eq!(previous_version(&inst.dir).as_deref(), Some("0.6.1"));
    }

    /// The stand-in built once for each version (into a temp target folder), or `None` without git and cargo.
    fn standin_exes(root: &Path, versions: &[&str]) -> Option<Vec<PathBuf>> {
        let cargo = super::super::git::find_cargo()?;
        let mut out = Vec::new();
        for v in versions {
            let tree = root.join(format!("exe-tree-{v}"));
            super::super::git::standin::write(&tree, v, &super::super::git::standin::Kind::default());
            let target = root.join(format!("exe-target-{v}"));
            let never = std::sync::atomic::AtomicBool::new(false);
            let how = bf_pack::run::Family {
                dir: Some(&tree.join("studio")),
                env: &[("CARGO_TARGET_DIR", target.as_os_str())],
                env_remove: &["RUSTUP_TOOLCHAIN"],
                scrub_git: true,
                below_normal: true,
            };
            let ran = bf_pack::run::run_family(
                &cargo,
                &[
                    OsStr::new("build"),
                    OsStr::new("--release"),
                    OsStr::new("-p"),
                    OsStr::new("bf-app"),
                ],
                &how,
                Duration::from_secs(300),
                &never,
                &mut |_| {},
            )
            .ok()?;
            assert!(ran.status.success(), "{:?}", ran.tail);
            out.push(target.join("release").join(EXE_NAME));
        }
        Some(out)
    }

    #[test]
    fn a_stopped_swap_is_finished_at_start() {
        let dir = TempDir::new("swap-start");
        let inst = installed(dir.path(), "0.6.0", None);
        staged(dir.path(), &inst, "0.6.1", &pretend(dir.path(), "pretend exe 0.6.1"));
        // Stopped as a power cut would: the marker and the earlier exe already in .previous.
        let prev = inst.dir.join(PREVIOUS);
        std::fs::create_dir(&prev).unwrap();
        std::fs::write(
            prev.join("INSTALLING"),
            "BALLFIGHT Studio is putting a new version in place.\r\n",
        )
        .unwrap();
        std::fs::copy(inst.dir.join(MANIFEST_NAME), prev.join(MANIFEST_NAME)).unwrap();
        std::fs::rename(inst.dir.join(EXE_NAME), prev.join(EXE_NAME)).unwrap();
        assert!(swap_unfinished(&inst.dir));
        assert_eq!(
            previous_version(&inst.dir),
            None,
            "no going back while a swap is under way"
        );
        assert_eq!(
            at_start(&inst, None),
            AtStart::Notice("The update to version 0.6.1 is finished.".to_string())
        );
        assert!(!swap_unfinished(&inst.dir));
        assert_eq!(std::fs::read(inst.dir.join(EXE_NAME)).unwrap(), b"pretend exe 0.6.1");
        assert_eq!(previous_version(&inst.dir).as_deref(), Some("0.6.0"));
        // An empty .next left behind is removed at start.
        std::fs::create_dir_all(inst.dir.join(NEXT).join("ffmpeg")).unwrap();
        assert_eq!(at_start(&inst, None), AtStart::Nothing);
        assert!(!inst.dir.join(NEXT).exists());
    }

    #[test]
    fn the_earlier_version_still_running_resumes_through_next() {
        let dir = TempDir::new("swap-resume");
        // The installed exe is a real program (a copy of ping) that runs during the start-up check.
        let inst = installed(dir.path(), "0.6.0", Some(&ping()));
        staged(dir.path(), &inst, "0.6.1", &pretend(dir.path(), "pretend exe 0.6.1"));
        let prev = inst.dir.join(PREVIOUS);
        std::fs::create_dir(&prev).unwrap();
        std::fs::write(
            prev.join("INSTALLING"),
            "BALLFIGHT Studio is putting a new version in place.\r\n",
        )
        .unwrap();
        let mut child = bf_pack::run::quiet(
            &inst.dir.join(EXE_NAME),
            &[OsStr::new("-n"), OsStr::new("9"), OsStr::new("127.0.0.1")],
        )
        .spawn()
        .unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let logs = dir.path().join("logs");
        let said = at_start(&inst, Some(&logs));
        assert_eq!(said, AtStart::Resume);
        assert!(
            swap_unfinished(&inst.dir),
            "nothing was moved under the running program"
        );
        // The finishing program failed as well (here: it never ran), and the earlier version starts again: no second
        // hand-over. The update is taken back, the earlier version runs, and the words say so.
        let said = at_start(&inst, Some(&logs));
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(
            said,
            AtStart::Notice(
                "The update to version 0.6.1 could not be finished, so it was taken back: this is still version \
                 0.6.0. Nothing was lost."
                    .to_string()
            )
        );
        assert!(!swap_unfinished(&inst.dir));
        assert_eq!(
            std::fs::read(inst.dir.join(EXE_NAME)).unwrap(),
            std::fs::read(ping()).unwrap()
        );
        check_kept_package(&inst.dir.join(NEXT)).unwrap();
        // The next start has nothing to do, and a later update can hand over again.
        assert_eq!(at_start(&inst, Some(&logs)), AtStart::Nothing);
        assert!(!logs.join(RESUMED_NAME).exists());
    }

    #[test]
    fn a_start_while_a_swap_runs_steps_aside() {
        let dir = TempDir::new("swap-busy");
        let inst = installed(dir.path(), "0.6.0", None);
        staged(dir.path(), &inst, "0.6.1", &pretend(dir.path(), "pretend exe 0.6.1"));
        let held = swap_lock(&inst.dir).unwrap();
        assert_eq!(at_start(&inst, None), AtStart::Busy);
        assert!(go_back_with(&inst, &|_| Ok(())).unwrap_err().contains(UPDATE_BUSY));
        drop(held);
        assert_eq!(at_start(&inst, None), AtStart::Nothing);
    }

    #[test]
    fn an_old_app_that_does_not_close_is_left_alone() {
        let dir = TempDir::new("swap-wait");
        let inst = installed(dir.path(), "0.6.0", None);
        staged(dir.path(), &inst, "0.6.1", &pretend(dir.path(), "pretend exe 0.6.1"));
        let before = snapshot(&inst.dir);
        // A program that keeps running stands for the old app.
        let mut child = bf_pack::run::quiet(&ping(), &[OsStr::new("-n"), OsStr::new("9"), OsStr::new("127.0.0.1")])
            .spawn()
            .unwrap();
        let logs = dir.path().join("logs");
        let (seen, rec) = recorder();
        let err = finish_with(&inst.dir, child.id(), Some(&logs), Duration::from_millis(500), &|p| {
            rec(&seen, p)
        })
        .unwrap_err();
        let _ = child.kill();
        let _ = child.wait();
        assert!(err.starts_with("Version 0.6.0 did not close in time"), "{err}");
        assert!(
            seen.lock().unwrap().is_empty(),
            "nothing is started while the old app may still be open"
        );
        assert_eq!(snapshot(&inst.dir), before, "nothing moved; .next stays staged");
        assert!(take_notice(&logs).is_some_and(|n| n.starts_with("Version 0.6.0 did not close in time")));
    }
}
