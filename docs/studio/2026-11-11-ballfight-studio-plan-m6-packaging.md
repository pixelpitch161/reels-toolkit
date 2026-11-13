# BALLFIGHT Studio, milestone 6 (packaging): Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the Studio into a program the owner installs once, starts with a double-click, keeps up to date with
a click, and can remove: its own pinned ffmpeg, an Update button that works, start-up that never crashes silently, a
short user guide with pictures of the real app, a Windows CI that builds the shipped package, and handoff notes for
later sessions (spec section 14, item 6).

**Architecture:** One new crate, `studio/crates/bf-pack` (library and the `ballfight-pack` binary), holds everything
about files on disk outside the app's own data: the ffmpeg pin and its verified download, the package folder, the
install, the shortcuts and removal. `bf-app` gains six modules (`version`, `report`, `startup`, `icon`, `guide`,
`update`) that start out as plain logic with unit tests, and are wired into `main.rs`, `app.rs` and the Settings
screen by three late tasks only (9, 10 and 13), so this milestone touches the files that milestones 3 to 5 and the
last app task also touch as little and as late as it can. `bf-export` learns to prefer the app's own ffmpeg and to offer one faster encoder
setting whose default output stays byte for byte what it is today.

**Tech Stack:** As pinned in `studio/Cargo.toml` (Rust 1.99.1, `x86_64-pc-windows-gnu`; egui and eframe 0.37.0;
`windows-sys` 0.61.2; `sha2`, `zip`, `toml`, `serde`, `png`, `chrono`, `directories`). **No new crates from
crates.io**: downloads go through Windows' own `curl.exe` (`%SystemRoot%\System32\curl.exe`, part of Windows 11),
shortcuts through Windows PowerShell's `WScript.Shell`, version comparison is a small function of our own.

**Spec:** `docs/studio/2026-11-10-ballfight-studio-design.md` (section 14 item 6; sections 2, 4, 9, 11, 13 and 16
bear on it). **Form and conventions:** `docs/studio/2026-11-10-ballfight-studio-plan-m2-app.md` (called "the M2 plan"
below); its Global Constraints still bind every task that touches `bf-app`.

**Scope:** Packaging only. No fighter, theme, fight rule, picture or sound changes. Tasks 17 to 19 need the final
content of milestones 3 to 5 and are marked **LAST**.

## Where things stand (read on Nov 11 from the code as merged on `lane/sound`, a842e11)

- `bf-app` builds `ballfight-studio.exe` with `windows_subsystem = "windows"` in release (no console window). Without
  `--fixtures` it still shows "The engine is not connected in this build": **M2 Task 35 (the real app end to end)
  is not merged**, and the build's ledger records no result for M2 Tasks 31 and 32 either. Every task here that
  edits `main.rs`, `app.rs` or the workflow waits for M2 Task 35.
- Data lives where `store::Paths::system()` says: `%APPDATA%\BALLFIGHT Studio\config` (`settings.toml`,
  `queue.json`) and `%LOCALAPPDATA%\BALLFIGHT Studio\cache` (`shortlists\`). Videos go to `Videos\BALLFIGHT` by
  default. A first start with no files uses the defaults and says nothing.
- The saved files are versioned: `settings.toml` has `format = 1`, a higher number is moved aside as
  `settings.newer-N.toml`, unknown keys are ignored (the parser reads only `format`, `output_folder`, `workers`);
  `queue.json` has `format = 1` and `app_version`, and a queue saved by another version with videos waiting gives a
  notice; the shortlist cache is keyed by app version, so another version is a miss; fight files have `format = 1`
  and the Library refuses a newer format in words.
- ffmpeg is found by `bf_export::find_ffmpeg`: `BALLFIGHT_FFMPEG`, then the first `ffmpeg.exe` on `PATH`, with
  ffprobe beside it; it must list `libx264` and `aac`. On the owner's PC that is **ffmpeg 9.0.2, the
  `full_build-www.gyan.dev` build installed by winget** (`...\WinGet\Links\ffmpeg.exe`), configured with
  `--enable-gpl --enable-version3 --enable-libx264`, so it is GPL version 3. The encode arguments are pinned by
  `encode.rs`'s `the_arguments_are_the_kits` (libx264 `slow`, CRF 18, the 360 x 640 frames piped and scaled up
  3x by ffmpeg); the mp4's bytes do not depend on the workers (`the_video_does_not_depend_on_the_workers`, ruling
  R42). No test pins the mp4's bytes against an ffmpeg version.
- CI is one Windows job (`.github/workflows/studio.yml`): build `bf-cli`, test `bf-core` and `bf-sim`, install
  ffmpeg with Chocolatey (unpinned) and check its encoders, `render_is_reproducible`, render a 3 s clip.
- The toolbar's "Update" is drawn disabled ("Updates from inside the app arrive in a later version."); Settings says
  "{version}. The Update button starts working in a later version." and, without ffmpeg, "ffmpeg was not found.
  Install ffmpeg, then start the app again. A later version brings its own."
- Left open by milestone 2 for this one (the M2 ledger): the folder dialog has no parent window and can open behind
  the app; abandoned `.tmp-` folders in the output folder are never cleared (and `bf-export` keeps a *finished*
  video in its hidden `.tmp-` folder when the final name was taken meanwhile, which is why nothing may simply delete
  them); start-up errors go to stderr, which a release build does not show; the debug exe is 485 MB, so only release
  is ever shipped; the window draws through DirectX 12 alone when the PC has a DirectX 12 card (M2-R27); emoji in
  captions draw in colour from Windows' own emoji font. `docs/studio/m1-measurements.md` section 12 leaves "make the
  kit's replay guard Windows-safe" to this milestone.
- The repo's `main` is 321 commits behind the build branch that every lane merges into (`origin/HEAD` points at it).
- On the owner's PC today: Git for Windows (`C:\Program Files\Git\cmd\git.exe`), already signed in to GitHub through
  its credential manager (the build has pushed); rustup with `1.99.1-x86_64-pc-windows-gnu`; `windres.exe` from a
  winget MinGW package on `PATH` (not part of the Rust toolchain); no `gh`.

## What success looks like

**The PC state this plan delivers on:** the owner's Windows 11 PC with Git for Windows signed in to GitHub and rustup
installed (both there today, and needed only by the recommended update route), and an internet connection for
updates. No ffmpeg, no Python, nothing else.

1. A build session installs the app once (Task 19). From then on the owner double-clicks **BALLFIGHT Studio** on the
   desktop or in the Start menu: the window opens with its icon, no console window flashes, and a second
   double-click brings the open window to the front instead of starting a second copy.
2. The owner makes a video (match-up, balance, seed, music, render) and it lands in `Videos\BALLFIGHT`, encoded by
   the app's own ffmpeg 9.0.2, byte for byte the same mp4 the PC's ffmpeg makes today for the same fight.
3. When a new version exists, the toolbar's Update button says so; the owner clicks it, then "Build version X" and,
   when it is ready, "Restart now". The app comes back at the new version with its settings, queue and Library.
   "Go back to version W" undoes one update.
4. A clean start (no settings), a PC without DirectX 12, a missing ffmpeg, an output folder that cannot be written
   and a fault in the app each give a screen in plain words, never a silent exit; a fault leaves a report file the
   owner can be asked to send.
5. "Remove BALLFIGHT Studio from this PC" in Settings removes the program and its shortcuts; videos and settings stay.
6. Nothing at any point asks the owner to type a command. Every request to the owner is a click.

## Global Rules

Every task's requirements include this section, and the M2 plan's Global Constraints for `bf-app`.

- **Nothing under `projects/`, `reelkit/` or `series/` is edited.** `git status --short projects reelkit series` is
  empty at the end of every task. GUITAR and SAX never appear. No model or AI names in commits, code or docs. Commit
  trailers follow the build's standing ruling R7.
- **The owner never runs a command.** Anything the app or the guide asks of the owner is a click. The guide holds no
  command line. Messages are plain words in sentence case, buttons say what happens (M2 wording rules); the
  `wording` test covers every new text the app shows.
- **Credentials are never touched by this build.** No task, test or script reads, prints, stores, copies or searches
  for a token, password, credential file, credential-manager entry or `.git-credentials`; no test sends anything to
  GitHub. Git is only ever run as a program that handles its own sign-in. If a step's `git` command opens a sign-in
  window, the worker stops and reports it: completing a sign-in is the owner's click, never a worker's. A remote URL
  that holds a user name or token (`https://name:...@...`) is refused by the code that would save it, without
  printing it.
- **The default video stays byte for byte the same.** `encode_args` for the default setting is unchanged
  (`the_arguments_are_the_kits` passes unedited), `studio/replay/render_hashes.txt` is unchanged, every golden and
  parity test passes unchanged, and the new pinned-encode test (Task 3) passes. A task that would change any of
  these stops and reports.
- **CI checks are kept.** No step, test or check is removed, weakened, skipped, marked `continue-on-error` or newly
  `#[ignore]`d to make CI pass. Replacing where CI's ffmpeg comes from keeps its encoder check (Task 14).
- **Tests never download and never touch the owner's folders.** Every test works in a temp folder, with fake
  downloads (a local file) and fake programs; a test that needs the network is `#[ignore]`d with the reason and run
  by name. Tests never write to `%APPDATA%`, `%LOCALAPPDATA%`, the desktop, the Start menu or `Videos`.
- **Programs are started one way.** Every child program (ffmpeg, git, cargo, curl, explorer, powershell, the app's
  own exe) is started directly with its arguments one by one, never pasted into a command line, with
  `CREATE_NO_WINDOW` (no console window), its error output read on a thread, and always waited for or handed off
  on purpose. `bf-pack::run::quiet` is that one helper; a source-scan test (Task 1) fails on any other
  `std::process::Command::new` in `bf-pack` and the new `bf-app` modules. The one fixed script (shortcuts, Task 11)
  is a constant in the code and takes its values from environment variables, never from pasted text.
- **Windows only, honestly.** The program only ever runs on the owner's Windows PC. Windows calls sit behind
  `#[cfg(windows)]` with a plain fallback (a function that says "only on Windows"), so `cargo check --workspace`
  stays clean.
- **Only release builds are shipped.** The debug exe is 485 MB; nothing debug is ever packaged.
- **Tests first** where code is involved, as in the M2 plan: step 1 writes the failing tests, step 2 runs them and
  sees them fail. `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean for the
  crates touched. Commits are per task step 5, made by path, with the subject given.

## The decisions

Each has its options, a recommendation, and what stays the owner's. **No task depends on the owner's answers**: the
recommended choice is built, and every alternative is a separate task or a one-line change, named below.

### D1. How the app updates itself (the owner's decision; recommended: A)

Both routes share everything after "a new exe exists": the check in the toolbar, the staging folder, the check that
the new exe starts and names the expected version, the swap on restart, "Go back", and the wording (Task 12, 13).
They differ only in where the new exe comes from (`update::Source`).

**A. Build on this PC** (the spec's route: "`git pull` then a release build").

- *How:* fetch the newest `studio-v*` tag into the app's own copy of the source, then `cargo build --release
  --locked -p bf-app` there.
- *Needs on the PC:* Git (signed in), rustup, about 5 to 10 GB for the build folder. All there today.
- *Needs elsewhere:* nothing.
- *How long:* the fetch a few seconds; the build is measured in Task 12. Expected: a few minutes after a small
  change; 10 to 20 minutes the first time, or after a toolchain change, when rustup downloads the new toolchain by
  itself.
- *Credentials:* **none in the app.** Git signs in by itself through the credential manager already set up; the app
  never sees a token.
- *Verification:* the exe is built from the tagged commit's source on this PC, `--locked` to `Cargo.lock` (cargo
  checks every crates.io package's checksum); the new exe must report the tag's version and commit before it
  replaces the old one.
- *What can go wrong:* a build error (the old version keeps running and the message says so); rustup or crates.io
  offline; the disk full; the build slows a render running at the same time (it runs at below-normal priority).

**B. Build in the cloud, download** (how the owner's other program works).

- *How:* a CI job builds the package for each `studio-v*` tag and attaches it to a GitHub Release; the app downloads
  the zip and its checksum file.
- *Needs on the PC:* Git signed in (for the token, below). No Rust.
- *Needs elsewhere:* the release job with its Release step (Task 14, Task B1); Windows CI minutes for every release
  (Windows minutes count double on a private repo); `contents: write` for that one job.
- *How long:* a download about the size of the package without ffmpeg (measured in Task 11), plus ffmpeg only when
  its pin changes: under a minute on broadband.
- *Credentials:* the repo is private, so every API call and download needs a token. The app would ask git's
  credential manager for it (`git credential fill`) at each check, keep it in memory only, send it only to
  `api.github.com` and GitHub's download host, and never write or log it. A token of the app's own in a plain file is
  never an option. (The other ways, a token the owner makes on github.com or a public releases repo, are many clicks
  or make the program public.)
- *Verification:* the zip's SHA-256 must match the release's `SHA256SUMS.txt`, and the new exe must report the tag's
  version and commit. This protects against a broken or partial download, not against someone who controls the repo
  (neither route does). GitHub's build attestations may not be offered for private repos on the owner's plan, so
  they are not relied on.
- *What can go wrong:* CI red, so no release; the token expired (git's sign-in window opens: one click by the
  owner); GitHub down.

**Recommendation: A.** Everything it needs is already on the PC and signed in, the app never handles a credential, no
CI minutes are spent, and it is what the spec says. B is the stated alternative: it is Task B1 plus the release
job's Release step (Task 14), and nothing else in this plan changes if the owner picks it later.

**Release tags (part of D1, both routes).** A version reaches the owner only when the controller pushes a tag
`studio-v<version>` (for example `studio-v0.6.0`, or `studio-v0.6.0-rc.1` for a candidate) on a commit whose CI is
green, after bumping `[workspace.package] version` to that version. Updates follow tags, not a branch, so `main`
being behind the build branch does not matter. The app updates to the newest tag above its own version; a
pre-release (`-rc.N`) is offered only to an app that is itself a pre-release of the same version or older.

### D2. Installer, or a plain folder and shortcuts (recommended: plain folder)

- **A plain per-user folder and two shortcuts** (`%LOCALAPPDATA%\Programs\BALLFIGHT Studio`, a Start menu and a
  desktop shortcut). For: no admin prompt; no extra tools; an update replaces files in a folder the app owns; removal
  is "delete the folder". Against: no entry in Windows' Apps list (removal is a button in Settings instead).
- **An installer** (MSI through WiX, Inno Setup or NSIS). For: an Apps-list entry, a familiar wizard. Against: new
  build tools on the PC and in CI; every update would have to run the installer again.

**Recommendation: the plain folder** (Task 11). Nothing stays undecided; an Apps-list entry can be added later
(a per-user registry key) if the owner asks.

### D3. Bundling ffmpeg (recommended: bundle the exact build the PC uses today)

- The video needs libx264, which is GPL, so **only a GPL build will do**; an LGPL build has no libx264.
- **Pin:** ffmpeg 9.0.2, `full_build` from www.gyan.dev, the same archive winget installed on the owner's PC, so the
  mp4s keep today's bytes (the mp4's bytes depend on the ffmpeg build, and winget can move the PC's own copy to a new
  version at any time). The `essentials_build` of the same version is about half the size and also GPL; it may be
  used instead only if Task 3's pinned clip hash comes out the same with it.
- **Where the files come from:** `studio/packaging/ffmpeg.toml` names the archive's URL, size and SHA-256 and each
  needed file's SHA-256. `bf-pack fetch-ffmpeg` downloads it into a cache folder outside the repo (CI: the runner's
  temp folder under `actions/cache`; the PC: `%LOCALAPPDATA%\BALLFIGHT Studio\data\downloads`), checks every
  checksum, and copies out `ffmpeg.exe`, `ffprobe.exe` and the licence. **The archive and the exes are never
  committed.**
- **Licence:** GPL version 3 (`--enable-gpl --enable-version3`). The app starts ffmpeg as a separate program and talks
  to it through pipes and arguments, which keeps the Studio's own code a separate program, not a work based on
  ffmpeg. What ships beside it: `ffmpeg\LICENSE.txt` (the GPL text from the archive, unchanged) and
  `ffmpeg\ABOUT.txt` (version, build, the archive's URL and SHA-256, where its source code is published, and one line
  saying the Studio runs it as a separate program). This is a private program for one person on one PC, so nobody
  else receives a copy; if it is ever given to anyone, the GPL's duty to offer ffmpeg's source applies, and ABOUT.txt
  says where that source is.
- Order of finding ffmpeg after this milestone: `BALLFIGHT_FFMPEG` (tests and CI), then the app's own
  `ffmpeg\ffmpeg.exe` beside the exe, then `PATH` (a developer's build in `target\release`).

### D4. A faster encoder setting (the owner's choice to make in Settings; recommended: offer it, default unchanged)

`m1-measurements.md` shows the encode is what keeps a render at about 82 s. The kit's settings stay the default
("Best quality", libx264 `slow`). Settings gains one more choice, "Faster", which changes only the x264 preset; CRF
18, the profile, `tune animation`, `mbtree=0`, the forced keyframes, the sound and the container stay. Which preset
"Faster" is gets measured in Task 4 by a stated rule. It is offered **only because** the default's bytes can be kept:
`the_arguments_are_the_kits` (unchanged), the new pinned clip hash for the default (Task 3), and
`render_is_reproducible` guard that. Every video, fast or not, still passes all 15 checks before it is saved. The
setting is not part of the fight file (it changes no frame and no sample); it applies to a job when the job starts.

### D5. The kit's replay guard on Windows (recommended: fix it, Task 16)

`tools/ballfight_replay_hash.py` drops the kit-episode lines on Windows (it compares paths with forward slashes). It
is a development tool, not something the owner runs, but kit sessions now happen on this Windows PC. The fix is in
`tools/`, not `projects/`. If the controller prefers to leave it, skip Task 16; nothing else depends on it.

### D6. The kit README's note (constraint, not a choice)

The spec says that once the port is complete "HANDOFF.md and the kit README are updated" to say Rust is the source of
truth. The kit README is `projects/ballfight/README.md`, and nothing under `projects/` is ever edited by this build.
So Task 18 writes the note in `docs/studio/HANDOFF.md` and `studio/README.md` only, and lists the kit README line as
a one-line change that a session makes only on the owner's word.

### D7. Smaller choices made here

Each with what it costs if it is wrong.

- **a.** One copy at a time: a second start brings the first window forward and exits. Copies with another
  `--data-dir` are separate (the lock is named after the settings folder). *If wrong:* remove one call in `main.rs`.
- **b.** The window title stays "BALLFIGHT Studio"; the version, with its commit and date, is in Settings. *If
  wrong:* add it to the title, one line.
- **c.** Version 1.0.0 at this milestone's release if milestones 3 to 5 are complete, else 0.6.0. *If wrong:* another
  number in Task 19.
- **d.** The icon is drawn in code from the app's greys (two touching discs on `ground`), not a fighter colour, since
  colour belongs to fighters. *If wrong:* redraw `icon::rgba` and bless one file.
- **e.** The exe file itself shows Windows' generic program icon in Explorer: embedding an icon needs `windres`,
  which the Rust toolchain does not ship (this PC has one from a separate MinGW package; CI may not). The shortcuts,
  the window and the taskbar show the app's icon. *If wrong:* add an `embed-resource` build step once `windres` is
  certain on both.
- **f.** Removing the app also deletes the app's rebuildable data (`data\source`, `data\build`, `data\downloads`,
  `cache`) and keeps settings, the queue, fault reports and every video. *If wrong:* keep more, one list.
- **g.** (Ruling M6-R4, replacing the Recycle Bin.) When the Library is scanned, a day-old abandoned `.tmp-` render
  folder is removed file by file, under strict rules: only the export's own file names, then the empty folder.
  Such a folder holds only a partial render. One recursive shell operation could follow whatever it finds, show
  a window, and fill the bin with gigabytes. A `.tmp-` folder that holds a finished `.mp4` is never touched, and
  from Task 10 on a finished video is never left in one. *If wrong:* turn it off, one call.

## Order of work

**Lanes.** As in milestones 1 and 2: each task in its own clone under `.lanes/`, on its own branch, merged by the
controller after review. Milestones 3 to 5 run at the same time in their own lanes.

| Task | Needs merged first | Can start now? |
|---|---|---|
| 1 ffmpeg pin and fetcher (`bf-pack`) | nothing | **yes** |
| 2 bf-export prefers the app's own ffmpeg | nothing | **yes** |
| 5 The app's new modules and its version | nothing | **yes** |
| 15 Saved files across versions | nothing | **yes** |
| 16 Replay guard on Windows (D5) | nothing | **yes** |
| 3 The encode is pinned | 1 | after 1 |
| 6 Fault reports | 5 | after 5 |
| 7 Start-up guards | 5 | after 5 |
| 8 The icon | 5 | after 5 |
| 11 Install, shortcuts and removal | 1, 8 | after 1 and 8 |
| 4 The faster encoder setting (engine and command line) | 2, 3 | after 2 and 3 |
| 12 The update engine | 5, 11 | after 5 and 11 |
| 9 Start-up wiring | 6, 7, 8, 12, **M2 Task 35** | waits for M2 Task 35 |
| 10 The folder dialog in front; what a render leaves behind | 9 | waits for M2 Task 35 |
| 14 CI: the pinned ffmpeg, the pinned encode, the package job | 1, 3, 11, **M2 Task 35** | waits for M2 Task 35 |
| 13 Settings and the toolbar | 4, 6, 9, 12 | waits (see the overlap table) |
| 17 Guide pictures and the guide | 13, **milestones 3 to 5 complete** | **LAST** |
| 18 Handoff docs | 17 | **LAST** |
| 19 The release and the release checklist | everything above | **LAST** |
| B1 Download route (only if the owner picks D1 B) | 12, 14 | only on request |

**What can run at the same time** (no two write the same file): 1, 2, 5, 15 and 16 at once (five clones); then 3,
6, 7 and 8; then 4, 11; then 12. After M2 Task 35: 9, then 10 and 14 side by side, then 13. 17, 18, 19 one after
another at the end.

**Files these tasks share with M2 Task 35 and milestones 3 to 5**, and how the conflict is avoided:

- **`bf-app/src/main.rs`, `src/app.rs`** (also M2 Task 35): Task 9 makes every `main.rs` change of this milestone
  in one task; Task 10 adds one line to `app.rs`. Both only after M2 Task 35 is merged, 9 before 10.
- **`.github/workflows/studio.yml`** (also M2 Task 35, which adds the app's build and tests; and milestone 3 if its
  reference storage moves to Git LFS, which changes the checkout to `lfs: true`): Task 14 only, after M2 Task 35, and
  never at the same time as a milestone 3 task that edits the workflow. The controller runs one workflow-editing task
  at a time.
- **`bf-app/src/seam/mod.rs`, `src/state/mod.rs`, `src/state/queue_run.rs`, `src/model/mod.rs`** (also milestone 5:
  the Music screen's controls and actions): Task 13 only (one field on `RunJob`, the view model's new fields, new
  actions). It runs while no milestone 5 task has these files open; otherwise it waits for that merge and rebases.
  It only adds; it never renames.
- **`bf-app/src/engine/jobs.rs`** (no other milestone): Tasks 2, 4 and 13, in that order.
- **`bf-app/src/screens/settings.rs`, `src/screens/shell.rs`, `src/fixtures/settings.rs`, `tests/ui/settings.rs`,
  `tests/snapshots/settings_*.png`** (no other milestone): Task 13 only, which holds every Settings and toolbar
  change of this milestone. The shell picture and every other screen's pictures stay as they are.
- **`bf-app/tests/ui/wording.rs`** (milestones 3 to 5 add names, which the rule already allows): Task 13 adds a word
  only if one is needed.
- **`studio/Cargo.toml`, `Cargo.lock`** (any milestone that adds a crate): Task 1 adds one member. Small; rebase.
- **`bf-export/src/encode.rs`, `job.rs`, `output.rs`** (not expected elsewhere: milestones 3 to 5 change
  `bf-content`, `bf-sim`, `bf-picture`, `bf-sound`, `bf-story`): Tasks 2, 4, 10, in that order.
- **`bf-cli/tests/e2e.rs`, `bf-cli/src/cmd/{render,bench}.rs`** (milestone 3 may add end-to-end tests): Tasks 3 and
  4 add only.
- **`docs/studio/HANDOFF.md`, `studio/README.md`** (every milestone's last task): Task 18, **LAST**.

## File map

```
studio/Cargo.toml  Cargo.lock                 member "crates/bf-pack" (Task 1)
studio/packaging/
  ffmpeg.toml                                 the ffmpeg pin (Task 1)
  guide/index.html  guide/pictures/*.png      the user guide (Task 17)
studio/crates/bf-pack/
  Cargo.toml  README.md
  src/lib.rs  run.rs  sha.rs  ffmpeg.rs  fetch.rs  package.rs  install.rs  remove.rs  config.rs  version.rs  main.rs
  tests/scan.rs                               the one-way-to-start-programs scan
studio/crates/bf-export/src/encode.rs  job.rs  output.rs                  (Tasks 2, 4, 10)
studio/crates/bf-cli/src/cmd/render.rs  bench.rs  tests/e2e.rs            (Tasks 3, 4)
studio/replay/encode_hashes.txt                                          (Task 3)
studio/crates/bf-app/
  build.rs                                    commit and date (Task 5)
  assets/icon/ballfight.ico                   (Task 8)
  src/version.rs  report.rs  startup.rs  icon.rs  guide.rs
  src/update/mod.rs  git.rs  stage.rs  swap.rs  worker.rs
  tests/compat.rs  tests/compat/0.1.0/{settings.toml,queue.json,fight.toml}   (Task 15)
tools/ballfight_replay_hash.py                                           (Task 16)
.github/workflows/studio.yml  studio-release.yml                          (Task 14)
docs/studio/m6-measurements.md                                            (Tasks 4, 11, 12, 19)
docs/studio/HANDOFF.md  studio/README.md  studio/crates/bf-app/README.md  (Task 18)
```

The installed program (Task 11):

```
%LOCALAPPDATA%\Programs\BALLFIGHT Studio\
  ballfight-studio.exe   ballfight.ico   update.toml   MANIFEST.txt
  ffmpeg\ffmpeg.exe  ffmpeg\ffprobe.exe  ffmpeg\LICENSE.txt  ffmpeg\ABOUT.txt
  guide\index.html  guide\pictures\*.png
  .next\       (only while an update waits for a restart)
  .previous\   (the version before the last update, for "Go back")
%APPDATA%\Microsoft\Windows\Start Menu\Programs\BALLFIGHT Studio.lnk
<Desktop>\BALLFIGHT Studio.lnk
```

The app's data (unchanged from milestone 2, plus `data\`):

```
%APPDATA%\BALLFIGHT Studio\config\     settings.toml  queue.json
%LOCALAPPDATA%\BALLFIGHT Studio\cache\  shortlists\
%LOCALAPPDATA%\BALLFIGHT Studio\data\   logs\  source\  build\  downloads\
```

`data\` is `directories::ProjectDirs::data_local_dir()`. `logs\` holds `start.txt` (the last start) and
`crash-YYYYMMDD-HHMMSS.txt` (the newest 20). `source\` is the app's own copy of the repo for updates (route A),
`build\` its cargo target folder, `downloads\` the ffmpeg archive.

---

### Task 1: The ffmpeg pin and its fetcher (`bf-pack`)

**Files:**
- Create: `studio/packaging/ffmpeg.toml`; `studio/crates/bf-pack/{Cargo.toml, README.md, src/lib.rs, src/run.rs,
  src/sha.rs, src/ffmpeg.rs, src/fetch.rs, src/main.rs, tests/scan.rs}` and empty `src/{package,install,remove,
  config,version}.rs` (declared in `lib.rs` now, filled by Tasks 11 and 12, so no later task edits `lib.rs`)
- Modify: `studio/Cargo.toml` (member `crates/bf-pack`, `bf-pack = { path = "crates/bf-pack" }`), `Cargo.lock`

**Interfaces:**
- Produces:
  ```rust
  // run.rs: the one way to start a program
  pub struct Ran { pub status: std::process::ExitStatus, pub stdout: Vec<u8>, pub tail: Vec<String> }
  pub fn quiet(program: &Path, args: &[&OsStr]) -> std::process::Command;   // CREATE_NO_WINDOW, stdin null, pipes
  pub fn run(program: &Path, args: &[&OsStr], limit: Duration, cancel: &AtomicBool) -> Result<Ran, String>;
  // sha.rs
  pub fn sha256_file(path: &Path) -> std::io::Result<String>;               // lower-case hex
  // ffmpeg.rs
  pub struct FfmpegPin { pub version: String, pub build: String, pub url: String, pub archive_sha256: String,
      pub archive_bytes: u64, pub licence: String, pub source_note: String, pub files: Vec<PinnedFile> }
  pub struct PinnedFile { pub name: String, pub in_archive: String, pub sha256: String }   // name: "ffmpeg.exe"
  pub fn pin() -> FfmpegPin;      // parses include_str!("../../../packaging/ffmpeg.toml"); a bad file is a test failure
  pub fn ensure(pin: &FfmpegPin, cache: &Path, dl: &dyn Download) -> Result<PathBuf, PackError>;  // verified files
  pub fn place(pin: &FfmpegPin, verified: &Path, dest: &Path) -> Result<(), PackError>;  // ffmpeg\ + ABOUT.txt
  // fetch.rs
  pub trait Download { fn fetch(&self, url: &str, to: &Path) -> Result<(), String>; }
  pub struct Curl;          // %SystemRoot%\System32\curl.exe -fsSL --retry 3 --proto =https -o <to> <url>
  // lib.rs
  pub enum PackError { Download(String), Checksum { what: String, want: String, got: String }, Archive(String),
      Io(String), Refused(String) }       // Display: one plain sentence each
  ```
  `ballfight-pack fetch-ffmpeg --cache <dir> --out <dir>` (exit 0, or 1 with the sentence on stderr).
- **The pin's values.** Read them on this PC, never retype them from memory: `winget show Gyan.FFmpeg --version
  9.0.2` gives the installer URL and its SHA-256; the size comes from the downloaded file; each needed file's SHA-256
  from the archive. Then check that the archive's `bin\ffmpeg.exe` and `bin\ffprobe.exe` have the same SHA-256 as the
  files the winget links point to on this PC (`...\WinGet\Packages\Gyan.FFmpeg_*\...\bin\`). If they differ, stop and
  report: the reference bytes were made with the PC's files. `licence = "GPL-3.0-or-later"`; `source_note` says where
  the build's README says its sources are. If winget names no URL, use the www.gyan.dev release archive of 9.0.2 and
  say so in the report.
- **`ensure`:** if `cache` holds the archive with the right size and SHA-256, it is not downloaded again; otherwise it
  is downloaded to `<name>.part`, checked (size, then SHA-256), and renamed. A wrong checksum deletes the `.part` file
  and gives `Checksum` naming what, the wanted and the found hash. Then the needed files are unpacked (the `zip`
  crate) into `cache/<version>-<first 12 of sha>/` and each checked; any mismatch removes that folder and is an error.
  A folder already there is re-checked, not trusted.
- [ ] **Step 1: Write the failing tests** (a `Download` that copies a local file, archives built in the test with the
  `zip` crate): `the_pin_reads` (version "9.0.2", three files named, every hash 64 hex digits, a GPL licence);
  `a_cached_archive_is_not_fetched_again` (the fake counts calls: 1, then 0); `a_wrong_archive_is_refused_and_removed`
  (`Checksum`, no `.part` left, no folder); `a_short_download_is_refused`; `a_wrong_file_inside_is_refused`;
  `a_tampered_unpacked_file_is_unpacked_again`; `no_download_says_so_in_words` (`Download` error text contains "could
  not be downloaded"); `place_writes_about_and_licence` (ABOUT.txt holds the version, the URL, the hash and the
  sentence about a separate program); `scan.rs::programs_start_one_way` (no `Command::new` in `bf-pack/src` outside
  `run.rs`; the same scan reads `bf-app/src/{version,report,startup,icon,guide}.rs` and `bf-app/src/update/` when they
  exist). Marked `#[ignore]`, run by name: `the_real_archive_fetches` (real `Curl` into a temp cache; the files'
  hashes match the pin).
- [ ] **Step 2:** `cd studio && cargo test -p bf-pack`. Expected: FAIL.
- [ ] **Step 3:** Implement. `README.md`: what the crate is, the commands, where downloads go, that nothing it fetches
  is committed.
- [ ] **Step 4:** Run: PASS. Run the ignored test once on this PC and note the archive's size in the report.
- [ ] **Step 5:** Commit `bf-pack: the pinned ffmpeg and its checked download`.

### Task 2: `bf-export` prefers the app's own ffmpeg

**Files:**
- Modify: `studio/crates/bf-export/src/encode.rs`, `src/lib.rs` (re-export), `studio/crates/bf-app/src/engine/jobs.rs`
  (the `CheckTools` answer only)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum FfmpegSource { Setting, WithTheApp, Path }
  pub struct Ffmpeg { pub ffmpeg: PathBuf, pub ffprobe: PathBuf, pub version: String, pub source: FfmpegSource }
  pub fn find_ffmpeg() -> Result<Ffmpeg, ExportError>;   // BALLFIGHT_FFMPEG, then <exe dir>\ffmpeg\, then PATH
  pub(crate) fn find_with(var: Option<OsString>, beside: Option<PathBuf>, path: Option<OsString>)
      -> Result<Ffmpeg, ExportError>;
  ```
  `beside` is `std::env::current_exe()`'s folder joined with `ffmpeg`. The app's own ffmpeg takes only the ffprobe in
  the same folder: a missing one is an error naming the app's folder ("The app's own ffprobe is missing from
  {folder}, so a finished video cannot be checked; install the app again."), never a fall-back to another version on
  `PATH`. Every struct literal of `Ffmpeg` in the workspace gains `source`. `CheckTools` answers `Ok("ffmpeg 9.0.2,
  comes with the app")` for `WithTheApp`, `Ok("ffmpeg 9.0.2, found on this PC")` otherwise (the Settings screen stops
  adding the words itself in Task 13; until then the row shows both, which no test pins).
- [ ] **Step 1: Write the failing tests** (temp folders holding a copy of the PC's ffmpeg when there is one, else a
  notice and a pass, as the file's other tests do): `the_setting_comes_first`; `the_apps_own_comes_before_the_path`
  (`source == WithTheApp`); `the_path_is_last`; `the_apps_own_ffmpeg_needs_its_own_ffprobe` (an ffprobe on `PATH`
  is not used; the error names the folder); and in `jobs.rs` `tools_say_where_ffmpeg_came_from`.
  `the_arguments_are_the_kits` is not touched.
- [ ] **Step 2:** `cargo test -p bf-export --lib encode` and `cargo test -p bf-app --release --lib engine::jobs`.
  Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run both, then `cargo test --release -p bf-cli --test e2e -- --exact render_is_reproducible`: PASS.
- [ ] **Step 5:** Commit `bf-export: the app's own ffmpeg comes before the PATH`.

### Task 3: The encode is pinned

**Files:**
- Create: `studio/replay/encode_hashes.txt`
- Modify: `studio/crates/bf-cli/tests/e2e.rs`

**Interfaces:**
- Produces: `encode_hashes.txt` in the form of `render_hashes.txt` (comment header saying what made it, "Do not edit by
  hand"), with the line `ffmpeg <version line ffmpeg prints first>` and `clip0-90.mp4 <sha256>`: the mp4 of
  `ballfight render fights/flute_vs_trombone.toml --rec reference/fights/flute_vs_trombone/ep392.bfrec --frames 0:90`.
  Test `the_encode_is_pinned`: render the clip with the found ffmpeg; if its first version line differs from the
  file's, print a notice and pass, except with `BF_NEED_PINNED_FFMPEG=1`, where it fails naming both; otherwise the
  mp4's SHA-256 must equal the file's. `BF_BLESS=1` writes the file, as `render_is_reproducible` does. Ignored, run by
  name: `the_whole_video_is_pinned` (the whole 61 s video, line `video.mp4 <sha256>` in the same file).
- [ ] **Step 1: Write the failing test** `the_encode_is_pinned` (no file yet).
- [ ] **Step 2:** `cargo test --release -p bf-cli --test e2e -- --exact the_encode_is_pinned`. Expected: FAIL.
- [ ] **Step 3:** Implement. Bless twice: with the PC's winget ffmpeg (`BALLFIGHT_FFMPEG` naming it) and with the
  pinned files from Task 1 (`bf-pack fetch-ffmpeg`): both must write the same hash, or stop and report (D3's
  premise). Then bless `the_whole_video_is_pinned` the same way.
- [ ] **Step 4:** Run with `BF_NEED_PINNED_FFMPEG=1` and the pinned ffmpeg: PASS. Run once with a different ffmpeg (if
  none is on the PC, a copy of `ffmpeg.exe` renamed elsewhere does not count; skip and say so): the notice prints.
- [ ] **Step 5:** Commit `bf-cli: the encode is pinned to the bundled ffmpeg`.

### Task 4: The faster encoder setting, in the engine and the command line

**Files:**
- Modify: `studio/crates/bf-export/src/encode.rs`, `src/job.rs`, `src/lib.rs`, `tests/job.rs` (its `opts()`
  helper), `studio/crates/bf-cli/src/main.rs`, `src/cmd/render.rs`, `src/cmd/bench.rs`, `tests/e2e.rs`,
  `studio/crates/bf-app/src/engine/jobs.rs` (`speed: EncodeSpeed::Best` in its `JobOpts`, until Task 13),
  `studio/replay/encode_hashes.txt` (one more line)
- Create: `docs/studio/m6-measurements.md` (section 1)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
  pub enum EncodeSpeed { #[default] Best, Faster }
  impl EncodeSpeed { pub fn preset(self) -> &'static str; pub fn name(self) -> &'static str; }  // "best", "faster"
  pub fn encode_args(wav: &Path, out: &Path, nf: usize) -> Vec<OsString>;   // unchanged: = encode_args_at(.., Best)
  pub fn encode_args_at(wav: &Path, out: &Path, nf: usize, speed: EncodeSpeed) -> Vec<OsString>;
  pub struct JobOpts { pub out_root: PathBuf, pub workers: usize, pub frames: Option<Range<usize>>,
      pub speed: EncodeSpeed }
  ```
  `ballfight render ... --encode best|faster` (default `best`); `bench --out` reports the encode setting it used.
- **Choosing "Faster".** On this PC, render the example episode (`--rec ...ep392.bfrec`) with each of the presets
  `medium`, `fast`, `faster` and `veryfast` (a one-off measurement build with the preset as a test-only argument;
  never committed), three runs each, all checks on. "Faster" is **the slowest of these whose median whole render is
  60 s or less and which passes all 15 checks in every run**; if none reaches 60 s, the one with the shortest median
  that passes every check, and the report says the target was missed. Write a table in `m6-measurements.md`
  (preset, the three times, median, file size, checks) and the choice. The Settings line in Task 13 uses the measured
  saving, rounded to the nearest 5 s.
- [ ] **Step 1: Write the failing tests:** `the_arguments_are_the_kits` stays as it is; add
  `best_is_the_kits_arguments` (`encode_args_at(.., Best) == encode_args(..)`, token for token);
  `faster_changes_only_the_preset` (the two lists differ in exactly one token, the one after `-preset`);
  `the_command_line_takes_encode` (bf-cli argument parsing, `--encode fast` refused in words naming the two
  choices); in `e2e.rs` `the_faster_clip_is_pinned` (a line `clip0-90-faster.mp4` in `encode_hashes.txt`, same
  version rule as Task 3) and `a_faster_clip_passes_its_checks`.
- [ ] **Step 2:** `cargo test -p bf-export --lib encode`, `cargo test -p bf-cli`. Expected: FAIL.
- [ ] **Step 3:** Implement; measure; bless the faster clip's line with the pinned ffmpeg.
- [ ] **Step 4:** Run: PASS; `the_encode_is_pinned` and `render_is_reproducible` still pass unchanged.
- [ ] **Step 5:** Commit `bf-export: a faster encoder setting beside the kit's`.

### Task 5: The app's new modules and its version

**Files:**
- Create: `studio/crates/bf-app/build.rs`, `src/version.rs`, and empty `src/{report,startup,icon,guide}.rs`,
  `src/update/{mod,git,stage,swap,worker}.rs` (`update/mod.rs` holds only its `pub mod` lines)
- Modify: `studio/crates/bf-app/Cargo.toml` (`build = "build.rs"`, dependency `bf-pack.workspace = true`),
  `src/lib.rs` (the six `pub mod` lines; no later task edits `lib.rs`)

**Interfaces:**
- Produces:
  ```rust
  pub const VERSION: &str = env!("CARGO_PKG_VERSION");
  pub const COMMIT: &str = env!("BF_COMMIT");       // 7 hex digits, or "unknown" when git is not there
  pub const DATE: &str = env!("BF_COMMIT_DATE");    // the commit's date, "2026-11-12", or "unknown"
  pub fn line() -> String;                          // "0.6.0 (a842e11, 12 Nov 2026)"
  pub fn version_text() -> String;                  // "BALLFIGHT Studio 0.6.0 (a842e11, 12 Nov 2026)", for --version
  ```
  `build.rs` runs `git rev-parse --short=7 HEAD` and `git log -1 --format=%cs` (through `bf_pack::run`, so no
  window), and asks to re-run when the git folder's `HEAD` or the ref it names changes (`git rev-parse --git-path
  HEAD`, works in lane clones and worktrees). The date is the commit's, never the clock's, so the same commit builds
  the same exe text.
- [ ] **Step 1: Write the failing tests:** `the_line_reads_plainly` (`line()` matches `^\d+\.\d+\.\d+(-rc\.\d+)?
  \((unknown|[0-9a-f]{7}), (unknown|\d{1,2} [A-Z][a-z]{2} \d{4})\)$`); `the_commit_is_this_checkout`
  (when git is there, `COMMIT` equals `git rev-parse --short=7 HEAD` run by the test).
- [ ] **Step 2:** `cargo test -p bf-app --lib version`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS; `cargo test -p bf-app --lib` and `--test ui` still pass (nothing else changed).
- [ ] **Step 5:** Commit `bf-app: its version names its commit`.

### Task 6: Fault reports

**Files:**
- Modify: `studio/crates/bf-app/src/report.rs`, `src/store/mod.rs` (`Paths` gains `logs`)

**Interfaces:**
- Produces:
  ```rust
  // store::Paths
  pub struct Paths { pub config: PathBuf, pub cache: PathBuf, pub logs: PathBuf }
  // system(): logs = data_local_dir()\logs; under(root): root\logs. Every literal of Paths gains `logs`.
  // report
  pub fn install_hook(logs: Option<PathBuf>);       // keeps the default hook's stderr line, then writes the report
  pub fn crash_text(message: &str, location: &str, thread: &str, backtrace: &str, when: &str) -> String;
  pub fn write_crash(logs: &Path, text: &str, now: chrono::DateTime<chrono::Local>) -> std::io::Result<PathBuf>;
  pub fn write_start(logs: &Path, lines: &[String]) -> std::io::Result<()>;   // start.txt, replaced each start
  pub fn tell(title: &str, text: &str);             // Windows message box (MB_ICONERROR); elsewhere stderr
  pub const CRASHED: &str = "BALLFIGHT Studio stopped because of a fault in the app. A report was saved as {file}. \
      Send that file to whoever builds the app, then start BALLFIGHT Studio again: your queue and settings are kept.";
  ```
  A report holds: `version::version_text()`, Windows' version (`RtlGetVersion` numbers), the time, the thread's
  name, the message, the file and line, and `std::backtrace::Backtrace::force_capture()` (the release profile keeps
  line tables, `debug = 1`). It never holds settings values, paths under the owner's home other than the report's
  own, or anything typed in the caption box. Writing is best effort: a hook that cannot write still shows the
  message box (without "A report was saved as"), and never panics itself. Only the newest 20 reports are kept.
  The message box is shown only for a panic on the interface thread (the engine already turns a worker's panic into
  words through `on_panic`); a worker's panic still writes a report.
- [ ] **Step 1: Write the failing tests:** `a_report_holds_what_is_needed` (version, message, `file.rs:12`, thread,
  "stack"); `a_report_is_named_by_its_time` (`crash-20261112-093005.txt`); `the_newest_twenty_are_kept`;
  `a_folder_that_cannot_be_written_is_survived` (a path under a plain file: `Err`, no panic);
  `the_hook_writes_a_report` (run this test program again as a child with `BF_APP_REPORT_CHILD=<folder>`, which
  installs the hook and panics: one report file appears, the child's exit code is not 0); `paths_hold_logs`.
- [ ] **Step 2:** `cargo test -p bf-app --lib report store::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS; the whole crate's tests still pass.
- [ ] **Step 5:** Commit `bf-app: a fault leaves a report`.

### Task 7: Start-up guards

**Files:**
- Modify: `studio/crates/bf-app/src/startup.rs`; `studio/crates/bf-app/Cargo.toml` (`windows-sys` features:
  `Win32_Foundation`, `Win32_System_Threading`, `Win32_UI_WindowsAndMessaging`)

**Interfaces:**
- Produces:
  ```rust
  pub enum Instance { First(Claim), Second }        // Claim holds the mutex until dropped
  pub fn claim(config_dir: &Path) -> Instance;       // named mutex "Local\BALLFIGHT Studio <16 hex of sha256 of the
                                                     // lower-cased absolute config path>"
  pub fn bring_forward(title: &str) -> bool;         // FindWindowW, ShowWindow(SW_RESTORE), SetForegroundWindow
  pub fn no_window_text(report: Option<&Path>) -> String;
  pub const NO_GRAPHICS: &str = "BALLFIGHT Studio could not open its window, because Windows did not offer it a \
      graphics card to draw with (it needs DirectX 12). Update the graphics driver from the maker of your graphics \
      card, restart the PC, then start BALLFIGHT Studio again.";      // no_window_text adds the report sentence
  pub const SOFTWARE_DRAWING: &str = "This PC draws the window without its graphics card, so the preview may be \
      slow. Updating the graphics driver usually fixes this.";
  pub fn software_adapter(info: &wgpu::AdapterInfo) -> bool;          // device_type == Cpu
  ```
- [ ] **Step 1: Write the failing tests:** `one_claim_per_folder` (a second `claim` of the same folder in this process
  is `Second`; after the first is dropped, `First` again; another folder is `First`); `the_name_ignores_case`
  (`C:\A\config` and `c:\a\CONFIG` give one lock); `the_texts_read_plainly` (each passes the M2 sentence-case rule
  used by `tests/ui/wording.rs`, through a shared function the wording test already has, or a copy of its rule
  if it is private: the copy is named in the report); `a_software_adapter_is_noticed`.
- [ ] **Step 2:** `cargo test -p bf-app --lib startup`. Expected: FAIL.
- [ ] **Step 3:** Implement. `bring_forward` is not run by tests (it moves real windows): keep it to a few lines.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: one copy at a time, and words for a PC that cannot draw`.

### Task 8: The icon

**Files:**
- Modify: `studio/crates/bf-app/src/icon.rs`
- Create: `studio/crates/bf-app/assets/icon/ballfight.ico`

**Interfaces:**
- Produces:
  ```rust
  pub const SIZES: [u32; 5] = [16, 24, 32, 48, 256];
  pub fn rgba(size: u32) -> Vec<u8>;            // drawn with integer maths only: `ground` square, two touching discs
                                                // (`text` #e2e2e2 on the left, `muted` #a0a0a0 on the right)
  pub fn ico_bytes() -> Vec<u8>;                // an ICO holding one PNG per size (the `png` crate)
  pub fn window_icon() -> egui::IconData;       // the 256 picture
  ```
- [ ] **Step 1: Write the failing tests:** `the_ico_holds_every_size` (parse the header and directory: five entries,
  widths 16 to 256 (256 written as 0), each a PNG that decodes to its size); `the_committed_icon_is_current`
  (`assets/icon/ballfight.ico` equals `ico_bytes()`; `BF_BLESS=1` writes it); `the_icon_is_greys_only` (every
  opaque pixel has r == g == b).
- [ ] **Step 2:** `cargo test -p bf-app --lib icon`. Expected: FAIL.
- [ ] **Step 3:** Implement; bless; look at the 256 and 16 pictures and describe them in the report.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: the icon`.

### Task 9: Start-up wiring (waits for M2 Task 35)

**Files:**
- Modify: `studio/crates/bf-app/src/main.rs`, `src/app.rs` (`start` takes the notices and the adapter line)

**Interfaces:**
- Consumes: `version`, `report`, `startup`, `icon` (Tasks 5 to 8), `update::{installed, swap::finish}` (Task 12),
  `store::Paths` (with `logs`).
- Produces, in this order in `main`:
  1. `--version` prints `version::version_text()` on stdout and exits 0 (before anything else; no window).
  2. `--finish-update <pid>` runs `update::swap::finish` (Task 12) and exits with its code; no window.
  2a. `--test-fault-report` (not in any help text) makes the interface thread panic after the first painted frame, on
     purpose, so the release checklist can see a real report and message box from the shipped exe.
  3. `report::install_hook(paths.logs)` for every run except `--smoke` and `--fixtures` without `--data-dir`,
     whose logs go in their temp folder.
  4. `startup::claim(&paths.config)`: `Second` calls `bring_forward("BALLFIGHT Studio")` and exits 0.
  5. The window: `with_icon(icon::window_icon())`. When `eframe::run_native` returns an error, write it to
     `start.txt` and show `report::tell("BALLFIGHT Studio", &startup::no_window_text(..))`, exit 1.
  6. In the creation closure: the adapter line, the paths, ffmpeg's answer when it comes, and the version go to
     `start.txt` (the stderr line stays); a software adapter adds the notice `SOFTWARE_DRAWING`.
  7. The existing start-up errors (a bad `--data-dir`, an unknown option) also go to `report::tell`, so a person
     who double-clicks sees them.
  A first start with no settings: the defaults, as today; Render and the Library already say what to do when
  `Videos\BALLFIGHT` does not exist yet (M2 Review Focus 1). Confirm that a start sends `CheckFolder` for the output
  folder (M2 Task 35 makes the start-up effects); if it does not, add it here and say so.
- [ ] **Step 1: Write the failing tests** (in `main.rs`'s tests, and process tests that run the built release exe
  with `--data-dir` in a temp folder): `the_command_line` gains `--version`, `--finish-update 1234` and
  `--test-fault-report`; `a_fault_report_from_the_exe` (`#[ignore]`, run by name: the release exe with
  `--test-fault-report --data-dir X`: a `crash-*.txt` appears in `X\logs`; the test closes the message box);
  `version_prints_and_exits` (stdout equals `version_text()`, exit 0, within 2 s); `a_second_copy_steps_aside`
  (`#[ignore]`, run by name: opens a window: start `--data-dir X --smoke` twice at once; the second exits 0 within
  2 s, the first after its 30 frames); `no_window_is_told` (`#[ignore]`, run by name: `WGPU_BACKEND=metal` gives no
  adapter on Windows: exit 1, `start.txt` holds the error; the message box is closed by the test through
  `FindWindowW`/`WM_CLOSE`).
- [ ] **Step 2:** `cargo test -p bf-app --release --bin ballfight-studio`, and the ignored ones by name. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS. Start the release exe by double-click from Explorer (the controller does this on the PC):
  no console window, the icon in the taskbar, `start.txt` written.
- [ ] **Step 5:** Commit `bf-app: start-up in plain words, one copy, its icon`.

### Task 10: The folder dialog in front, and what a render leaves behind

**Files:**
- Modify: `studio/crates/bf-app/src/app.rs` (one line: remember the window handle), `src/engine/files.rs`,
  `studio/crates/bf-export/src/output.rs`, `studio/crates/bf-app/src/store/library.rs`

**Interfaces:**
- Produces:
  ```rust
  // engine::files
  pub fn set_parent_window(hwnd: isize);            // a OnceLock; app::start calls it from cc's window handle
  // PickFolder: rfd::FileDialog::set_parent(..) when a handle was set
  pub fn clear_abandoned(root: &Path, older_than: Duration, now: SystemTime) -> Result<usize, String>;
  // bf-export output.rs: when the final move finds the name taken, the finished folder is made visible and
  // renamed <name>_kept-<k> (k from 1) beside it, and the error names that folder
  ```
  `clear_abandoned` sends to the Recycle Bin (`SHFileOperationW`, `FO_DELETE | FOF_ALLOWUNDO | FOF_NOCONFIRMATION |
  FOF_SILENT | FOF_NOERRORUI`) every folder in `root` whose name starts with `.tmp-`, that is older than `older_than`,
  that is not a link, and that holds no file ending in `.mp4` (a `.mp4.part` does not count); it never looks below
  `root`'s own children and never follows a link. It runs before each Library scan with one day. A folder it cannot
  move is left and counted in the report line, never retried in a loop.
- [ ] **Step 1: Write the failing tests:** `a_kept_video_is_visible_and_named` (bf-export: the name taken
  meanwhile; the folder `<name>_kept-1` exists, is not hidden, holds the mp4; the error names it);
  `old_leftovers_go_to_the_bin` (a `.tmp-x-1` folder two days old holding a `.mp4.part` is gone from `root`;
  `flute_vs_trombone`, a `.keep` folder and a new `.tmp-` folder stay); `a_leftover_with_a_video_is_never_touched`;
  `links_are_never_followed` (a junction named `.tmp-y-1` to a folder outside: the outside folder and its files stay);
  `the_dialog_has_a_parent_once_set` (unit: the OnceLock value is passed to the builder; the dialog itself is not
  opened by tests).
- [ ] **Step 2:** `cargo test -p bf-export --lib output` and `cargo test -p bf-app --release --lib engine::files`.
  Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS. On the PC: "Change folder" opens the dialog in front of the app, ten times in a row.
- [ ] **Step 5:** Commit `bf-app: the folder dialog in front; old render leftovers go to the bin`.

### Task 11: Install, shortcuts and removal

**Files:**
- Modify: `studio/crates/bf-pack/src/{package,install,remove,config,version}.rs`, `src/main.rs`, `README.md`
- Modify: `docs/studio/m6-measurements.md` (section 2: the package's files and sizes)

**Interfaces:**
- Produces:
  ```rust
  // config.rs: update.toml, beside the installed exe
  pub struct UpdateConfig { pub source: PathBuf, pub remote: String, pub tag_prefix: String, pub build_dir: PathBuf,
      pub downloads: PathBuf }
  pub fn read_config(path: &Path) -> Result<UpdateConfig, PackError>;
  pub fn write_config(path: &Path, c: &UpdateConfig) -> Result<(), PackError>;   // refuses a remote with user info
  // version.rs
  #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
  pub struct Version { pub major: u64, pub minor: u64, pub patch: u64, pub rc: Option<u64> }  // rc sorts before none
  pub fn parse_version(s: &str) -> Option<Version>;      // "0.6.0", "0.6.0-rc.1"; nothing else
  // package.rs
  pub struct Manifest { pub version: String, pub files: Vec<(String, u64, String)> }   // path, bytes, sha256
  pub fn package(exe: &Path, tree: &Path, ffmpeg: &Path, out: &Path) -> Result<Manifest, PackError>;
      // out: ballfight-studio.exe, ballfight.ico (tree's bf-app/assets/icon), ffmpeg\ (place), guide\ (tree's
      // studio/packaging/guide, when present), MANIFEST.txt
  // install.rs
  pub struct Layout { pub programs: PathBuf, pub start_menu: PathBuf, pub desktop: PathBuf, pub data: PathBuf }
  impl Layout { pub fn system() -> Option<Layout>; pub fn under(root: &Path) -> Layout; }
  pub fn install(package: &Path, layout: &Layout, config: &UpdateConfig) -> Result<(), PackError>;
  pub fn shortcuts(layout: &Layout) -> Result<(), PackError>;
  // remove.rs
  pub fn removal_plan(layout: &Layout) -> Vec<PathBuf>;   // what goes (D7 f), in order
  pub fn schedule_removal(layout: &Layout, after_pid: u32) -> Result<(), PackError>;
  ```
  `ballfight-pack package --exe <exe> --out <folder>`; `ballfight-pack install --exe <exe> --from <owner's clone>`
  (packages, then installs into `Layout::system()`, makes the app's source copy for updates and writes
  `update.toml`; `--root <folder>` installs under a test folder instead, with no shortcuts made outside it).
- **The source copy (route A).** `install` reads the owner's clone's remote with `git -C <from> remote get-url
  origin`; a URL with user info is refused in words without printing it. Then `git clone --no-checkout <remote>
  <data>\source` (git signs in by itself; see the Global Rules on sign-in windows), and `update.toml` records
  `source`, `remote`, `tag_prefix = "studio-v"`, `build_dir = <data>\build`, `downloads = <data>\downloads`. A source
  folder that exists and is a clone of the same remote is kept.
- **Shortcuts.** One fixed Windows PowerShell script in `install.rs` (`-NoProfile -NonInteractive -Command`), which
  reads the link path, the target, the icon and the folder from environment variables set on that one child and
  makes the link with `WScript.Shell`; run through `bf_pack::run`. Names: `BALLFIGHT Studio.lnk` in the Start menu's
  `Programs` folder and on the desktop (`directories::UserDirs::desktop_dir()`).
- **Install is a swap too.** A new install into a folder that holds a running app is refused in words ("Close
  BALLFIGHT Studio first."). Files go to `<programs>\BALLFIGHT Studio.installing\`, then the old folder (if any)
  becomes `.previous` inside the new one, as an update does (Task 12).
- **Removal** (`schedule_removal`): writes nothing to the registry. It starts one hidden Windows PowerShell child
  with a second fixed script (values again from environment variables) that waits for `after_pid` to end
  (`Wait-Process -Id <pid> -Timeout 60`), then removes, in order, the two shortcuts, the program folder and the
  rebuildable data folders of D7 f, and exits; the app closes right after starting it. Settings, the queue, `logs`
  and every video stay. The paths it removes are checked before the
  command is built: each must be the exact path `Layout` gives, under `%LOCALAPPDATA%`, `%APPDATA%` or the desktop,
  and named as above; anything else is refused.
- [ ] **Step 1: Write the failing tests** (everything under a temp `Layout::under`; shortcuts made by the real
  PowerShell into the temp folders, since that writes nothing outside them): `versions_order`
  (`0.6.0-rc.1 < 0.6.0 < 0.6.1 < 0.10.0`; `v0.6`, `0.6.0-beta` are `None`); `a_token_in_the_remote_is_refused`
  (the error does not contain the token's text); `the_package_holds_every_file` (MANIFEST.txt lists each file with
  its size and hash, and the hashes are right); `install_into_a_fresh_folder`; `install_over_an_old_one_keeps_it_as_
  previous`; `install_refuses_while_the_app_runs` (a child process holding the exe open); `the_shortcuts_point_at_
  the_exe` (read the `.lnk` back through the same PowerShell object: target, icon, working folder);
  `removal_plans_only_the_apps_own_paths` (exact list; settings, logs and videos absent; a layout whose folder is
  not named "BALLFIGHT Studio" is refused); `removal_happens_after_the_app_ends` (`#[ignore]`, run by name: a child
  that sleeps 2 s; the temp program folder is gone within 10 s after it ends, the temp config folder stays).
- [ ] **Step 2:** `cargo test -p bf-pack`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS, and the ignored test by name. Package the current release exe once and write section 2
  of `m6-measurements.md` (each file's size, the total).
- [ ] **Step 5:** Commit `bf-pack: install, shortcuts and removal`.

### Task 12: The update engine

**Files:**
- Modify: `studio/crates/bf-app/src/update/{mod,git,stage,swap,worker}.rs`
- Modify: `docs/studio/m6-measurements.md` (section 3: how long an update takes)

**Interfaces:**
- Produces:
  ```rust
  // update/mod.rs
  pub struct Installed { pub dir: PathBuf, pub config: bf_pack::config::UpdateConfig }
  pub fn installed() -> Option<Installed>;          // update.toml beside current_exe(); None for a build in target\
  pub struct Release { pub tag: String, pub version: bf_pack::version::Version, pub commit: String }
  pub trait Source: Send {
      fn newest(&self) -> Result<Option<Release>, String>;
      // the built exe
      fn build(&self, r: &Release, progress: &dyn Fn(String), cancel: &AtomicBool) -> Result<PathBuf, String>;
  }
  #[derive(Clone, Debug, PartialEq)]
  pub enum UpdateState { NotInstalled, Checking, UpToDate, Available { version: String },
      Building { version: String, seconds: u64, doing: String }, Ready { version: String }, Failed { text: String } }
  #[derive(Clone, Debug, PartialEq)]
  pub struct UpdateView { pub state: UpdateState, pub previous: Option<String>, pub reports: Option<PathBuf>,
      pub guide: Option<PathBuf> }
  pub fn newer(current: &Version, tags: &[Release]) -> Option<Release>;   // the D1 tag rule
  // update/git.rs: route A
  pub struct GitSource { config: UpdateConfig, git: PathBuf, cargo: PathBuf }
  impl GitSource { pub fn new(c: UpdateConfig) -> Result<GitSource, String>; pub fn with_programs(..) -> GitSource; }
  pub fn parse_tags(for_each_ref: &str, prefix: &str) -> Vec<Release>;   // "<sha> <peeled sha> refs/tags/studio-v0.6.0"
  // update/stage.rs
  pub fn stage(inst: &Installed, exe: &Path, tree: &Path) -> Result<PathBuf, String>;   // <dir>\.next
  pub fn check_staged(next: &Path, want: &Release) -> Result<(), String>;  // runs the staged exe with --version
  // update/swap.rs
  pub fn launch_finish(inst: &Installed) -> Result<(), String>;   // starts .next's exe with --finish-update <our pid>
  pub fn finish(dir: &Path, old_pid: u32) -> Result<(), String>;  // the swap, then starts the new exe
  pub fn go_back(inst: &Installed) -> Result<(), String>;         // .previous becomes .next, then launch_finish
  // update/worker.rs: a thread of its own, like store::Writer, so the interface never waits
  pub enum Request { Check, Build, StopBuild, Restart, GoBack, OpenGuide, Remove }
  pub struct Worker; impl Worker { pub fn start(source: Option<Box<dyn Source>>, waker: Waker) -> Worker;
      pub fn send(&self, r: Request); pub fn view(&self) -> UpdateView; }
  ```
- **Route A, step by step.** Check: `git -C <source> fetch --prune --tags origin` (environment
  `GIT_TERMINAL_PROMPT=0`, so git never waits on a console that is not there; its own sign-in window may still open
  for the owner), then the tags are read with `git -C <source> for-each-ref --format="%(objectname) %(*objectname)
  %(refname)" refs/tags/<prefix>*` (`parse_tags` reads that output; the commit is the peeled one for an annotated
  tag). The check runs once at start and on "Look again". Build:
  `git -C <source> checkout --detach --force <tag>` (the copy is the app's own; `--force` only discards changes inside
  it, and a copy whose `origin` is not `config.remote` is refused), then `cargo build --release --locked -p bf-app`
  in `<source>\studio` with `CARGO_TARGET_DIR=<build_dir>` at below-normal priority (`BELOW_NORMAL_PRIORITY_CLASS`).
  `doing` is the last `Compiling <crate>` line. Cargo is `%USERPROFILE%\.cargo\bin\cargo.exe`, else the first on
  `PATH`; git is `PATH`'s, else `%ProgramFiles%\Git\cmd\git.exe`. Not found: `Failed` with "This PC can no longer
  build updates, because Git (or Rust) is not installed. Ask for it to be set up again."
- **Staging** copies the built exe and the tagged tree's icon and guide into `.next` with `bf_pack::package`, using
  the ffmpeg of the tag's own `ffmpeg.toml` (read from the tree; `bf_pack::ffmpeg::ensure` downloads only when the
  pin changed), then `check_staged`: the staged exe must print `BALLFIGHT Studio <tag's version> (<tag's commit
  first 7>, ...)` within 10 s, or the stage is removed and the update fails in words.
- **The swap** (`finish`, run by the new exe from `.next`): wait at most 30 s for `old_pid` to end; remove the old
  `.previous`; move every top-level item of `dir` except `.next`, `.previous` and `update.toml` into a new `.previous`;
  move every item of `.next` into `dir`; start `dir\ballfight-studio.exe`; exit 0. If any move fails: move back what
  was moved, start the old exe, and leave a line in `logs\start.txt` that the old app shows as a notice ("The update
  to version X could not be put in place, so this is still version W. Nothing was lost."). The new app removes an
  empty `.next` at start.
- **When a render is running**, `Restart` and `GoBack` are refused by the interface (Task 13) with "A video is
  rendering. Restart when it is done, or cancel it first."; the build itself may run during a render.
- **Route B** plugs in as another `Source` (Task B1); nothing else here changes.
- [ ] **Step 1: Write the failing tests** (a real local git repository made in a temp folder by the test, with tags;
  no network, no credentials; a fake cargo, which is this test program run again with `BF_APP_FAKE_CARGO=<exe to
  copy>`, as M2 Task 33 did for ffmpeg): `tags_parse` (annotated and plain tags, other prefixes ignored, a bad version
  ignored); `the_tag_rule` (from 0.6.0: 0.6.1 offered, 0.6.1-rc.1 not; from 0.6.1-rc.1: 0.6.1-rc.2 and 0.6.1
  offered); `no_newer_tag_is_up_to_date`; `a_newer_tag_is_available`; `a_build_that_fails_leaves_everything`
  (`Failed` with cargo's last line; no `.next`; the install folder unchanged); `a_staged_exe_must_name_the_tag`
  (the fake exe prints another version: refused, `.next` removed); `the_swap_moves_and_keeps_previous`;
  `a_failed_swap_puts_everything_back` (a file in the install folder held open by the test so its move fails);
  `go_back_swaps_previous_in`; `stop_build_stops_cargo` (the fake cargo sleeps; `StopBuild`; it ends within 2 s,
  `UpToDate` or `Available` again); `not_installed_says_so` (`installed()` is `None` in `target\`);
  `a_copy_of_another_remote_is_refused`; `the_worker_never_blocks_its_caller` (`send` returns within 1 ms while a
  build runs). Marked `#[ignore]`, run by name: `a_real_update_on_this_pc` (route A against a temp clone of this
  repo's own lane clone, two local tags, real cargo: measures the cold and the warm build).
- [ ] **Step 2:** `cargo test -p bf-app --release --lib update`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS, and the ignored test by name; write section 3 of `m6-measurements.md` (fetch, cold
  build, warm build after a one-line change in `bf-app`, staging, swap; the PC's busy share before each).
- [ ] **Step 5:** Commit `bf-app: the update engine (build on this PC)`.

### Task 13: Settings and the toolbar

Waits for Tasks 4, 6, 9 and 12, and for a moment when no milestone 5 task has `state/` or `seam/` open.

**Files:**
- Modify: `studio/crates/bf-app/src/seam/mod.rs` (`RunJob` gains `speed`), `src/model/mod.rs` (`Settings` gains
  `encode`), `src/state/mod.rs` (view-model fields, actions, effects), `src/state/queue_run.rs` (the speed on
  `RunJob`; refusing a restart while rendering), `src/store/settings.rs` (the `encode` key), `src/engine/jobs.rs`
  (the speed into `JobOpts`), `src/app.rs` (carries out `Effect::Upkeep`), `src/screens/settings.rs`,
  `src/screens/shell.rs`, `src/fixtures/settings.rs`, `src/seam/fake.rs` (accepts the new field),
  `tests/ui/settings.rs`, `tests/ui/shell.rs`, `tests/ui/wording.rs` (only if a word needs the list),
  `tests/snapshots/settings_1440x900.png`, `tests/snapshots/settings_1280x800.png`

**Interfaces:**
- Produces:
  ```rust
  pub enum EncodeChoice { Best, Faster }              // model; jobs.rs maps it to bf_export::EncodeSpeed
  pub struct Settings { pub output_folder: PathBuf, pub workers: usize, pub encode: EncodeChoice }
  // seam: RunJob { id, fight_toml, out_root, speed: EncodeChoice }
  // state: ViewModel { .., pub update: UpdateView, pub app_line: String }
  pub enum Action { .., SetEncode(EncodeChoice), Upkeep(update::Request), AskRemove, KeepApp }
  pub enum Effect { .., Upkeep(update::Request) }
  ```
  `settings.toml` stays `format = 1`: `encode = "best" | "faster"` is written; a file without it, or with any other
  value, reads as Best with no notice (an older app reading a new file ignores the key: Step 1's first test pins that
  before the key is added).
- **Settings rows** (one column, as today; it scrolls when the window is short), labels 150 wide:
  - "Output folder", "Worker threads": as today.
  - "Video encoding": segments "Best quality" and "Faster"; under them `small` `muted`: "Best quality is the
    series' own setting. Faster saves about {s} seconds a video and makes a slightly larger file." ({s} from Task 4's
    measurement, rounded to 5 s.) A change applies to the next video that starts.
  - "Video encoder": the `Tools` text as it comes ("ffmpeg 9.0.2, comes with the app"); missing: "ffmpeg was not
    found in the app's folder, so videos cannot be saved. Ask for the app to be installed again."
  - "Version": `version::line()`.
  - "Updates", by state: `NotInstalled`: "This copy was not installed, so it does not update itself."; `Checking`:
    "Looking for a newer version."; `UpToDate`: "This is the newest version." and box button "Look again";
    `Available`: "Version {v} is ready to be built on this PC. It takes about {m} minutes, and you can keep working."
    and primary button "Build version {v}"; `Building`: "Building version {v}: {mm:ss} so far." and box button "Stop
    building"; `Ready`: "Version {v} is ready." and primary button "Restart now" (disabled while a video renders,
    with the reason); `Failed`: the text and box button "Try again". With `previous`: box button "Go back to version
    {w}". ({m} is section 3's warm build rounded up to whole minutes.)
  - "Problems": "If something goes wrong, a report is saved in {logs}." and box button "Open the reports folder"
    (sends the existing `RevealFolder`).
  - "Guide": box button "Open the guide" (`Upkeep(OpenGuide)`: opens `guide\index.html` with Windows' default
    browser through `explorer.exe <that file>`; only that one file, checked to exist).
  - "Remove": box button "Remove BALLFIGHT Studio from this PC" (installed copies only); after it, the row reads
    "The app and its shortcuts are removed. Your videos, settings and fault reports stay." with primary button
    "Remove the app" and box button "Keep it". Confirming sends `Upkeep(Remove)`: the worker schedules the removal
    (Task 11), the app saves and closes.
- **The toolbar's "Update"**: `Checking` and `NotInstalled` draw it exactly as today (disabled), with the hover text
  of the state, so **no existing picture changes**: the shell picture and every screen's picture are left as they
  are and must pass unchanged. `UpToDate` and `Failed`: a box button; `Available` and `Ready`: the primary look. A
  click goes to Settings (the "Updates" row). The fixtures keep `Checking`.
- [ ] **Step 1: Write the failing tests:** first, on the code as it is, `an_unknown_settings_key_is_ignored` (a format
  1 file with `encode = "faster"` loads with no notice: this proves an older app reads a newer file; it passes
  before the change and must keep passing); then `the_encode_key_round_trips`; `no_encode_key_is_best`;
  `a_job_starts_with_the_setting` (state: `SetEncode(Faster)`, then the next `RunJob` carries `Faster`);
  `jobs_pass_the_speed_on` (engine unit); Settings: `the_rows` (every label above); `the_encode_segments_push`;
  `each_update_state_reads` (seven fixture states, each text and button); `restart_waits_for_a_render`;
  `remove_asks_first` (one click shows the confirming row; "Keep it" goes back; nothing is sent until "Remove the
  app"); `nothing_is_unnamed`; `both_sizes_match` (re-bless the two Settings pictures after looking at them);
  shell: `update_follows_the_state` (labels and look per state); and the existing shell, screen and
  `small_window` pictures pass **without** re-blessing.
- [ ] **Step 2:** `cargo test -p bf-app --test ui settings:: shell:: wording:: small_window::` and `cargo test -p
  bf-app --lib state:: store::settings engine::jobs`. Expected: FAIL.
- [ ] **Step 3:** Implement. Remove `UPDATE_LATER` and the old "starts working in a later version" text.
- [ ] **Step 4:** Run the whole crate (`cargo test -p bf-app`, and `--release --lib engine::`): PASS.
- [ ] **Step 5:** Commit `bf-app: Settings and the toolbar for updates, encoding, reports and removal`.

### Task 14: CI: the pinned ffmpeg, the pinned encode, the package job (waits for M2 Task 35)

**Files:**
- Modify: `.github/workflows/studio.yml`
- Create: `.github/workflows/studio-release.yml`

**Interfaces:**
- Produces, in `studio.yml`'s one Windows job (every existing step kept, in its order):
  1. The step "Install ffmpeg" is replaced by "Fetch the app's own ffmpeg": `actions/cache@v4` on
     `${{ runner.temp }}/ffmpeg-cache` keyed by `hashFiles('studio/packaging/ffmpeg.toml')`, then `cargo run --release
     --locked -p bf-pack -- fetch-ffmpeg --cache "$RUNNER_TEMP/ffmpeg-cache" --out "$RUNNER_TEMP/ffmpeg"`, then **the
     same encoder check as today** (the `libx264` and `aac` loop, unchanged) on that ffmpeg, then `BALLFIGHT_FFMPEG`
     set to it. A checksum failure fails the job.
  2. A new step after "Test that a render is reproducible": `BF_NEED_PINNED_FFMPEG=1 cargo test --release --locked -p
     bf-cli --test e2e -- --exact the_encode_is_pinned` (a 3 s clip).
  3. `cargo test --release --locked -p bf-pack` added to an existing test step's package list (no network: its one
     network test is ignored).
  4. If M2 Task 35's `bf-app` build and test steps are not in the job when this task starts, stop and report (they
     are M2's to add).
- `studio-release.yml`: runs on `push` of tags `studio-v*` and on `workflow_dispatch` only, no `paths` filter;
  `permissions: contents: read`. One Windows job: the same toolchain and cache steps as `studio.yml`; check that the
  tag's version equals `[workspace.package] version` (fails otherwise); `cargo build --release --locked -p bf-app`;
  fetch ffmpeg as above; `ballfight-pack package`; zip it as `ballfight-studio-<version>.zip`; write
  `SHA256SUMS.txt`; `actions/upload-artifact@v4` with both, `retention-days: 30`. For route B only (Task B1), a last
  step attaches both to a GitHub Release of the tag, with `contents: write` given to that job alone.
- **Minutes:** the main job grows by the fetch (cached after the first run) and one 3 s clip; the release job runs
  only when a tag is pushed.
- [ ] **Step 1:** No Rust tests; the check is CI itself. Write the two files.
- [ ] **Step 2:** The controller pushes the branch: the main job must pass; then pushes a throwaway tag
  `studio-v0.0.0-rc.1` on a commit whose version is set to it on a throwaway branch, which the release job must
  package; both tag and branch are deleted after, by the controller.
- [ ] **Step 3:** Fix anything red without removing or weakening a check.
- [ ] **Step 4:** Read the release artifact: the zip holds the package's files and its hash matches `SHA256SUMS.txt`.
- [ ] **Step 5:** Commit `ci: the pinned ffmpeg, the pinned encode and the package job`.

### Task 15: Saved files across versions

**Files:**
- Create: `studio/crates/bf-app/tests/compat.rs`, `tests/compat/0.1.0/{settings.toml, queue.json, fight.toml,
  README.md}`

**Interfaces:**
- Produces: the three files exactly as version 0.1.0 writes them (`save_settings`, `save_queue` with one waiting job
  and one done job, a fight file from `starter` with a seed), made once by a helper in the test file run with
  `BF_BLESS=1` and then frozen; `README.md` says what they are and the rule: **every release adds a folder for its own
  version, and no folder is ever rewritten.** Tests: `settings_from_0_1_0_read` (the values, no notice);
  `a_queue_from_0_1_0_reads_with_its_warning` (both jobs, the "saved by another version" notice exactly, only when
  the running version is not 0.1.0); `a_fight_file_from_0_1_0_reopens`; `the_frozen_files_are_unchanged` (their
  SHA-256s are written in the test).
- [ ] **Step 1:** Write the tests; **Step 2:** run them before blessing: FAIL (no files). **Step 3:** bless once.
  **Step 4:** `cargo test -p bf-app --test compat`: PASS. **Step 5:** Commit `bf-app: files saved by 0.1.0 still read`.

### Task 16: The kit's replay guard on Windows (D5)

**Files:**
- Modify: `tools/ballfight_replay_hash.py`

**Interfaces:**
- Produces: the tool compares paths in one form (`Path(...).as_posix()`) on every system, so on Windows it prints a
  line for every entry of its list. Nothing under `projects/` changes.
- [ ] **Step 1:** On this PC, run the tool as it is and save its output (`before.txt`, scratch folder); count its
  lines against its list's entries times their seeds.
- [ ] **Step 2:** Confirm the kit-episode lines are the missing ones.
- [ ] **Step 3:** Implement the smallest change.
- [ ] **Step 4:** Run it again: every entry has its lines; every line that `before.txt` holds is byte for byte the
  same; the new kit-episode lines equal the ones milestone 1 got by running the child commands one by one (the
  procedure in `m1-measurements.md` section 12). `git status --short projects reelkit series` is empty.
- [ ] **Step 5:** Commit `tools: the replay guard lists every episode on Windows`.

### Task 17: Guide pictures and the guide (LAST: needs the final content of milestones 3 to 5)

**Files:**
- Modify: `studio/crates/bf-app/src/guide.rs`, `src/main.rs` (one flag)
- Create: `studio/packaging/guide/index.html`, `studio/packaging/guide/pictures/*.png`,
  `studio/crates/bf-app/tests/guide.rs`

**Interfaces:**
- Produces: `ballfight-studio.exe --guide-pictures <folder> --data-dir <temp>`: runs the real app in a real window on
  the real engine, drives it through `Action`s (never through the mouse), waits for each state on the view model with
  a time limit, and saves each picture with egui's `ViewportCommand::Screenshot` (the real window's pixels, with
  Windows' own emoji font), then closes. Pictures, at 1440 x 900: `1-matchup.png` (two fighters picked),
  `2-balance.png` (balanced), `3-seeds.png` (a shortlist, a seed playing), `4-music.png`, `5-render.png` (a caption,
  the queue with one video rendering), `library.png`, `settings.png`, `update-ready.png` (the Settings "Updates" row
  in `Available`, from the fixtures, cropped to the row), `failed-job.png` (the fixtures' failed job, cropped to its
  row). It uses the guide's own match-up (named in the guide) and a seed found by the search, never a fixed seed.
- **The guide** (`index.html`, one page, plain HTML and CSS in the app's greys, no scripts, readable at 900 px wide):
  short sections with one picture each: Starting the app; 1 Match-up; 2 Balance; 3 Seeds; 4 Music; 5 Render; The
  queue (pause, cancel, reorder; it survives closing the app); The Library (reopen a video); Settings (output folder,
  worker threads, video encoding, where reports are); Updating (the Update button, "Build version X", "Restart now",
  "Go back"); When something goes wrong (a stopped job names the check and its numbers and what to try; ffmpeg
  missing; the output folder cannot be used; a fault report and where it is; who to ask: a new build session for
  new fighters, themes and features); Removing the app (what goes, what stays). Every on-screen word in the guide is
  quoted exactly as the app shows it, in `<b class="ui">`. No command lines.
- [ ] **Step 1: Write the failing tests** (`tests/guide.rs`): `every_picture_exists` (each `<img src>` is a file in
  `pictures/` and each file there is used); `every_quoted_label_is_in_the_app` (every `<b class="ui">` text is an
  accessible label or a text on screen in at least one of the M2 fixture view models or the real-engine screen of
  M2 Task 35, collected through the harness); `no_command_lines` (no `<code>`, no `cargo`, `git`, `powershell`, `cmd`
  or `.exe ` followed by an option); `the_guide_is_short` (at most 1,500 words of text); `the_wording_rule`
  (sentences in the guide's headings pass the M2 sentence-case rule).
- [ ] **Step 2:** `cargo test -p bf-app --test guide`. Expected: FAIL.
- [ ] **Step 3:** Implement; take the pictures on this PC; write the guide.
- [ ] **Step 4:** Run: PASS. Read the whole guide next to the app and fix any step that does not match what happens.
- [ ] **Step 5:** Commit `docs: the user guide, with pictures from the app`.

### Task 18: Handoff docs (LAST)

**Files:**
- Modify: `docs/studio/HANDOFF.md`, `studio/README.md`, `studio/crates/bf-app/README.md`, `studio/crates/bf-pack/
  README.md`

**Interfaces:**
- Produces:
  - `HANDOFF.md`: a new status section at the top, in plain words: what the owner has (the installed app, the guide,
    updates); where the program and its data live (the two trees above); how a version reaches the owner (bump the
    version, merge, CI green, push the tag `studio-v<version>`, then the owner clicks Update); what a build session
    needs on this PC; **"How to add a fighter"**: spec section 10's steps, made concrete with the commands and tests
    milestones 3 and 4 used (copied from their plans as merged, named, not retyped from memory), ending with the
    release steps; the rule for `tests/compat/<version>/`; that nothing under `projects/`, `reelkit/` or `series/` is
    edited. The old "How to run it" command section is replaced by "the owner double-clicks the app; build sessions
    use `ballfight` as `studio/README.md` describes".
  - **If milestones 3 to 5 are complete** (all 30 fighters and 26 themes in the registry, the music system merged):
    `HANDOFF.md` and `studio/README.md` say "Rust is now the source of truth for BALLFIGHT. The Python kit in
    `projects/ballfight/` is a frozen archive: it is kept so old episodes replay, and no new work goes there." **If
    they are not**, they say instead which fighters and themes are in the Studio (counted from the registry), that the
    Python kit stays the reference until the port is complete, and that this sentence changes when it is. In both
    cases (D6) the kit README itself is not edited; `HANDOFF.md` lists the one line the owner may ask a session to add
    to `projects/ballfight/README.md`.
  - `studio/README.md`: the `bf-pack` row in the crate table; "Packaging" (the package, the pin, the release job, the
    tags); the update route in use (D1) and how to switch to the other.
  - `bf-app/README.md`: the new flags (`--version`, `--finish-update`, `--guide-pictures`), the start log and reports.
- [ ] **Step 1:** Count the registry's fighters and themes on the merged code and decide which text applies.
- [ ] **Step 2:** Write. **Step 3:** Every command named in the docs runs on this PC as written (the controller runs
  each once). **Step 4:** `git status --short projects reelkit series` is empty.
- [ ] **Step 5:** Commit `docs: handoff notes for the packaged Studio`.

### Task 19: The release and the release checklist (LAST)

**Files:**
- Modify: `studio/Cargo.toml` (`[workspace.package] version`, D7 c), `Cargo.lock`, `docs/studio/m6-measurements.md`
  (section 4: the checklist's results)

**Interfaces:**
- Produces: the tags `studio-v<version>-rc.1` and `studio-v<version>` (pushed by the controller), the installed app
  on the owner's PC, and the filled-in checklist below.
- [ ] **Step 1:** Bump the version to `<version>-rc.1`; full gates: `cargo test --workspace --release`, the ignored
  tests by name (`the_whole_video_is_pinned`, `a_whole_video_with_the_real_engine`, `a_real_update_on_this_pc`, the
  bf-pack and start-up ignored tests), fmt and clippy, `git status --short projects reelkit series` empty. The
  controller pushes; CI green; the controller pushes the tag `studio-v<version>-rc.1`; the release job packages it.
- [ ] **Step 2:** Run the release checklist below, part A to part C, on the owner's PC.
- [ ] **Step 3:** Bump to `<version>`, commit, CI green, tag `studio-v<version>`; run part D (the update) and part E.
- [ ] **Step 4:** Write every result into section 4 of `m6-measurements.md`.
- [ ] **Step 5:** Commit `milestone 6: release <version>; the checklist's results`.

### Task B1 (only if the owner picks D1 route B): The download route

**Files:**
- Create: `studio/crates/bf-app/src/update/download.rs` (declared by adding one line to `update/mod.rs`)
- Modify: `.github/workflows/studio-release.yml` (the Release step), `src/update/worker.rs` (the source chosen by
  `update.toml`'s `route = "download"`)

**Interfaces:**
- Produces: `pub struct DownloadSource` implementing `Source`: `newest` lists the repo's releases through the GitHub
  API; `build` downloads the zip and `SHA256SUMS.txt` with `curl.exe` (the token passed on curl's standard input
  as a header file, `-H @-`, never on its command line), checks the hash, unpacks into a temp folder and returns its
  exe. The token comes from `git credential fill` (`protocol=https`, `host=github.com`) at the moment it is needed,
  lives in one `Zeroizing`-style buffer that is overwritten after use, and is never written, logged, shown or put in
  an error. **Tests use a local fake server process and a fake credential helper set through `GIT_CONFIG_*`
  environment variables on the child only; no test ever asks the real credential manager.** Tests: a wrong hash is
  refused; a missing release is "up to date"; an error text never contains the fake token; a download is cancelled.
- Steps as the others; commit `bf-app: updates downloaded from the release (route B)`.

---

## The release checklist

Run by the controller on the owner's PC in Task 19. Each step has its expected result; a step that does not give it
stops the release. `<version>` is the release's version, `<rc>` the candidate `<version>-rc.1`. The owner does only
the steps marked **(owner)**, each a click.

**Part A: a clean-data start**

1. Close the app if it runs. Rename, never delete, `%APPDATA%\BALLFIGHT Studio` to `BALLFIGHT Studio before
   <version>` and the same for `%LOCALAPPDATA%\BALLFIGHT Studio`. Expected: neither folder exists under its old name.
2. Download the `<rc>` package artifact from the release job; check its zip against `SHA256SUMS.txt`. Then
   `ballfight-pack install --exe <the zip's exe> --from "F:\!Ai Stuff\InstrumentBattleMachine"`. If a sign-in window
   opens, stop: the owner completes it. Expected: the program folder holds exactly the files of `MANIFEST.txt` with
   matching hashes; `ffmpeg\ffmpeg.exe`'s hash is the pin's; both shortcuts exist and point at the exe;
   `data\source` is a clone whose `origin` is the GitHub URL; `update.toml` holds no user name or token.
3. **(owner)** Double-click "BALLFIGHT Studio" on the desktop. Expected: within 5 s a window titled "BALLFIGHT
   Studio" with the app's icon in the taskbar; no console window at any moment; the Match-up screen on the real
   roster; `data\logs\start.txt` names the RX 9070 XT on Dx12 and the version.
4. Settings. Expected: output folder `C:\Users\<owner>\Videos\BALLFIGHT`; worker threads = the PC's logical cores
   minus one; "Best quality" chosen; "ffmpeg 9.0.2, comes with the app"; "Version" reads `<rc> (<commit>, <date>)`;
   "Updates" reads "This is the newest version."
5. **(owner)** Double-click the desktop shortcut again. Expected: the open window comes to the front; Task Manager
   shows one `ballfight-studio.exe`.

**Part B: a full render**

6. Make a video of the guide's match-up through all five steps, by clicks, with Best quality. Expected: the queue
   row reads "Done in" after about the time of `m6-measurements.md` section 1's Best median; the folder
   `Videos\BALLFIGHT\<a>_vs_<b>\` holds the mp4, `cover.png`, `caption.txt` and the fight file; ffprobe (the app's
   own, run by the controller) reports 61.01 s, h264 1080 x 1920 at 30 fps and aac 48 kHz stereo; the video plays
   with sound in Windows' own player and loops without a jump.
7. Set "Faster", render the next seed. Expected: about the Faster median; every check passed.
8. Library. Expected: both videos, newest first, with covers; "Reopen" on the first lands on Match-up with its seed.
9. Queue two videos, close the window while the first renders, start the app again. Expected: the notice that 1
   video was interrupted; "Resume queue" renders both.

**Part C: the failure screens**

10. Start with `WGPU_BACKEND=metal` set for that one start (the controller starts the exe). Expected: a message box
    with `NO_GRAPHICS`'s text and the report sentence; exit; `start.txt` holds the error.
11. Rename the program folder's `ffmpeg` to `ffmpeg.off`, start the app. Expected: Settings shows the missing-ffmpeg
    text; Render's "Add to queue" is disabled with the reason. Rename it back.
12. Choose an output folder the controller has made read-only (`icacls <folder> /deny <owner>:(W)`). Expected: Render
    and the Library name the folder and say what to do; adding to the queue says so instead of failing later.
    Remove the deny entry; delete the folder.
13. The controller starts the installed exe once with `--test-fault-report` (Task 9). Expected: the `CRASHED`
    message box naming the report; a `crash-*.txt` in `data\logs` holding the version, the commit and the panic's
    file and line; the next normal start opens as usual with the queue and settings intact.

**Part D: an update**

14. With `<rc>` installed and running, the controller pushes `studio-v<version>` (Task 19 step 3). **(owner)** Click
    "Update" in the toolbar when it shows the primary look (within a minute of starting the app, or after "Look
    again"). Expected: Settings' "Updates" row reads "Version <version> is ready to be built on this PC...".
15. **(owner)** Click "Build version <version>". Expected: "Building version <version>: mm:ss so far." counting;
    the app stays usable; it ends within section 3's cold-build time plus 50 %; the row reads "Version <version> is
    ready." with "Restart now".
16. **(owner)** Click "Restart now". Expected: the window closes and reopens within 10 s at `<version>`; settings,
    the queue and the Library are as before; `.previous` holds `<rc>`; Settings offers "Go back to version <rc>".
17. **(owner)** Click "Go back to version <rc>", then update again as in 14 to 16. Expected: `<rc>`, then
    `<version>` again, with nothing lost; the second build finishes within a minute, since `data\build` already holds
    it and nothing changed.

**Part E: removal, and handing back**

18. **(owner)** Settings, "Remove BALLFIGHT Studio from this PC", then "Remove the app". Expected: the window closes;
    within 10 s the program folder and both shortcuts are gone; `data\source`, `data\build`, `data\downloads` and
    `cache` are gone; `config` (settings, queue), `data\logs` and every video remain, as the guide says.
19. Install `<version>` again (step 2, with the `<version>` artifact). Move the folders renamed in step 1 back into
    place (the new ones from this checklist renamed to `... checklist <date>` first). **(owner)** Double-click the
    desktop shortcut. Expected: the owner's own settings, queue and Library from before the checklist; version
    `<version>`; "This is the newest version."

## Risks and open questions

1. **M2 Task 35 is not merged** (the app still says the engine is not connected), and the build's ledger records no
   result for M2 Tasks 31 and 32. Tasks 9, 10, 13, 14 and 17 wait for it; the rest do not.
2. **The ffmpeg archive's exact URL and checksum were not read while planning** (no network use here). Task 1 reads
   them from winget's record of the PC's own install and proves the files are the ones that made today's videos.
   If the archive is no longer published, D3 falls back to the essentials build or another GPL build, which needs
   Task 3's hashes blessed anew and the owner told that new videos' bytes change (not their pictures or sound).
3. **Update build times are unmeasured.** The expectation (minutes warm, 10 to 20 minutes cold) is an estimate until
   Task 12 measures it. A rust-toolchain pin change makes the next update download a toolchain through rustup.
4. **The build folder is large** (several GB under `data\build`); the PC has 670 GB free. Removal deletes it.
5. **A native crash is not a panic.** An access violation in a driver or a library ends the process without the
   hook; Windows logs it (Application log, event 1000). The guide says to send `start.txt` then.
6. **`SetForegroundWindow` can be refused by Windows** when the second start is not the foreground process; then the
   open window flashes in the taskbar instead. Acceptable; noted in the guide only if seen in the checklist.
7. **The exe's own icon in Explorer** stays Windows' generic one (D7 e).
8. **Antivirus** may scan or briefly hold a freshly built exe during staging; `check_staged` retries the start once
   after 5 s before it fails, and the failure text says an antivirus may be the cause.
9. **Version bumps are a human step.** If a tag's version and the workspace version differ, the release job fails;
   if a version is tagged twice, the update is not offered (it is not newer). HANDOFF.md states the release steps.
10. **Route B's verification** rests on HTTPS and a checksum file from the same release; build attestations for a
    private repo may not be available on the owner's GitHub plan. Not needed for route A.
11. **The GPL's source duty** applies if the program is ever given to anyone else; ABOUT.txt says where ffmpeg's
    source is.
12. **The kit README** cannot carry the "source of truth" note under this build's rules (D6).
13. **Faster encoding might not reach 60 s.** Task 4's rule then picks the quickest preset that passes every check
    and says the target was missed; the frames stage is shared between drawing and encoding on 16 threads.
14. **Open for the owner:** D1 (update route, recommended A), D4 (whether to use "Faster", their choice in Settings at
    any time), and the icon's look (D7 d). Everything else is decided above.
