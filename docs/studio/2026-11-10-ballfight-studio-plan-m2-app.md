# BALLFIGHT Studio, milestone 2 (the app): Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the desktop app `bf-app`: the five steps (Match-up, Balance, Seeds, Music, Render), the live preview
with sound, the seed shortlist, the render queue, the Library and Settings, on the one match-up milestone 1 delivers
(FLUTE vs TROMBONE, theme `big_top`, classic arrangement), so a complete video is made without leaving the app.

**Architecture:** One new crate, `studio/crates/bf-app`, an eframe + egui app drawn through wgpu. The interface is a
pure state machine (`AppState`) that turns user actions and engine events into a `ViewModel`, which the screens draw;
it talks to the engine only through a narrow seam of plain-data commands and events over channels, so the interface
never blocks and can be built and tested against a sample engine. The plan has two blocks: **Block A** builds the
whole interface on sample data and needs nothing from milestone 1; **Block B** puts the real engine behind the seam.

**Tech Stack:** Rust 1.99.1 (`x86_64-pc-windows-gnu` here, Linux in CI). Pinned exactly: `eframe`, `egui` and
`egui_kittest` 0.37.0 (which bring `egui-wgpu` 0.37.0, `wgpu` 30.0.1, `winit` 0.30.13), `cpal` 0.18.2 (sound out),
`rfd` 0.18.0 (folder dialog), `directories` 6.0.0 (settings folder, Videos folder), `chrono` 0.4.45 (local dates in
the Library). Already in the workspace: `serde`, `serde_json`, `toml`, `sha2`, `png`. Fonts built into the exe: IBM
Plex Sans, IBM Plex Mono and Noto Emoji (all SIL Open Font License 1.1).

**Spec:** `docs/studio/2026-11-10-ballfight-studio-design.md` (§ numbers below refer to it; §4, §5, §7 "Live
preview", §9, §11, §13 and §14 define this milestone). **Interface reference:**
`docs/studio/interface-reference.md` holds every measurement, colour, wording and egui note taken from the approved
mock-ups and the owner's design system; workers build from it and never need the artifacts. Engine interfaces come
from `docs/studio/2026-11-10-ballfight-studio-plan-m0-m1.md` (called "the M1 plan" below).

**Scope:** Spec milestone 2 only, plus what the M1 plan leaves to it: the queue's persistence and reordering, the
Library, the per-search-key shortlist cache, caption drafting from each fighter's emoji and tags, live preview and
sound. Not here: the other 28 fighters and 25 themes (milestones 3 and 4), the music system (5), ffmpeg bundling and
the Update button's function (6). By the owner's instruction of Nov 10 this plan is executed without waiting for
approval.

## Global Constraints

Every task's requirements include this section. The M1 plan's Global Constraints still bind every engine crate; the
lines below are the ones this milestone adds or leans on.

- **Block A touches only `studio/crates/bf-app/**`**, plus its own entries in `studio/Cargo.toml` and
  `studio/Cargo.lock` (Task 1), and is built in its own lane clone. It must not read, need or change any other crate
  except `bf-core`, of which it uses only `bf_core::grid::loop_bpm`. Block B may add to engine crates only where a task
  says so.
- **Nothing in `projects/`, `reelkit/` or `series/` is edited. GUITAR and SAX never appear** in any roster, list,
  sample data, test or file the app writes. No model or AI names in commits, code or docs.
- **The interface never blocks (§5).** The interface thread never simulates, draws a video frame, builds sound,
  encodes, opens a native dialog, or touches a file other than through `store::Writer` (Task 10) and the engine. It
  only sends commands, takes events and paints. Anything that can take over 4 ms goes behind the seam.
- **Honest fights (§3).** The app offers no control that edits a fight. Handicap is the only balance lever; tempo
  snaps with `bf_core::grid::loop_bpm`; a seed is chosen, never altered.
- **`bf-app` is not an engine crate.** It may use std maths for layout and drawing. It never computes a number that
  reaches a fight, a frame, a soundtrack or a fight file with std transcendentals; those values come from the engine
  crates.
- **Look (§11, the interface reference).** Greys only; colour belongs to fighters and never means a state, so errors
  are stated in words. Controls 26 tall, corners 2 (chips and rows 8), 1 px dividers, the inverted fill as the only
  "on" and primary signal. Reference window 1440 x 900, minimum 1280 x 800. Toolbar 40, bout bar 48, stage column
  410, story timeline 112, status bar 24.
- **Wording (§11).** Sentence case, plain verbs, buttons that say what happens. Empty and failed states say what to
  do next. No code names, no jargon ("seed" and "handicap" are the app's own words and stay).
- **Every control has an accessible name.** The test harness fails any control without one; never switch that check
  off.
- **Video and sound numbers:** 1,830 frames at 30 fps, internal 360 x 640; soundtrack 48,000 Hz stereo, exactly
  2,928,640 samples (61.013 s); 1,600 samples per frame. Safe zone: internal x 20 to 340, y 96 to 500, minus x >= 300
  below y 290.
- **Tests that draw with wgpu run one at a time.** Two software-adapter devices drawing at once crash the test
  process on Windows (seen while planning: about half of all parallel runs). Every such test takes the lock from
  `tests/ui/common.rs` (Task 1) first. Tests never open a real sound device, a native dialog or a window.
- **Seeds are Studio-owned.** Search tries seeds upward from 1; balance uses seeds from `1 << 40` upward.
- **Commits** are small and per task step 5, made by path, with the subject the step gives. `cargo fmt` and `cargo
  clippy --workspace --all-targets -- -D warnings` clean for the crates touched.

## Review Focus

Failure modes the spec implies but states no test for, most likely first. Each has a test in the task named.

1. **The output folder is missing or cannot be written** (deleted, a drive unplugged, read-only), when a video is
   added to the queue, when a job starts and when the Library opens. The app names the folder and says what to do
   (choose another, or create it); the job fails cleanly and the queue goes on; nothing crashes. Tasks 12, 13, 23,
   24, 33, 34.
2. **Saved files the app cannot use:** a settings file that is corrupt or half-written, a queue file that is corrupt
   or was written by a newer version, a shortlist cache from another version. The app starts with defaults, moves
   the unreadable file aside without deleting it, and says so in words. Tasks 10, 11, 13.
3. **Results that arrive late:** search results, balance results, a fight's description, a preview picture or its
   sound arriving after the fight's inputs changed or another seed was chosen. They are dropped; the screen never
   shows numbers or a picture for a different fight. Tasks 9, 25, 29, 30, 31.
4. **A job that fails or is interrupted:** a failed check, ffmpeg missing, a cancel, the app closed in the middle of
   a render. The failure is named with its numbers, nothing is saved, the queue continues with the next video, and
   after a restart the interrupted video is waiting again. Tasks 11, 12, 23, 25, 33.
5. **A small window and no sound:** at 1280 x 800 every control of every screen can still be reached and no primary
   button is cut off; on a PC with no sound device the preview still plays, silently, and says so. Tasks 16 to 25.

## Order of work

**Block A can start now**, while milestone 1 is still being built. It runs the way milestone 1's lanes do
(`lanes.md` in the build's scratch folder): in a clone of its own, `.lanes/app` on the branch `lane/app`, merged by
the controller task by task; side-by-side tasks get their own clones the same way. Tasks 1 to 3 come first, in
order. Then three groups that share no file:

- **Widgets:** 4, 5, 6, 7.
- **State and storage:** 8, then 9; 10, 11 and 13 can run beside 9; 12 needs 9 and 11.
- **Preview:** 14, then 15 and 16, which need Task 8 for the frame-source, sound and player types.
- **Screens** (need all three lanes): 17 first, then 18, then 19 to 24 in any order or side by side.
- **Together:** 25.

**Block B starts when milestone 1's Task 30 is merged** (the `ballfight` command line works end to end) and Block A
is done. 26 and 27 first (they add to engine crates), then 28; then 29, 30, 31, 33 and 34 in any order; 32 after 31;
35 last. Every Block B task begins by checking the M1 interfaces it uses against the code as merged.

Task 1 creates every module file of the crate, empty, and declares them all, so no later task edits `lib.rs`, a
`mod.rs` or `tests/ui/main.rs`; each task fills in its own files.

## What is live and what waits

| Place | In this milestone | Until its milestone |
|---|---|---|
| Music: theme, chord loop | Live: lists what the content has (one theme, one loop) | More arrive with milestones 3 to 5 |
| Music: who plays bass | Live for fighters whose classic part can play bass (TROMBONE); the other choice is drawn disabled with the reason | Milestone 5 gives every fighter both roles |
| Music: key | Shown, disabled: "E", "harmonic minor, from the theme" | Milestone 5 |
| Music: music seed, "Re-roll music" | Shown, disabled, with the line "New takes arrive with the music system. This version plays the series' own arrangement." | Milestone 5 |
| Music: arrangement lanes | Live: one lane per part of the built soundtrack, showing where it sounds, with the fight's markers | Milestone 5 adds named sections |
| Music: "What the music listens for" | Replaced by "Loudness" (the take's level checks, run as soon as it is built) and one stated line about what arrives later | Milestone 5 |
| Toolbar: "Update" | Drawn disabled; hovering says "Updates from inside the app arrive in a later version." | Milestone 6 |
| ffmpeg | Found on the PC (`BALLFIGHT_FFMPEG`, then `PATH`); Settings shows what was found; if none, Render says so and "Add to queue" is disabled | Milestone 6 bundles it |

## Performance budget for the preview

Targets from §7: live playback at 30 fps with headroom; scrubbing draws the asked frame on demand. Named inputs,
measured by milestone 1's Task 31 and written in `docs/studio/m1-measurements.md`: **`frame_ms_p95`** (one frame on
one thread), **`soundtrack_build_s`**, **`fights_per_second`**.

| Quantity | Budget | Where it is checked |
|---|---|---|
| Frames shown on time | at least 99 % over 20 s of playback from FIGHT!, search yielding | Task 32, `--preview-bench` |
| Threads the preview needs | `P = ceil(1.25 * frame_ms_p95 / 33.3)`; must be at most `workers` (cores minus one: 7 on the owner's PC, so `frame_ms_p95` up to 186 ms). One thread is enough up to 26.7 ms | Task 32 |
| If `P > workers` | the scheduler draws every second frame and holds it; sound never stalls; Task 32 reports it against Task 31's "Speed plan" | Tasks 15, 32 |
| Draw-ahead buffer | 12 frames (0.4 s) ahead of the playhead; 64 frames kept (44 MB) | Task 15 |
| Scrub | the newest asked frame is on screen within `frame_ms_p95` + 20 ms; older requests are skipped | Tasks 15, 32 |
| Interface thread | no single update over 4 ms besides painting: one 360 x 640 texture upload per shown frame, channel sends | Tasks 17, 35 |
| First picture after choosing a seed | within one fight simulation, one picture set-up and one frame; playback may start silent and the sound joins when built (`soundtrack_build_s`) | Tasks 16, 31 |
| While a video renders | the render keeps its `workers`; the preview may skip frames and the status bar says so | Task 33 |

## File map

```
studio/Cargo.toml  Cargo.lock                    one member and eight dependency lines added (Task 1)
studio/crates/bf-app/
  Cargo.toml  kittest.toml  .gitignore  README.md
  assets/fonts/      IBMPlexSans-{Regular,Medium,SemiBold}.ttf  IBMPlexMono-Regular.ttf  NotoEmoji-Variable.ttf
                     OFL-IBM-Plex.txt  OFL-Noto-Emoji.txt  SOURCES.md
  src/main.rs        arguments, storage folder, which engine, eframe::run_native
  src/lib.rs         module list
  src/app.rs         BfApp: eframe::App (logic = events in, ui = paint, actions out)
  src/theme/         mod.rs tokens.rs fonts.rs visuals.rs
  src/widgets/       mod.rs buttons.rs icons.rs slider.rs fields.rs pixel.rs chips.rs ribbon.rs
  src/model/         mod.rs draft.rs roster.rs story.rs shortlist.rs balance.rs take.rs queue.rs library.rs
  src/seam/          mod.rs (Command, Event, EngineHandle)  sample.rs (sample data)  fake.rs (sample engine)
  src/state/         mod.rs (AppState, ViewModel, Action, Effect)  fight.rs  seeds.rs  queue_run.rs  events.rs
  src/store/         mod.rs (Paths, write_atomic, Writer)  settings.rs  queue_file.rs  shortlist_cache.rs  library.rs
  src/preview/       mod.rs clock.rs scale.rs scheduler.rs audio.rs player.rs
  src/screens/       mod.rs shell.rs stage.rs timeline.rs matchup.rs balance.rs seeds.rs music.rs render.rs
                     library.rs settings.rs
  src/wording.rs     shared text formatters
  src/fixtures/      mod.rs base.rs matchup.rs balance.rs seeds.rs music.rs render.rs library.rs settings.rs
  src/engine/        (Block B) mod.rs convert.rs content.rs balance.rs search.rs takes.rs jobs.rs files.rs
  tests/ui/          main.rs common.rs and one module per task (smoke, fonts, theme, buttons, fields, pixel,
                     ribbon, shell, timeline, matchup, balance, seeds, music, render, library, settings, flows,
                     wording, small_window, real)
  tests/snapshots/   the reference pictures (*.png)
studio/crates/bf-content/src/cards.rs  starter.rs        (Block B, Task 26)
docs/studio/interface-reference.md                       what the mock-ups and the design system say
docs/studio/m2-measurements.md                           written in Tasks 32 and 35
```

Pure logic is unit-tested inside its own file (`#[cfg(test)] mod tests`, run with `cargo test -p bf-app --lib
<module path>`). Everything that needs egui is tested in the one integration binary `tests/ui` (run with `cargo test
-p bf-app --test ui <module>::`), so the crate links one test program and the wgpu lock covers every drawing test.

---

# Block A: the interface on sample data

### Task 1: The crate, pinned dependencies and the test harness

**Files:**
- Modify: `studio/Cargo.toml` (add `"crates/bf-app"` to `members`; add the dependency lines below to
  `[workspace.dependencies]`), `studio/Cargo.lock`
- Create: `studio/crates/bf-app/Cargo.toml`, `kittest.toml`, `.gitignore`, `README.md`, `src/main.rs`, `src/lib.rs`,
  `src/app.rs`, every other file under `src/` in the file map except `src/engine/` (empty, or holding only its `pub
  mod` lines; `src/model/roster.rs`, `story.rs` and `take.rs` hold the five types below), `tests/ui/main.rs`,
  `tests/ui/common.rs`, `tests/ui/smoke.rs` and one empty file per other module listed under `tests/ui/` in the
  file map

**Interfaces:**
- Produces:
  ```toml
  # studio/Cargo.toml, [workspace.dependencies]
  bf-app = { path = "crates/bf-app" }
  chrono = { version = "=0.4.45", default-features = false, features = ["clock", "std"] }
  cpal = "=0.18.2"
  directories = "=6.0.0"
  eframe = { version = "=0.37.0", default-features = false, features = ["wgpu", "accesskit", "clipboard", "system_fonts", "x11", "wayland"] }
  egui = { version = "=0.37.0", default-features = false }
  egui_kittest = { version = "=0.37.0", features = ["snapshot", "wgpu", "eframe"] }
  rfd = "=0.18.0"
  ```
  `bf-app/Cargo.toml`: package `bf-app`, library `bf_app`, binary `ballfight-studio` (`src/main.rs`); dependencies
  `bf-core`, `chrono`, `cpal`, `directories`, `eframe`, `egui`, `png`, `rfd`, `serde`, `serde_json`, `sha2`, `toml`
  (all `.workspace = true`); dev-dependency `egui_kittest.workspace = true`.
  ```rust
  // src/app.rs
  pub struct BfApp { /* grows in Task 17 */ }
  impl BfApp { pub fn blank(cc: &eframe::CreationContext<'_>) -> BfApp; }   // paints the window `ground` (#1e1e1e)
  impl eframe::App for BfApp { fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame); }
  // tests/ui/common.rs
  pub fn gpu() -> std::sync::MutexGuard<'static, ()>;   // the one-at-a-time lock; ignores poisoning
  pub const REF: (f32, f32) = (1440.0, 900.0);
  pub const MIN: (f32, f32) = (1280.0, 800.0);
  pub fn app_harness<'a>(size: (f32, f32), make: impl FnOnce(&mut eframe::CreationContext<'a>) -> bf_app::app::BfApp)
      -> egui_kittest::Harness<'a, bf_app::app::BfApp>;   // edge to edge, 1 pixel per point, no cursor drawn, wgpu
  pub fn widget_harness<'a>(size: (f32, f32), ui: impl FnMut(&mut egui::Ui) + 'a) -> egui_kittest::Harness<'a>;
  pub fn pixels<S>(harness: &mut egui_kittest::Harness<'_, S>, at: &[(u32, u32)]) -> Vec<[u8; 3]>;   // renders once
  // src/model/roster.rs, story.rs and take.rs: the five small types the Widgets lane and the State lane both use
  #[derive(Clone, Debug, PartialEq)]
  pub struct PixelMask { pub w: usize, pub h: usize, pub on: Vec<bool> }   // row-major; h is 7 for one line of text
  #[derive(Clone, Debug, PartialEq)]
  pub struct Sprite { pub w: usize, pub h: usize, pub rgba: std::sync::Arc<[u8]> }   // a fighter's portrait
  #[derive(Clone, Debug, PartialEq)]
  pub struct Frame { pub w: u32, pub h: u32, pub rgb: std::sync::Arc<[u8]> }         // one picture, 360 x 640 unless said
  #[derive(Clone, Debug, PartialEq)]
  pub struct AbilityMark { pub t: f32, pub who: usize, pub name: String }   // first use of an ability; name as shown ("Snake dance")
  #[derive(Clone, Debug, PartialEq)]
  pub struct StoryView { pub seed: u64, pub winner: Option<usize>, pub t_ko: Option<f64>, pub winner_hp: f64,
      pub lead_changes: u32, pub score: f64, pub fits: bool, pub misses: Vec<String>, pub t_fight: f64,
      pub lead: Vec<(f32, f32)>,          // (seconds, hp[0] - hp[1]) after each hit
      pub firsts: Vec<AbilityMark> }
  ```
  `main.rs`: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`; window title "BALLFIGHT
  Studio", inner size 1440 x 900, minimum inner size 1280 x 800; the flag `--smoke` closes the window after 30
  painted frames and exits with code 0. `kittest.toml` holds `output_path = "tests/snapshots"`. `.gitignore` holds
  `tests/snapshots/**/*.new.png` and `tests/snapshots/**/*.diff.png`. `lib.rs`, each `mod.rs` and
  `tests/ui/main.rs` declare every module of the file map now (`src/engine/` excepted).
- [ ] **Step 1: Write the failing tests** in `tests/ui/smoke.rs`: `the_test_adapter_is_software` (take `gpu()`;
  build `egui_kittest::wgpu::create_render_state(egui_kittest::wgpu::default_wgpu_setup(),
  eframe::egui_wgpu::RendererOptions::PREDICTABLE)`; `format!("{:?}", state.adapter.get_info().device_type) ==
  "Cpu"`; print the adapter's name) and `the_app_fills_a_1440x900_window` (take `gpu()`; `app_harness(REF, |cc|
  BfApp::blank(cc))`; `harness.run()`; `harness.render()` gives an image 1440 wide and 900 tall; `pixels(&mut
  harness, &[(0, 0), (720, 450), (1439, 899)])` are all `[0x1e, 0x1e, 0x1e]`).
- [ ] **Step 2:** `cd studio && cargo test -p bf-app --test ui smoke::`. Expected: FAIL (the crate does not exist).
- [ ] **Step 3: Implement.** `app_harness` is `Harness::builder().with_size(egui::Vec2::new(w, h))
  .with_pixels_per_point(1.0).with_render_cursor(false).wgpu().build_eframe(make)`; `widget_harness` the same with
  `.with_context_setup(|ctx| bf_app::theme::install(ctx))` and `.build_ui(ui)` (until Task 3 fills `install` it is
  an empty function). Do not add eframe's `default_fonts` feature: the app carries its own fonts (Task 2), so until
  Task 2 a test must not draw text.
- [ ] **Step 4:** Run: PASS, twenty times in a row (`for i in $(seq 20); do cargo test -p bf-app --test ui smoke::
  || break; done`), to prove the lock. `cargo build --release -p bf-app`, then run `target/release/
  ballfight-studio.exe --smoke` once: a window opens and closes by itself within 3 s, exit code 0. Write
  `README.md`: what the crate is, how to run the app and its tests.
- [ ] **Step 5:** Commit `bf-app: crate, pinned dependencies, test harness`.

### Task 2: Fonts in the exe

**Files:**
- Create: `studio/crates/bf-app/assets/fonts/` (the five font files, the two licence texts, `SOURCES.md`),
  `src/theme/fonts.rs`, `tests/ui/fonts.rs`

**Interfaces:**
- Consumes: the `tests/ui` test program (Task 1).
- Produces:
  ```rust
  pub fn definitions() -> egui::FontDefinitions;
  pub fn medium() -> egui::FontFamily;      // FontFamily::Name("medium".into())
  pub fn semibold() -> egui::FontFamily;    // FontFamily::Name("semibold".into())
  pub const FILES: [(&str, &[u8]); 5];      // file name and bytes, for the size test
  ```
  Families: Proportional = Plex Sans Regular, then Noto Emoji; Monospace = Plex Mono Regular, Plex Sans Regular, Noto
  Emoji; `medium` = Plex Sans Medium, Noto Emoji; `semibold` = Plex Sans SemiBold, Noto Emoji.
- **Where the files come from** (the table with sizes and git blob ids is in the interface reference, section 10).
  Download each with `curl -L -o <file> <url>` into `assets/fonts/`:
  `https://raw.githubusercontent.com/IBM/plex/1da12f02587b630c07e92692d21492d722f53614/packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Regular.ttf`
  (and `-Medium`, `-SemiBold`, and `license.txt` from the same folder, saved as `OFL-IBM-Plex.txt`);
  `https://raw.githubusercontent.com/IBM/plex/2f9ba1b25957d958db71a849e85d72e3ecfb845a/packages/plex-mono/fonts/complete/ttf/IBMPlexMono-Regular.ttf`;
  `https://raw.githubusercontent.com/google/fonts/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/notoemoji/NotoEmoji%5Bwght%5D.ttf`
  saved as `NotoEmoji-Variable.ttf`, and `OFL.txt` from the same folder saved as `OFL-Noto-Emoji.txt`. Check each
  with `git hash-object <file>` against the blob id in the interface reference; a file that does not match is not
  used. `SOURCES.md` lists, per file: the URL, the licence (SIL Open Font License 1.1), the byte count, the blob id
  and the SHA-256.
- [ ] **Step 1: Write the failing tests.** `the_font_files_are_the_pinned_ones`: the byte lengths in `FILES` are
  200_500, 202_460, 202_632, 173_052 and 1_982_596, in the order Regular, Medium, SemiBold, Mono, Noto.
  `every_character_the_app_shows_has_a_glyph`: in a harness built directly
  (`egui_kittest::Harness::builder().with_context_setup(|ctx| ctx.set_fonts(fonts::definitions())).build_ui(..)`:
  nothing is drawn with wgpu, so no lock), lay out one label per family (Proportional, Monospace, `medium()`,
  `semibold()`) holding every character from U+0020 to U+007E plus `×`, `…`, `’`, `–`; and one Proportional label
  holding the emoji U+1FA88 (flute), U+1F3BA (trumpet), U+1F447 (pointing down), U+1F3B7 (saxophone), U+1F3BB (violin),
  U+1F941 (drum), U+1F3B9 (keyboard), U+1FA97 (accordion), U+1FA89 (harp). The harness panics on any character without a
  glyph, which is the assertion. Do not wrap it in `catch_unwind`.
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui fonts::`. Expected: FAIL.
- [ ] **Step 3:** Fetch and check the files, write `SOURCES.md`, implement `fonts.rs` with `include_bytes!` and
  `egui::FontData::from_static`. A variable font loads at its default weight (400), which is the one wanted.
- [ ] **Step 4:** Run: PASS. If the Noto file does not load or lacks one of the listed emoji, do not swap in another
  font: leave Noto out of the families, remove the emoji from this test, and report DONE_WITH_CONCERNS naming the
  missing glyphs (the real app still draws emoji through the PC's own fonts, which eframe's `system_fonts` feature
  finds; Task 23's sample caption then drops its emoji).
- [ ] **Step 5:** Commit `bf-app: IBM Plex and Noto Emoji built in`.

### Task 3: Theme: tokens and visuals

**Files:**
- Create: `src/theme/tokens.rs`, `src/theme/visuals.rs`, `src/theme/mod.rs` (re-exports), `tests/ui/theme.rs`

**Interfaces:**
- Consumes: `fonts::{definitions, medium, semibold}`.
- Produces:
  ```rust
  // theme::tokens (all egui::Color32 unless noted)
  pub const GROUND: Color32;      // #1e1e1e
  pub const PANEL: Color32;       // #282828
  pub const STAGE: Color32;       // #2b2b2b
  pub const RAISED: Color32;      // #333333
  pub const SELECTED: Color32;    // #383838
  pub const HOVER: Color32;       // #3a3a3a
  pub const LINE_SOFT: Color32;   // #383838
  pub const LINE: Color32;        // #444444
  pub const FAINT: Color32;       // #777777
  pub const MUTED: Color32;       // #a0a0a0
  pub const TEXT: Color32;        // #e2e2e2
  pub const TEXT_STRONG: Color32; // #ffffff
  pub const ON_LIGHT: Color32;    // #1e1e1e
  pub const HAIRLINE: Color32;    // #303030
  pub const GROUP: Color32;       // #2e2e2e
  pub const BAR_DIM: Color32;     // #666666
  pub const BAND_CLIP: Color32;   // #3a3a3a
  pub const CONTROL_H: f32 = 26.0;  pub const VALUE_H: f32 = 22.0;   pub const ROW_H: f32 = 30.0;
  pub const TOOLBAR_H: f32 = 40.0;  pub const BOUT_H: f32 = 48.0;    pub const STAGE_W: f32 = 410.0;
  pub const TIMELINE_H: f32 = 112.0; pub const TRANSPORT_H: f32 = 30.0; pub const STATUS_H: f32 = 24.0;
  pub const LABEL_W: f32 = 108.0;   pub const VALUE_W: f32 = 62.0;   pub const LANE_NAME_W: f32 = 120.0;
  pub const RADIUS: u8 = 2;         pub const RADIUS_ROW: u8 = 8;
  pub fn rgb(c: [u8; 3]) -> Color32;
  pub fn mix(over: Color32, base: Color32, share: f32) -> Color32;   // per channel round(base + share * (over - base))
  pub fn large_fill(fighter: [u8; 3]) -> Color32;                    // mix(rgb(fighter), PANEL, 0.22)
  pub fn body() -> egui::FontId;      // Plex Sans 13
  pub fn small() -> egui::FontId;     // Plex Sans 12
  pub fn section() -> egui::FontId;   // Plex Sans SemiBold 13
  pub fn strong() -> egui::FontId;    // Plex Sans Medium 13
  pub fn title() -> egui::FontId;     // Plex Sans SemiBold 18
  pub fn value() -> egui::FontId;     // Plex Mono 12
  pub fn timecode() -> egui::FontId;  // Plex Mono 14
  // theme
  pub fn install(ctx: &egui::Context);   // fonts, visuals, spacing and text styles, for every egui theme
  ```
- [ ] **Step 1: Write the failing tests.** Unit, in `tokens.rs`: `large_fill([0xde, 0xe8, 0xf6]) ==
  Color32::from_rgb(0x50, 0x52, 0x55)`; `large_fill([0x78, 0xf0, 0xb8]) == Color32::from_rgb(0x3a, 0x54, 0x48)`;
  `mix(TEXT, PANEL, 0.0) == PANEL`; `mix(TEXT, PANEL, 1.0) == TEXT`. In `tests/ui/theme.rs`,
  `the_design_systems_egui_notes_hold`: inside a `widget_harness`, read `ui.style()` and assert
  `visuals.panel_fill == PANEL`, `visuals.window_fill == PANEL`, `visuals.extreme_bg_color == GROUND`,
  `visuals.faint_bg_color == RAISED`, `visuals.window_stroke == Stroke::new(1.0, LINE)`,
  `visuals.widgets.inactive.corner_radius == CornerRadius::same(2)`, `visuals.widgets.inactive.weak_bg_fill ==
  Color32::TRANSPARENT`, `visuals.widgets.hovered.weak_bg_fill == HOVER`, `visuals.widgets.active.weak_bg_fill ==
  TEXT`, `visuals.widgets.active.fg_stroke.color == ON_LIGHT`, `visuals.selection.bg_fill == TEXT`,
  `spacing.item_spacing == vec2(8.0, 6.0)`, `spacing.button_padding == vec2(10.0, 5.0)`, `spacing.interact_size.y ==
  26.0`, and `text_styles[&TextStyle::Body] == body()`, `[Button] == body()`, `[Small] == small()`, `[Monospace] ==
  value()`, `[Heading] == title()`. `plain_text_is_the_text_colour` (takes `gpu()`): a label
  "Flute" drawn on a `PANEL` background renders at least one pixel equal to `[0xe2, 0xe2, 0xe2]`.
- [ ] **Step 2:** Run `cargo test -p bf-app --lib theme::` and `--test ui theme::`. Expected: FAIL.
- [ ] **Step 3:** Implement with `ctx.set_fonts(definitions())` and `ctx.all_styles_mut(|style| ...)`; set
  `visuals.override_text_color = Some(TEXT)`, the open state like the active state, `visuals.window_shadow` and
  `popup_shadow` to the menu shadow (offset 0 x 8, blur 20, `#000000aa`), `visuals.menu_corner_radius` 2.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: theme tokens and visuals`.

### Task 4: Buttons, segments, switch, dropdown, icons, progress bar

**Files:**
- Create: `src/widgets/buttons.rs`, `src/widgets/icons.rs`, `tests/ui/buttons.rs`,
  `tests/snapshots/widgets_buttons.png`

**Interfaces:**
- Consumes: `theme::tokens`, `widget_harness`, `gpu`, `pixels`. Control specs: interface reference, section 3.
- Produces (every function gives its control an accessible name: the label, or `name`):
  ```rust
  // widgets::buttons
  pub fn flat(ui: &mut egui::Ui, label: &str) -> egui::Response;                 // 26 tall, transparent until hovered
  pub fn toggle(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response;      // flat; inverted when on
  pub fn boxed(ui: &mut egui::Ui, label: &str) -> egui::Response;                 // RAISED with a LINE border
  pub fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response;               // TEXT fill, ON_LIGHT label
  pub fn disabled(ui: &mut egui::Ui, label: &str, why: &str) -> egui::Response;   // FAINT text, no hover fill, `why` on hover
  pub fn segmented(ui: &mut egui::Ui, name: &str, options: &[(&str, Option<&str>)], chosen: Option<usize>, width: f32)
      -> Option<usize>;   // each option: label and, when it cannot be chosen, the reason; returns the clicked index
  pub fn switch(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response;      // 30 x 16 square switch, then the label
  pub fn dropdown(ui: &mut egui::Ui, name: &str, options: &[String], chosen: usize, width: f32) -> Option<usize>;
  pub fn progress(ui: &mut egui::Ui, name: &str, share: f32, width: f32) -> egui::Response;   // 4 tall
  // widgets::icons
  pub enum Icon { Play, Pause, Done, Now, Todo, Chevron }
  pub fn paint(painter: &egui::Painter, rect: egui::Rect, icon: Icon, color: egui::Color32);   // 12 x 12 box, 10 x 10 for Chevron
  pub fn icon_button(ui: &mut egui::Ui, icon: Icon, name: &str, enabled: bool) -> egui::Response;   // 24 x 26
  ```
- [ ] **Step 1: Write the failing tests** (each takes `gpu()` and draws on a `PANEL` background): `heights_are_26`:
  `flat`, `toggle`, `boxed`, `primary` and `disabled`, each alone in a harness and labelled "Swap sides", give
  `get_by_label("Swap sides").rect().height() == 26.0`. `fills_match_the_tokens`: the pixel 3 right and 3 below the
  top-left corner of each button is `PANEL` for `flat`, `[0xe2; 3]` for `primary` and for `toggle(.., true)`, `[0x33;
  3]` for `boxed`. `a_segment_click_returns_its_index`: `segmented(ui, "Winner", &[("Any", None), ("Flute", None),
  ("Trombone", None)], Some(0), 206.0)`; clicking "Trombone" returns `Some(2)`; a segment with a reason (`("Flute",
  Some("Flute has no bass part"))`) returns nothing when clicked. `the_switch_reports_clicks_and_its_state`: `switch(ui,
  "Show fights outside the beat sheet", false)`; the node found by that label has `accesskit_node().toggled() ==
  Some(Toggled::False)`; after a click the response's `clicked()` was true. `the_dropdown_lists_and_returns`:
  `dropdown(ui, "Theme", &["Big top".into(), "Dark disco".into()], 0, 220.0)`; click the control named "Theme", then
  "Dark disco": returns `Some(1)`. `nothing_is_unnamed`: `harness.unnamed_widgets().is_empty()` with one of each control
  on screen, including `icon_button(ui, Icon::Play, "Play", true)` and `progress(ui, "Drawing frames", 0.64, 200.0)`.
  `the_gallery_matches`: one of each control in a 420 x 330 harness, `harness.snapshot("widgets_buttons")`.
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui buttons::`. Expected: FAIL.
- [ ] **Step 3:** Implement. Buttons are `egui::Button` with explicit fill, stroke and `min_size`; the switch, the
  progress bar and the icons are painted by hand on an allocated rect and named with `response.widget_info(...)`
  (`Role::Switch` with its state, `Role::ProgressIndicator`, `Role::Button`); the dropdown is an
  `egui::ComboBox::from_id_salt(name)` given its name with `.labelled_by` on an invisible label, its list styled as
  the menu. Icon coordinates are in the interface reference, section 3.
- [ ] **Step 4:** Run with `UPDATE_SNAPSHOTS=true` once, open the picture and compare it with section 3 of the
  interface reference; then run without it: PASS.
- [ ] **Step 5:** Commit `bf-app: buttons, segments, switch, dropdown, icons`.

### Task 5: Slider, value field and text fields

**Files:**
- Create: `src/widgets/slider.rs`, `src/widgets/fields.rs`, `tests/ui/fields.rs`, `tests/snapshots/widgets_fields.png`

**Interfaces:**
- Consumes: `theme::tokens`, the harness helpers.
- Produces:
  ```rust
  // widgets::slider
  pub fn knob_x(value: f64, range: (f64, f64), left: f32, width: f32) -> f32;
  pub fn value_at(x: f32, range: (f64, f64), step: f64, left: f32, width: f32) -> f64;   // clamped, snapped to step
  pub struct SliderOut { pub changed: bool, pub released: bool }
  pub fn bar_slider(ui: &mut egui::Ui, name: &str, value: &mut f64, range: (f64, f64), step: f64, width: f32) -> SliderOut;
  // widgets::fields
  pub struct Unit<'a> { pub prefix: &'a str, pub suffix: &'a str, pub decimals: usize }
  pub fn value_field(ui: &mut egui::Ui, name: &str, value: &mut f64, range: (f64, f64), unit: Unit<'_>, width: f32) -> egui::Response;
  pub fn read_field(ui: &mut egui::Ui, name: &str, text: &str, width: f32) -> egui::Response;   // a well that cannot be edited
  pub fn text_line(ui: &mut egui::Ui, name: &str, text: &mut String, hint: &str, width: f32) -> egui::Response;
  pub fn text_box(ui: &mut egui::Ui, name: &str, text: &mut String, rows: usize) -> egui::Response;   // fills the width
  ```
  The slider is a 2 px `LINE` track, a `MUTED` fill up to the knob and a 3 x 14 `TEXT_STRONG` knob, in a row 26 tall;
  it takes drags and clicks anywhere on its row, and with the keyboard on it the left and right arrows move one
  `step`. `released` is true on the frame a drag or a click ends.
- [ ] **Step 1: Write the failing tests.** Unit: `knob_x(2.0, (0.05, 3.0), 100.0, 200.0)` is within 0.01 of
  `100.0 + 200.0 * (1.95 / 2.95)`; `value_at(100.0 + 0.66 * 200.0, (0.05, 3.0), 0.01, 100.0, 200.0) == 2.0`;
  `value_at(-50.0, ..) == 0.05`; `value_at(9999.0, ..) == 3.0`; `value_at(x, (24.0, 42.0), 1.0, ..)` is always a
  whole number. UI: `dragging_changes_then_releases` (drag from the knob to the row's right end with
  `harness.drag_at` and `harness.drop_at`: the value ends at the range's top, `changed` was seen, `released` was
  seen exactly once); `the_value_field_shows_its_unit` (`value_field(.., &mut 2.0, (0.05, 12.0), Unit { prefix: "×",
  suffix: "", decimals: 2 }, 62.0)` has width 62, height 22, and the text "×2.00" is on screen; with `Unit { prefix:
  "", suffix: " s", decimals: 1 }` and 49.0 the text is "49.0 s"); `typing_into_a_text_line` (focus the field found
  by its hint "Seed number", type "20418": the bound string is "20418"); `nothing_is_unnamed`;
  `the_gallery_matches` (`harness.snapshot("widgets_fields")`).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib widgets::slider` and `--test ui fields::`. Expected: FAIL.
- [ ] **Step 3:** Implement. The slider is painted by hand with `Sense::click_and_drag()` and named with
  `WidgetInfo::slider`; the value field is an `egui::DragValue` with `prefix`, `suffix`, `fixed_decimals`, `range`
  and `speed` one hundredth of the range, in a `GROUND` well, tied to an invisible label carrying `name`; text
  fields are `egui::TextEdit` with `hint_text` and the `GROUND` well.
- [ ] **Step 4:** Bless the picture after comparing it with section 3 of the interface reference; run: PASS.
- [ ] **Step 5:** Commit `bf-app: slider, value field, text fields`.

### Task 6: Pixel text, fighter marks, chips and rows

**Files:**
- Create: `src/widgets/pixel.rs`, `src/widgets/chips.rs`, `tests/ui/pixel.rs`, `tests/snapshots/widgets_chips.png`

**Interfaces:**
- Consumes: `PixelMask { w, h, on }`, `Sprite { w, h, rgba }`, `Frame { w, h, rgb }` (Task 1), `theme::tokens`, the
  harness helpers.
- Produces:
  ```rust
  // widgets::pixel
  pub fn fit_scale(mask_w: usize, max_scale: u32, avail_w: f32) -> u32;   // largest scale <= max_scale whose width fits; at least 1
  pub fn pixel_text(ui: &mut egui::Ui, mask: &PixelMask, scale: u32, color: egui::Color32, name: &str) -> egui::Response;
  // widgets::chips
  pub fn fighter_bar(ui: &mut egui::Ui, color: [u8; 3], height: f32) -> egui::Response;   // 4 wide, corner 1
  pub fn portrait(ui: &mut egui::Ui, color: [u8; 3], sprite: Option<&Sprite>, size: f32, name: &str) -> egui::Response;
  pub fn picture(ui: &mut egui::Ui, key: &str, frame: &Frame, size: egui::Vec2, name: &str) -> egui::Response;
  pub struct ChipOut { pub left: bool, pub right: bool }   // clicked, right-clicked
  pub fn roster_chip(ui: &mut egui::Ui, display: &str, color: [u8; 3], side: Option<usize>, available: bool, width: f32) -> ChipOut;
  pub fn object_row<R>(ui: &mut egui::Ui, name: &str, selected: bool, height: f32, add: impl FnOnce(&mut egui::Ui) -> R)
      -> (egui::Response, R);   // corner 8; selected look = SELECTED fill and a 1.5 px TEXT ring drawn inside
  ```
  `pixel_text` takes exactly `mask.w * scale` by `mask.h * scale` points and paints one square per lit pixel, snapped
  to whole pixels. A chip's accessible name is "Flute", "Flute, left corner" or "Flute, right corner"; a chip that
  is not available is drawn `FAINT`, takes no clicks and says "Not built yet. It arrives in an update." on hover.
  Without a sprite the portrait is a disc filled with `large_fill(color)` inside a 3 px ring of the full colour.
  `picture` and a portrait's sprite are drawn with nearest filtering; their texture is made once per `key` (or
  name), kept in egui's temporary data (`ctx.data_mut`), and made again when the pixels are another allocation.
- [ ] **Step 1: Write the failing tests.** Unit: `fit_scale(29, 4, 172.0) == 4` (FLUTE is 29 wide); `fit_scale(47,
  4, 172.0) == 3` (TROMBONE is 47 wide); `fit_scale(47, 4, 20.0) == 1`. UI: `pixel_text_is_exact` (a 5 x 7 mask of
  the letter F, rows `#####`, `#....`, `#....`, `####.`, `#....`, `#....`, `#....`, at scale 3 in white: the response
  is 15 x 21; the pixel 1 right and 1 below its top-left corner is white; the pixel at 13 right and 4 below is the
  background); `a_chip_reports_left_and_right_clicks` (`.click()` gives `left`, `.click_secondary()` gives
  `right`); `a_chip_that_is_not_built_takes_no_clicks`; `the_selected_chip_is_ringed` (with `side: Some(0)` the
  pixel just inside the chip's left edge at mid height is the fighter's colour, the small label "Left" is on screen
  and the name is "Flute, left corner"); `nothing_is_unnamed`; `the_gallery_matches`
  (`harness.snapshot("widgets_chips")`: FLUTE and TROMBONE names at scales 3 and 4, both portraits, four chips of
  which one is chosen and one not built, one selected and one plain `object_row`).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib widgets::pixel` and `--test ui pixel::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after comparing with sections 3 and 5.1 of the interface reference; run: PASS.
- [ ] **Step 5:** Commit `bf-app: pixel text, fighter marks, chips and rows`.

### Task 7: The lead ribbon

**Files:**
- Create: `src/widgets/ribbon.rs`, `tests/ui/ribbon.rs`, `tests/snapshots/widgets_ribbon.png`

**Interfaces:**
- Consumes: `model::story::{StoryView, AbilityMark}` (Task 1: `StoryView { seed, winner: Option<usize>, t_ko:
  Option<f64>, winner_hp, lead_changes, score, fits, misses, t_fight: f64, lead: Vec<(f32, f32)>, firsts }`),
  `theme::tokens`, the harness helpers.
- Produces:
  ```rust
  // widgets::ribbon
  pub const DUR: f32 = 61.0;
  pub fn x_of(t: f32, left: f32, width: f32) -> f32;        // left + t / 61 * width
  pub fn t_of(x: f32, left: f32, width: f32) -> f32;        // clamped to 0..=61
  pub fn lead_height(lead: f32, mid: f32) -> f32;           // min(|lead| * (mid - 1) / 42, mid - 1)
  pub fn curve(story: &StoryView) -> Vec<(f32, f32)>;       // (t, lead): starts (t_fight, 0), one point per hit, ends (t_ko, 0) when there is a K.O.
  pub fn mesh(story: &StoryView, rect: egui::Rect, colors: [egui::Color32; 2]) -> egui::Mesh;
  pub fn paint(painter: &egui::Painter, rect: egui::Rect, story: &StoryView, colors: [[u8; 3]; 2], ko_window: bool);
  pub fn small(ui: &mut egui::Ui, story: &StoryView, colors: [[u8; 3]; 2], width: f32) -> egui::Response;   // 28 tall, takes no input
  ```
  `paint` draws, in order: the K.O. window (49 to 53 s, `RAISED`, full height) when asked; the 1 px `LINE` centre
  line; the mesh; a 2 px `TEXT_STRONG` bar at the K.O., inset 2 above and below (none when `t_ko` is `None`). The
  mesh is a strip: for each pair of neighbouring curve points, one quad between the centre line and the heights of
  the positive parts (left fighter's colour, above) and one for the negative parts (right fighter's colour, below);
  a part's height at a point is `lead_height(max(lead, 0), mid)` above or `lead_height(max(-lead, 0), mid)` below.
- [ ] **Step 1: Write the failing tests.** Unit: `x_of(30.5, 0.0, 1320.0) == 660.0`; `t_of(660.0, 0.0, 1320.0) ==
  30.5`; `t_of(-5.0, ..) == 0.0`; `t_of(99999.0, ..) == 61.0`; `(x_of(49.0, 0.0, 1320.0) - 1060.3).abs() < 0.1` and
  `(x_of(53.0, 0.0, 1320.0) - x_of(49.0, 0.0, 1320.0) - 86.6).abs() < 0.1`; `lead_height(42.0, 20.5) == 19.5`;
  `lead_height(100.0, 20.5) == 19.5`; `lead_height(21.0, 20.5) == 9.75`; `lead_height(0.0, 20.5) == 0.0`. For a
  story with `t_fight 3.6`, `lead [(10.0, 21.0), (20.0, -21.0)]`, `t_ko Some(51.0)`: `curve` is `[(3.6, 0.0), (10.0,
  21.0), (20.0, -21.0), (51.0, 0.0)]`; in `mesh` over a rect 400 x 28 every vertex above the centre line has the
  first colour, every vertex below it the second, none lies outside the rect, and there are `2 * 3 * 2` triangles;
  with `t_ko: None` the curve ends at the last hit. UI: `the_small_ribbon_is_28_tall` and `the_gallery_matches`
  (`harness.snapshot("widgets_ribbon")`: that story at 400 x 28, and at 1320 x 41 with the K.O. window, on `PANEL`,
  in the FLUTE and TROMBONE colours).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib widgets::ribbon` and `--test ui ribbon::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after comparing with section 4 of the interface reference ("The lead ribbon"); run: PASS.
- [ ] **Step 5:** Commit `bf-app: the lead ribbon`.

### Task 8: The model, the seam and the sample data

**Files:**
- Create: `src/model/{mod.rs,draft.rs,shortlist.rs,balance.rs,queue.rs,library.rs}` (types only; the queue's
  methods are Task 11), `src/seam/mod.rs`, `src/seam/sample.rs`, `src/wording.rs`
- Modify: `src/model/roster.rs` and `src/model/take.rs` (add to what Task 1 made)

**Interfaces:**
- Consumes: `bf_core::grid::loop_bpm(bpm: f64, dur: f64, beats_per_bar: u32) -> f64`; `PixelMask`, `Sprite`,
  `Frame`, `StoryView`, `AbilityMark` (Task 1).
- Produces the plain data everything else shares. All derive `Clone` and `Debug`; all but those holding a `dyn`
  also `PartialEq`; `Draft`, `CaptionTexts`, `Settings`, `Queue`, `Job`, `JobSpec`, `JobState`, `StepLine`, `StepKey`,
  `StepState` and `FailHint` also `serde::Serialize` and `Deserialize`.
  ```rust
  // model (mod.rs)
  pub type Gen = u64;  pub type TakeId = u64;  pub type JobId = u64;
  pub const FPS: u32 = 30;  pub const NF: u32 = 1_830;  pub const SR: u32 = 48_000;  pub const NA: u64 = 2_928_640;
  pub const SAMPLES_PER_FRAME: u64 = 1_600;
  pub enum Step { Matchup, Balance, Seeds, Music, Render }
  pub enum View { Step(Step), Library, Settings }
  pub enum Platform { Instagram, TikTok, Shorts }
  pub struct Notice { pub text: String }
  pub struct Settings { pub output_folder: std::path::PathBuf, pub workers: usize }
  // model::draft
  pub struct CaptionTexts { pub instagram: String, pub tiktok: String, pub shorts: String }
  pub struct Draft { pub fighters: [String; 2],     // roster names, left then right
      pub theme: String, pub bars: u32,             // tempo as the number of bars that fill 61 s
      pub handicap: [f64; 2], pub seed: Option<u64>,
      pub chord_loop: usize, pub bass: Option<usize>, pub music_seed: u64,
      pub captions: CaptionTexts, pub captions_edited: bool,
      pub base: Option<String> }                    // the fight file this draft was opened from, as TOML
  impl Draft { pub fn tempo_bpm(&self) -> f64; }    // loop_bpm(bars as f64 * 4.0 * 60.0 / 61.0, 61.0, 4)
  // model::roster
  pub enum Section { Woodwinds, Brass, Strings, Keys, Percussion }   // shown as "Keys and electronics" for Keys
  pub struct AbilityLine { pub tag: String, pub name: String, pub text: String }   // "LV2", "Flutter tongue", "Twice as fast for 8 beats"
  pub struct FighterCard { pub weapon: String, pub text: String, pub abilities: [AbilityLine; 3], pub range: String,
      pub usually_plays: String, pub colour_name: String, pub music_title: String, pub music_text: String,
      pub can_play_bass: bool, pub portrait: Option<Sprite> }
  pub struct RosterEntry { pub name: String, pub display: String, pub section: Section, pub color: [u8; 3],
      pub available: bool, pub music_feeds_fight: bool, pub mask: PixelMask, pub card: Option<FighterCard> }
  pub struct ThemeEntry { pub name: String, pub display: String, pub text: String, pub key: String, pub scale: String,
      pub chord_loops: Vec<Vec<String>>, pub bars: (u32, u32) }
  pub struct Roster { pub fighters: Vec<RosterEntry>, pub themes: Vec<ThemeEntry>, pub vs: PixelMask,
      pub start: [String; 2], pub app_version: String }
  // model::shortlist
  pub enum Finish { Any, Close, Comeback }
  pub struct FilterSpec { pub winner: Option<usize>, pub ko: (f64, f64), pub min_lead_changes: u32, pub finish: Finish,
      pub ability: Option<String>, pub show_outside: bool }   // Default: None, (49.0, 53.0), 2, Any, None, false
  pub enum SearchState { Idle, Searching, Stopped, Full }
  pub struct Shortlist { pub rows: Vec<StoryView>, pub fits_total: u32, pub checked: u64, pub state: SearchState,
      pub workers: usize }                                    // rows are already filtered and ranked
  // model::balance
  pub struct BalanceStats { pub fights: u32, pub win_share: f64,   // the left fighter's share of decided fights
      pub ko_median: f64, pub ko_p10: f64, pub ko_p90: f64, pub no_ko_share: f64, pub lead_changes_mean: f64,
      pub fit_rate: f64, pub ko_hist: Vec<u32>,                   // 28 one-second bins, 30 s to 58 s
      pub seconds: f64 }
  impl BalanceStats { pub fn in_range(&self) -> bool; }           // 0.40 <= win_share <= 0.60
  // model::take
  pub trait FrameSource: Send + Sync { fn frames(&self) -> u32; fn draw(&self, f: u32) -> Frame; }
  pub struct AudioTake { pub samples: std::sync::Arc<[[f32; 2]]> }   // 48 kHz, NA long
  pub struct CheckLine { pub title: String, pub pass: bool, pub note: String }
  pub enum LaneGroup { Fighters, Band }
  pub struct Lane { pub group: LaneGroup, pub name: String, pub color: Option<[u8; 3]>, pub clips: Vec<(f32, f32)> }
  pub struct MusicView { pub checks: Vec<CheckLine>, pub lanes: Vec<Lane>, pub markers: Vec<(f32, String)> }
  pub enum TransportAction { PlayPause, Step(i32), ScrubTo(f32 /* seconds */), ScrubEnd, SetLoop(bool), SetSafeZone(bool) }
  pub enum SoundState { None, Building, Ready, NoDevice }
  pub struct TransportView { pub enabled: bool, pub frame: u32, pub playing: bool, pub looping: bool,
      pub safe_zone: bool, pub sound: SoundState }
  pub enum PlayerCmd { Clear, Still(Frame), Load { take: TakeId, frames: Arc<dyn FrameSource>, start: u32, autoplay: bool },
      Sound { take: TakeId, audio: AudioTake }, Transport(TransportAction) }
  // model::queue
  pub struct JobSpec { pub title: String, pub colors: [[u8; 3]; 2], pub theme: String, pub seed: u64,
      pub fight_toml: String, pub out_root: std::path::PathBuf }
  pub enum StepKey { Soundtrack, SafeZone, Focus, Loudness, Band, Flashes, LoopPoint, Frames, Encode, Verify }
  pub enum StepState { Todo, Now, Done, Failed }
  pub struct StepLine { pub key: StepKey, pub state: StepState, pub note: Option<String> }
  pub enum FailHint { OpenInSeeds, OpenInMusic, CheckFolder, InstallFfmpeg, None }
  pub enum JobState { Waiting, Rendering { done: u32, total: u32 }, Done { folder: std::path::PathBuf, seconds: f64 },
      Failed { headline: String, detail: String, hint: FailHint }, Cancelled }
  pub struct Job { pub id: JobId, pub spec: JobSpec, pub state: JobState, pub steps: Vec<StepLine>, pub interrupted: bool }
  pub struct Queue { pub jobs: Vec<Job>, pub paused: bool, pub next_id: JobId }   // its methods are Task 11
  pub enum JobUpdate { Step { key: StepKey, state: StepState, note: Option<String> },
      Frames { done: u32, total: u32, seconds: f32 },   // seconds since the first frame was drawn
      Done { folder: std::path::PathBuf, seconds: f64 }, Failed { headline: String, detail: String, hint: FailHint },
      Cancelled }
  // model::library
  pub struct LibraryEntry { pub folder: std::path::PathBuf, pub title: String, pub colors: [[u8; 3]; 2],
      pub seed: Option<u64>, pub modified: u64 /* seconds since 1970 */, pub fight_path: Option<std::path::PathBuf>,
      pub problem: Option<String> }
  pub enum FolderState { Unknown, Ok { taken: bool }, Missing, NotWritable(String) }
  // seam
  pub enum Command {
      LoadRoster,
      DescribeFight { gen: Gen, draft: Draft },
      AutoBalance { gen: Gen, draft: Draft },  Measure { gen: Gen, draft: Draft },  StopBalance,
      StartSearch { gen: Gen, draft: Draft, filters: FilterSpec },  StopSearch,  SetFilters { gen: Gen, filters: FilterSpec },
      LoadTake { take: TakeId, draft: Draft },  PreviewBusy(bool),
      Enqueue { draft: Draft, out_root: PathBuf },  RunJob { id: JobId, fight_toml: String, out_root: PathBuf },
      CancelJob { id: JobId },
      ScanLibrary { root: PathBuf },  OpenFight { path: PathBuf },  OpenFightText { toml: String },
      SetWorkers(usize),  CheckTools,  CheckFolder { root: PathBuf, name: String },  CreateFolder { root: PathBuf },
      PickFolder { start: PathBuf },  RevealFolder { path: PathBuf },  Shutdown }
  pub enum Event {
      Roster(Roster),
      FightInfo { gen: Gen, pair_rules: Vec<String>, captions: CaptionTexts, title_card: Option<Frame>, problem: Option<String> },
      BalanceProgress { gen: Gen, done: u32, total: u32 },
      Balanced { gen: Gen, handicap: [f64; 2], stats: BalanceStats },  Measured { gen: Gen, stats: BalanceStats },
      Shortlist { gen: Gen, list: Shortlist },
      TakePicture { take: TakeId, story: StoryView, frames: Arc<dyn FrameSource> },
      TakeSound { take: TakeId, audio: AudioTake, music: MusicView },
      TakeFailed { take: TakeId, problem: String },
      Enqueued { spec: JobSpec },  EnqueueFailed { problem: String },
      Job { id: JobId, update: JobUpdate },
      Library { root: PathBuf, entries: Vec<LibraryEntry>, problem: Option<String> },
      LibraryCover { folder: PathBuf, cover: Frame },
      FightOpened { draft: Draft, title: String },
      Tools { ffmpeg: Result<String, String> },
      Folder { root: PathBuf, state: FolderState },  FolderPicked { path: Option<PathBuf> },
      Problem { text: String } }
  pub type Waker = Arc<dyn Fn() + Send + Sync>;
  pub struct EngineHandle { /* Sender<Command>, Receiver<Event> */ }
  impl EngineHandle { pub fn send(&self, c: Command); pub fn poll(&self) -> Option<Event>; }
  pub struct EngineSide { pub commands: std::sync::mpsc::Receiver<Command>, /* Sender<Event>, Waker */ }
  impl EngineSide { pub fn emit(&self, e: Event); }          // sends, then calls the waker; `Clone`-able sender inside
  pub fn channel(waker: Waker) -> (EngineHandle, EngineSide);
  // seam::sample (sample data: the mock-ups' own; used by tests, by the fixtures and by --fixtures)
  pub fn mask(text: &str) -> PixelMask;                      // 5 x 7 glyphs A to Z and space, advance 6
  pub fn roster() -> Roster;                                 // the 30 fighters of the interface reference, all available
  pub fn story(seed: u64) -> StoryView;                      // the mock-ups' generator, below
  pub fn stories() -> Vec<StoryView>;                        // the mock-ups' 14 seeds, best score first
  pub fn stats() -> BalanceStats;                            // 3,000 fights, 47 %, the histogram of the Balance mock-up
  pub fn frames(colors: [[u8; 3]; 2]) -> Arc<dyn FrameSource>;   // drawn shapes, a different picture for every frame
  pub fn title_card(colors: [[u8; 3]; 2], masks: [&PixelMask; 2]) -> Frame;
  pub fn audio() -> AudioTake;                               // a 10 ms 1 kHz blip on every beat at 133.77 bpm, peak 0.03
  pub fn music() -> MusicView;                               // the lanes, clips and markers of the Music mock-up
  pub fn captions() -> CaptionTexts;                         // the Render mock-up's caption and tags
  pub fn library() -> Vec<(LibraryEntry, Frame)>;            // six past videos with drawn 180 x 320 covers
  // wording
  pub fn display_name(roster_name: &str) -> String;          // "BASS CLARINET" -> "Bass clarinet"
  pub fn thousands(n: u64) -> String;                        // 18240 -> "18,240"
  pub fn percent(share: f64) -> String;                      // 0.47 -> "47%"
  pub fn times(x: f64) -> String;                            // 2.0 -> "×2.00"
  pub fn seconds(x: f64) -> String;                          // 50.6 -> "50.6 s"
  pub fn plural(n: u64, one: &str, many: &str) -> String;    // (1, "video", "videos") -> "1 video"
  pub fn folder_stem(names: &[String; 2]) -> String;         // ["BASS CLARINET", "BASSOON"] -> "bass_clarinet_vs_bassoon"
  pub fn versus(names: &[String; 2]) -> String;              // ["FLUTE", "TROMBONE"] -> "Flute vs Trombone"
  ```
- **The sample generator** (a port of the mock-ups' script, so the sample shortlist is the approved one):
  ```rust
  fn rng(seed: u32) -> impl FnMut() -> f64 {   // mulberry32
      let mut a = seed;
      move || { a = a.wrapping_add(0x6D2B_79F5);
          let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
          t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
          f64::from(t ^ (t >> 14)) / 4_294_967_296.0 } }
  // story(seed): r = rng(seed as u32); ko = 49 + r() * 4; winner = if r() < 0.47 { 0 } else { 1 };
  // left = (4 + r() * 25).round(); 30 values L: L[0] = 0, L[i] = L[i-1] + (r() - 0.5) * 22;
  // target = left for winner 0, -left for winner 1; d = target - L[29]; L[i] += d * (i / 29)^3;
  // score = 6.2 + r() * 1.7. Then lead[i] = (3.6 + (ko - 3.6) * i / 29, L[i]); t_fight = 3.6; winner_hp = left;
  // lead_changes counts sign flips of L outside the band -5..5; fits = true; the abilities' first uses are at
  // 11.96 s and 30.62 s (fighter 0: "Flutter tongue", "Snake dance") and 17.02 s, 25.99 s and 41.0 s (fighter 1:
  // "Sad trombone", "Power slide", "Pedal tone").
  ```
  The seeds are 20418, 7731, 15290, 3306, 11874, 26051, 902, 18433, 5627, 23980, 14402, 8175, 29766, 1248. Glyph
  rows for `mask` are copied from the `_G` table in `reelkit/font.py` (read only; A to Z). The sample roster's cards
  for FLUTE and TROMBONE carry the Match-up mock-up's texts (interface reference, 5.1) with `can_play_bass` false
  for FLUTE and true for TROMBONE; every other fighter's card says "Sample fighter" as its weapon and "This card is
  sample data." as its text. The sample theme list is one entry: `big_top`, "Big top", the mock-up's line, key E,
  "harmonic minor", two chord loops (`Em Am B7 Em C Am B7 B7` and `Em C Am B7 Em C Am B7`), bars (24, 42).
  `roster().start` is `["FLUTE", "TROMBONE"]`; PIANO has `music_feeds_fight: true`, everyone else false.
- [ ] **Step 1: Write the failing tests** (unit, in the files that own them). `the_sample_roster_is_the_series`: 30
  fighters; 9, 6, 8, 5 and 2 per section in the order of the interface reference; no entry is named "GUITAR" or
  "SAX"; FLUTE is `[0xde, 0xe8, 0xf6]`, TROMBONE `[0x78, 0xf0, 0xb8]`, TUBA `[0x68, 0x5c, 0xec]`, BASS DRUM `[0x80,
  0x80, 0x8c]`. `masks_use_the_series_font`: `mask("FLUTE")` is 29 x 7; `mask("F")` rows are `#####`, `#....`,
  `#....`, `####.`, `#....`, `#....`, `#....`; `mask("VS").w == 11`. `the_sample_stories_are_ranked`: 14 stories,
  scores descending, every `t_ko` between 49 and 53, every `lead` 30 points starting at 3.6 s.
  `tempo_snaps_to_whole_bars`: a draft with `bars: 34` has `(tempo_bpm() - 133.770_491_803_278_7).abs() < 1e-12`;
  with 24 bars `tempo_bpm() == 24.0 * 4.0 * 60.0 / 61.0`. `the_wording_helpers`: every example in the comments
  above. `events_reach_the_other_side`: `channel(waker)`; `side.emit(Event::Problem { .. })` makes
  `handle.poll()` return it and the waker has run once; `handle.send(Command::StopSearch)` arrives on
  `side.commands`. `filters_default_to_the_beat_sheet`.
- [ ] **Step 2:** Run `cargo test -p bf-app --lib model:: seam:: wording::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: model, the engine seam, sample data`.

### Task 9: App state: navigation, the draft, what feeds the fight, late results

**Files:**
- Create: `src/state/mod.rs`, `src/state/fight.rs`, `src/state/seeds.rs`, `src/state/events.rs`,
  `src/fixtures/base.rs`

**Interfaces:**
- Consumes: everything in `model` and `seam` (Task 8), `wording`.
- Produces:
  ```rust
  pub struct BalanceView { pub stats: Option<BalanceStats>, pub running: Option<(u32, u32)> }   // running: done, total
  pub struct TakeView { pub story: Option<StoryView>, pub music: Option<MusicView>, pub loading: bool, pub problem: Option<String> }
  pub struct LibraryView { pub entries: Vec<LibraryEntry>, pub covers: BTreeMap<PathBuf, Frame>, pub selected: Option<PathBuf>,
      pub scanning: bool, pub problem: Option<String> }
  pub struct ViewModel { pub view: View, pub step: Step, pub roster: Option<Roster>, pub draft: Draft,
      pub pair_rules: Vec<String>, pub fight_problem: Option<String>, pub balance: BalanceView, pub filters: FilterSpec,
      pub shortlist: Shortlist, pub take: TakeView, pub transport: TransportView, pub caption_tab: Platform,
      pub folder: FolderState, pub ffmpeg: Option<Result<String, String>>, pub queue: Queue, pub selected_job: Option<JobId>,
      pub library: LibraryView, pub settings: Settings, pub cores: usize, pub notices: Vec<Notice>, pub status: String }
  impl ViewModel { pub fn fighter(&self, side: usize) -> Option<&RosterEntry>; pub fn colors(&self) -> [[u8; 3]; 2];
      pub fn theme(&self) -> Option<&ThemeEntry>; pub fn names(&self) -> [String; 2]; }   // names as shown: "Flute"
  pub enum QueueAction { Pause, Resume, Cancel(JobId), Remove(JobId), Move { id: JobId, to: usize }, Requeue(JobId),
      Select(JobId), OpenFolder(JobId), OpenIn { id: JobId, step: Step } }
  pub enum Action { Go(View), PickFighter { side: usize, name: String }, SwapSides, SetTheme(String), SetBars(u32),
      SetHandicap { side: usize, value: f64, released: bool }, ResetHandicap, AutoBalance, StopBalance,
      SetFilters(FilterSpec), StopSearch, SearchOn, SelectSeed(u64), UseSeed, SetChordLoop(usize), SetBass(usize),
      SetCaptionTab(Platform), SetCaption { platform: Platform, text: String }, ChangeFolder, CreateFolder, AddToQueue,
      Queue(QueueAction), Transport(TransportAction), RefreshLibrary, SelectVideo(PathBuf), Reopen(PathBuf),
      OpenVideoFolder(PathBuf), SetWorkers(usize), Dismiss(usize) }
  pub enum Effect { Send(Command), Player(PlayerCmd), SaveSettings(Settings), SaveQueue(Queue) }
  pub struct AppState { pub vm: ViewModel, /* the generation, the wanted take, the title card */ }
  impl AppState {
      pub fn new(settings: Settings, queue: Queue, notices: Vec<Notice>, cores: usize) -> (AppState, Vec<Effect>);
      pub fn showing(vm: ViewModel) -> AppState;              // for fixtures and screen tests
      pub fn dispatch(&mut self, action: Action) -> Vec<Effect>;
      pub fn apply(&mut self, event: Event) -> Vec<Effect>;
      pub fn generation(&self) -> Gen; }
  // fixtures::base
  pub fn vm() -> ViewModel;   // sample roster, FLUTE vs TROMBONE, big_top, 34 bars, handicap [1.0, 1.0], no seed, Match-up
  ```
  `ViewModel` and its parts derive `Clone`, `Debug` and `PartialEq`. This task handles every action and event
  except the queue, folder, library and settings ones (Task 12), which it leaves as no-ops.
- **The rules.**
  - `new` shows Match-up and returns `Send(LoadRoster)`, `Send(CheckTools)` and `Send(CheckFolder { root:
    settings.output_folder, name: "" })`. On `Event::Roster` the draft becomes `roster.start`, the first theme, 34
    bars clamped to the theme's range, handicap `[1.0, 1.0]`, no seed, chord loop 0, bass `None`, music seed 1; the
    generation becomes 1; `Send(DescribeFight)` follows.
  - **What feeds the fight** (§4): the two fighters in order, `bars` and `handicap`; and also `chord_loop` and
    `music_seed` when either chosen fighter has `music_feeds_fight`. An action that changes any of these adds 1 to
    the generation, clears the seed, the shortlist (to `Idle`), the balance numbers and the take, and returns
    `Send(StopSearch)` if a search was running, `Send(StopBalance)` if a balance was, and `Player(Still(card))` with
    the last title card (or `Player(Clear)`); when the fighters changed it also returns `Send(DescribeFight)`.
    Changing the theme, the bass player or (for other fighters) the chord loop never changes the generation; if a
    seed is chosen they ask for a new take.
  - `PickFighter` with the fighter already in the other corner is `SwapSides` (fighters and handicap swap). Any other
    new fighter resets the handicap to `[1.0, 1.0]`, forgets `draft.base` and un-sets `captions_edited`.
  - `SetHandicap` clamps to 0.05 to 12.0 and rounds to two decimals; when `released` it returns `Send(Measure)`.
    `ResetHandicap` sets both to 1.0 and measures. `Go(Step(Balance))` with no numbers and nothing running starts
    `AutoBalance` when the handicap is `[1.0, 1.0]`, else `Measure`. `Balanced` for the current generation sets the
    handicap it found (a new generation if it differs) and keeps its numbers for that new generation.
  - While `fight_problem` is set, no balance, measurement, search or take is asked for.
  - `Go(Step(Seeds))` with the shortlist `Idle` returns `Send(StartSearch)` and marks it `Searching`. `SetFilters`
    stores them (K.O. bounds kept inside 30 to 58 and in order) and returns `Send(SetFilters)`.
  - `SelectSeed(n)` sets the seed, takes the next `TakeId`, marks the take loading and returns `Send(LoadTake)`.
    `TakePicture` for the wanted take stores the story and returns `Player(Load { start: (story.t_fight *
    30.0).round(), autoplay: view is Seeds })`; `TakeSound` stores the music view and returns `Player(Sound)`;
    `TakeFailed` stores the text and returns the title card. `UseSeed` goes to Music, only when the story has a
    winner.
  - `FightInfo` for the current generation sets the pair rules, the problem, the captions (unless
    `captions_edited`) and the title card, which is shown when no take is. `SetCaption` stores the text, sets
    `captions_edited`, and keeps the first line of the Instagram and TikTok texts the same (the fight file has one
    caption): editing either one's first line rewrites the other's.
  - `Transport(a)` returns `Player(PlayerCmd::Transport(a))` and nothing else; the shell copies the player's view
    back into `vm.transport` (Task 17).
  - **Late results** (Review Focus 3): `FightInfo`, `BalanceProgress`, `Balanced`, `Measured` and `Shortlist` whose
    `gen` is not the current generation, and `TakePicture`, `TakeSound` and `TakeFailed` whose `take` is not the
    wanted one, change nothing and return no effects.
  - `vm.status` after each change: "{Left} vs {Right}, not balanced yet"; "Balanced on {n} fights in {s} s"
    (`thousands`, one decimal); "Searching on {workers} cores"; "Seed {n} is ready"; and the problem text when
    there is one.
- [ ] **Step 1: Write the failing tests** (unit, in `state/mod.rs`, driving `dispatch` and `apply` only; a helper
  `sent(&effects)` lists the commands): `starts_by_asking_for_the_roster`; `the_roster_sets_the_starting_pair`
  (fighters `["FLUTE", "TROMBONE"]`, `bars == 34`, generation 1, one `DescribeFight`);
  `picking_the_other_corners_fighter_swaps` (after `SetHandicap` to 2.0 and 0.4: `PickFighter { side: 0, name:
  "TROMBONE" }` gives fighters `["TROMBONE", "FLUTE"]` and handicap `[0.4, 2.0]`);
  `every_fight_input_starts_a_new_generation` (for `PickFighter` to OBOE, `SwapSides`, `SetBars(30)`, `SetHandicap`:
  the generation rises by exactly 1, `draft.seed` is `None`, the shortlist is empty and `Idle`, `balance.stats` is
  `None`); `music_settings_keep_the_generation` (`SetTheme`, `SetBass(1)`, `SetChordLoop(1)`);
  `notes_that_are_a_weapon_feed_the_fight` (with PIANO in a corner `SetChordLoop(1)` raises the generation);
  `late_results_are_dropped` (apply each of the eight events above with the old generation or an old take after a
  `SwapSides` or a second `SelectSeed`: `vm` is equal to a clone taken before, and no effect comes back);
  `releasing_the_handicap_measures` (`released: false` sends nothing, `true` sends one `Measure`; 2.004 becomes
  2.0, 99.0 becomes 12.0); `opening_balance_balances_once`; `a_balanced_handicap_keeps_its_numbers`;
  `opening_seeds_starts_one_search` (a second `Go` sends none); `choosing_a_seed_asks_for_its_take` then
  `the_picture_starts_at_fight` (`t_fight` 3.6 gives `start == 108`, `autoplay` true on Seeds and false on Music);
  `a_seed_with_no_winner_cannot_be_used`; `drafted_captions_arrive_until_edited`;
  `the_caption_line_is_shared` (setting the TikTok text to "New line\n#fyp" makes the Instagram text start with
  "New line\n" and keep its own tags).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib state::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: app state, what feeds the fight, late results`.

### Task 10: Storage folder, the background writer and settings

**Files:**
- Create: `src/store/mod.rs`, `src/store/settings.rs`

**Interfaces:**
- Consumes: `model::{Settings, Queue, Notice}`.
- Produces:
  ```rust
  // store
  pub struct Paths { pub config: PathBuf, pub cache: PathBuf }
  impl Paths { pub fn system() -> Option<Paths>;      // directories::ProjectDirs::from("", "", "BALLFIGHT Studio")
      pub fn under(root: &Path) -> Paths;             // <root>/config and <root>/cache: tests and --data-dir
      pub fn settings(&self) -> PathBuf;              // config/settings.toml
      pub fn queue(&self) -> PathBuf;                 // config/queue.json
      pub fn shortlists(&self) -> PathBuf; }          // cache/shortlists
  pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()>;   // creates the folder, writes <name>.tmp, renames over
  pub fn move_aside(path: &Path, tag: &str) -> std::io::Result<PathBuf>;   // settings.toml, "corrupt" -> settings.corrupt-1.toml (first free number)
  pub struct Writer { /* one thread; of each kind only the newest value is written */ }
  impl Writer { pub fn start(paths: Paths) -> Writer; pub fn settings(&self, s: &Settings); pub fn queue(&self, q: &Queue);
      pub fn flush(&self); }                          // waits until everything asked for is on disk
  // store::settings
  pub const SETTINGS_FORMAT: u32 = 1;
  pub fn default_settings(cores: usize) -> Settings;  // <Videos>/BALLFIGHT (else <home>/Videos/BALLFIGHT); workers = max(1, cores - 1)
  pub fn load_settings(path: &Path, cores: usize) -> (Settings, Option<Notice>);
  pub fn save_settings(path: &Path, s: &Settings) -> std::io::Result<()>;   // TOML: format, output_folder, workers
  ```
  Until Task 11 exists `Writer::queue` may write the queue with `serde_json` directly; Task 11 swaps in
  `save_queue`.
- [ ] **Step 1: Write the failing tests** (unit; each in its own temp folder under `std::env::temp_dir()`, removed
  at the end): `defaults_leave_one_core_free` (`default_settings(8).workers == 7`, `default_settings(1).workers ==
  1`, the folder ends in `BALLFIGHT`); `settings_round_trip`; `a_missing_file_gives_defaults_and_no_notice`;
  Review Focus 2: `a_corrupt_file_is_moved_aside` (the file holds `output_folder = [[[`: defaults come back, the
  notice is "Your settings file could not be read, so the app is using its defaults. The old file was kept as
  settings.corrupt-1.toml.", that file exists with the old bytes and `settings.toml` is gone; a second corrupt file
  becomes `settings.corrupt-2.toml`); `a_half_written_file_is_corrupt` (the first 10 bytes of a good file);
  `a_newer_file_is_moved_aside` (`format = 2`: notice "Your settings were saved by a newer version of BALLFIGHT
  Studio, so the app is using its defaults. The file was kept as settings.newer-1.toml.");
  `workers_stay_between_1_and_the_cores` (0 loads as 1, 999 as `cores`); `unknown_keys_are_ignored`;
  `write_atomic_leaves_no_tmp_file` and creates a missing folder; `the_writer_writes_the_newest` (ask for three
  settings in a row, `flush()`: the file holds the third).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib store::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: storage folder, background writer, settings`.

### Task 11: The queue and its file

**Files:**
- Create: `src/store/queue_file.rs`
- Modify: `src/model/queue.rs` (the methods), `src/store/mod.rs` (`Writer::queue` uses `save_queue`)

**Interfaces:**
- Consumes: `Queue { jobs, paused, next_id }`, `Job`, `JobSpec`, `JobState`, `JobUpdate`, `StepKey`, `StepLine`,
  `write_atomic`, `move_aside`.
- Produces:
  ```rust
  impl Queue {
      pub fn new() -> Queue;                                    // empty, not paused, next_id 1
      pub fn add(&mut self, spec: JobSpec) -> JobId;            // waiting, last in line
      pub fn job(&self, id: JobId) -> Option<&Job>;
      pub fn remove(&mut self, id: JobId) -> bool;              // refused for the job being rendered
      pub fn move_to(&mut self, id: JobId, to: usize) -> bool;  // a waiting job to place `to` among the waiting jobs
      pub fn requeue(&mut self, id: JobId) -> bool;             // failed or cancelled -> waiting, last in line, steps cleared
      pub fn rendering(&self) -> Option<JobId>;
      pub fn next_to_run(&self) -> Option<JobId>;               // the first waiting job; None while paused or rendering
      pub fn start(&mut self, id: JobId);                       // Rendering { done: 0, total: 1830 }, the ten steps Todo
      pub fn apply(&mut self, id: JobId, update: JobUpdate);
      pub fn counts(&self) -> (usize, usize);                   // rendering, waiting
      pub fn shown(&self) -> Vec<JobId>;                        // rendering, then waiting in order, then finished, newest first
      pub fn after_restart(&mut self) -> usize;                 // rendering -> waiting and `interrupted`; pauses if anything waits; how many were interrupted
      pub fn trim(&mut self); }                                 // keeps the 20 newest finished jobs
  pub const STEP_ORDER: [StepKey; 10];   // Soundtrack, SafeZone, Focus, Loudness, Band, Flashes, LoopPoint, Frames, Encode, Verify
  pub const QUEUE_FORMAT: u32 = 1;
  pub fn load_queue(path: &Path) -> (Queue, Option<Notice>);
  pub fn save_queue(path: &Path, q: &Queue) -> std::io::Result<()>;   // JSON: format, app_version, paused, next_id, jobs
  ```
  One job renders at a time (§9). `apply`: `Step` sets that line; `Frames` sets `Rendering { done, total }`; `Done`,
  `Failed` and `Cancelled` set the state (a failed step keeps its `Failed` line). A job saved while rendering is
  written as waiting with `interrupted: true`. `save_queue` trims first.
- [ ] **Step 1: Write the failing tests** (unit). Model: `ids_count_up`; `one_job_at_a_time` (`next_to_run` is `None`
  while one renders and while paused); `the_running_job_cannot_be_removed`; `moving_reorders_waiting_jobs_only` (three
  waiting A, B, C: `move_to(C, 0)` gives C, A, B; moving a finished job returns false);
  `requeue_goes_last_with_clean_steps`; `updates_change_the_job` (each `JobUpdate`); Review Focus 4:
  `a_failed_job_lets_the_next_one_run` (after `Failed`, `next_to_run` is the next waiting job) and
  `after_a_restart_the_interrupted_job_waits` (a rendering job becomes waiting with `interrupted`, the queue is paused,
  the count returned is 1; with nothing waiting it is not paused); `shown_order`; `trim_keeps_20_finished`. File:
  `queue_round_trip`; `a_rendering_job_is_saved_as_waiting`; Review Focus 2: `a_newer_queue_is_left_alone` (`"format":
  2`: an empty queue comes back with the notice "The render queue was saved by a newer version of BALLFIGHT Studio, so
  it was left alone as queue.newer-1.json. Videos you add now go in a fresh queue.", and that file holds the original
  bytes) and `a_corrupt_queue_is_moved_aside` (notice "The render queue could not be read, so it starts empty. The old
  file was kept as queue.corrupt-1.json."); `a_missing_queue_is_empty_and_silent`.
- [ ] **Step 2:** Run `cargo test -p bf-app --lib model::queue store::queue_file`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: the render queue and its file`.

### Task 12: Queue, output folder, library and settings in the app state

**Files:**
- Create: `src/state/queue_run.rs`
- Modify: `src/state/mod.rs` (route the actions and events below to it)

**Interfaces:**
- Consumes: `AppState`, `Action`, `QueueAction`, `Effect` (Task 9); `Queue` methods (Task 11); `Command`, `Event`,
  `FolderState`, `wording::{folder_stem, versus, plural}`.
- Produces: the behaviour of `AddToQueue`, every `QueueAction`, `ChangeFolder`, `CreateFolder`, `RefreshLibrary`,
  `SelectVideo`, `Reopen`, `OpenVideoFolder`, `SetWorkers`, `Dismiss`, `Go(Library)`, and of the events `Enqueued`,
  `EnqueueFailed`, `Job`, `Library`, `LibraryCover`, `FightOpened`, `Tools`, `Folder`, `FolderPicked`, `Problem`.
- **The rules.**
  - `AddToQueue` returns `Send(Enqueue { draft, out_root })` only when a seed is chosen, its take has a winner, the
    folder state is `Ok` and ffmpeg was found; otherwise it changes nothing (Render shows why, Task 23).
  - `Enqueued` adds the job and returns `SaveQueue`; then, like every event that ends a job, it starts the next one:
    if `queue.next_to_run()` gives a job, `queue.start(id)` and `Send(RunJob)`. `EnqueueFailed` adds its text as a
    notice.
  - `Job { id, update }` goes to `queue.apply`. `Done`, `Failed` and `Cancelled` return `SaveQueue`, start the next
    job (Review Focus 4: the queue continues) and, for `Done`, `Send(ScanLibrary)`. `Frames { done, total, seconds }`
    with `done > 0` sets the status "Rendering {title}, about {n} s left", n being `seconds * (total - done) / done`
    rounded; before that the status is "Rendering {title}".
  - `Pause` and `Resume` set `queue.paused` (the job being rendered finishes; `Resume` starts the next); `Cancel`
    returns `Send(CancelJob)` for the rendering job only; `Remove`, `Move` and `Requeue` call the queue and save;
    `Select` sets `selected_job`; `OpenFolder` returns `Send(RevealFolder)`; `OpenIn { id, step }` returns
    `Send(OpenFightText { toml })` and remembers the step, which `FightOpened` then shows.
  - `FightOpened { draft, title }` replaces the draft (a new generation, as a fight-input change), shows Match-up
    (or the remembered step), sets the status "Opened {title}" and returns `Send(DescribeFight)`, plus
    `Send(LoadTake)` when the draft has a seed.
  - At start (`AppState::new`), `queue.after_restart()` runs; if it interrupted n > 0 jobs a notice is added: "1
    video was interrupted when the app closed. It is back in the queue. Resume the queue to render it." (`plural`
    for more); if jobs are only waiting: "{n} videos are waiting in the queue. Resume the queue to render them."
  - Folder (Review Focus 1): `Folder { root, state }` for the current output folder sets `vm.folder`;
    `ChangeFolder` returns `Send(PickFolder)`; `FolderPicked(Some(p))` sets `settings.output_folder`, returns
    `SaveSettings`, `Send(CheckFolder)` and `Send(ScanLibrary)`; `CreateFolder` returns `Send(CreateFolder)`; the
    draft's fighters or seed changing returns `Send(CheckFolder { name: folder_stem(..) })` again.
  - `Go(Library)` and `RefreshLibrary` return `Send(ScanLibrary)` and set `scanning`; `Library` for the current
    folder fills the entries and the problem; `LibraryCover` adds a cover; `Reopen(folder)` returns
    `Send(OpenFight { path })` for that entry's fight file.
  - `SetWorkers(n)` (kept in 1 to `cores`) returns `SaveSettings` and `Send(SetWorkers)`. `Tools` sets `vm.ffmpeg`.
    `Problem { text }` adds a notice. `Dismiss(i)` removes notice i.
- [ ] **Step 1: Write the failing tests** (unit, in `queue_run.rs`): `adding_needs_a_seed_a_folder_and_ffmpeg` (each
  missing one alone sends nothing); `an_enqueued_job_starts_when_idle` (effects: `SaveQueue`, `RunJob`);
  `a_second_job_waits`; `the_queue_goes_on_after_done_failed_and_cancelled` (three tests: the next `RunJob` is sent);
  `the_status_counts_down` (`Frames { done: 610, total: 1830, seconds: 10.0 }` for a job titled "Tuba vs Bass drum"
  gives "Rendering Tuba vs Bass drum, about 20 s left"); `pausing_lets_the_current_job_finish_and_holds_the_next`;
  `resuming_starts_it`; `cancel_reaches_only_the_rendering_job`; `a_restart_explains_itself` (the two notices above,
  word for word, and `queue.paused`); `a_missing_folder_blocks_adding_and_can_be_created` (`Folder { state: Missing }`
  then `AddToQueue` sends nothing; `CreateFolder` sends the command); `a_picked_folder_is_saved_checked_and_scanned`;
  `reopening_loads_the_fight` (`FightOpened` raises the generation, shows Match-up, status "Opened Flute vs Trombone,
  seed 20418", sends `DescribeFight` and `LoadTake`); `open_in_music_lands_on_music`; `workers_are_saved_and_sent`;
  `a_problem_becomes_a_notice_and_can_be_dismissed`.
- [ ] **Step 2:** Run `cargo test -p bf-app --lib state::queue_run`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `cargo test -p bf-app --lib state::`: PASS.
- [ ] **Step 5:** Commit `bf-app: queue, folder, library and settings in the app state`.

### Task 13: Shortlist cache, library scan and cover thumbnails

**Files:**
- Create: `src/store/shortlist_cache.rs`, `src/store/library.rs`

**Interfaces:**
- Consumes: `write_atomic`, `Frame`, `LibraryEntry`, `FolderState`; the `sha2`, `png` and `chrono` crates.
- Produces:
  ```rust
  // store::shortlist_cache: only seed numbers are kept; the engine re-runs them (fights are reproducible)
  pub const CACHE_FORMAT: u32 = 1;  pub const CACHE_FILES: usize = 200;
  pub struct CachedShortlist { pub next_seed: u64, pub checked: u64, pub fit_seeds: Vec<u64>, pub other_seeds: Vec<u64> }
  pub fn cache_file(dir: &Path, app_version: &str, search_key: &str) -> PathBuf;   // <dir>/<first 16 hex of sha256(app_version + "\n" + search_key)>.json
  pub fn load_shortlist(dir: &Path, app_version: &str, search_key: &str) -> Option<CachedShortlist>;
  pub fn save_shortlist(dir: &Path, app_version: &str, search_key: &str, c: &CachedShortlist) -> std::io::Result<()>;
  // store::library
  pub struct FightSummary { pub title: String, pub colors: [[u8; 3]; 2], pub seed: Option<u64> }
  pub fn scan(root: &Path, read: &dyn Fn(&str) -> Result<FightSummary, String>) -> Result<Vec<LibraryEntry>, String>;
  pub fn thumbnail(cover_png: &[u8]) -> Result<Frame, String>;   // 1080 x 1920 -> 180 x 320: each pixel the mean of a 6 x 6 block
  pub fn folder_state(root: &Path, name: &str) -> FolderState;
  pub fn local_date(secs: u64) -> String;                        // "Nov 10, 2026", in local time
  ```
  The cache file stores `format`, `app_version`, `search_key` and the four fields; a file whose three headers do not
  all match, or that does not parse, is a miss (`None`), never an error. `save_shortlist` then deletes the oldest
  files beyond `CACHE_FILES`. `scan`: one entry for each direct subfolder of `root` whose name does not start with
  "." and that holds `cover.png`; its fight file is the one `.toml` in it, given to `read` as text; no `.toml` gives
  `problem: Some("The fight file is missing, so this video cannot be reopened.")`; an `Err` from `read` becomes the
  problem; the title falls back to the folder's name; `modified` is the cover's; newest first. A missing root is
  `Err("The output folder {path} is missing.")`. `folder_state`: `Missing` when `root` is not a folder;
  `NotWritable(reason)` when a probe file cannot be created and deleted in it; else `Ok { taken }` where `taken`
  says `<root>/<name>` exists (false for an empty name).
- [ ] **Step 1: Write the failing tests** (unit, temp folders). Cache: `cache_round_trip`; `another_version_is_a_miss`;
  `another_key_is_a_miss`; Review Focus 2: `a_corrupt_cache_is_a_miss`; `the_file_name_is_16_hex_digits`;
  `old_files_are_pruned` (with `CACHE_FILES + 3` files, three are deleted). Library: `scan_finds_videos_newest_first`
  (two folders with `cover.png` and a `.toml`, one hidden `.tmp-x` folder, one folder without a cover: two entries);
  `a_folder_without_a_fight_file_cannot_be_reopened`; `an_unreadable_fight_file_is_named` (`read` returns `Err("this app
  reads fight files up to format 1")`: the entry's problem is that text); Review Focus 1:
  `a_missing_output_folder_is_named` and `folder_states` (missing; `Ok { taken: false }`; `Ok { taken: true }` after
  creating `<root>/flute_vs_trombone`); `thumbnail_is_180x320` (a 1080 x 1920 PNG made in the test with the `png` crate,
  left half `[200, 0, 0]`, right half `[0, 0, 200]`: pixel (0, 0) is `[200, 0, 0]`, pixel (179, 319) is `[0, 0, 200]`);
  `a_wrong_sized_cover_is_an_error` ("The cover is 100 x 100, not 1080 x 1920."); `dates_read_naturally` (`local_date`
  of a `chrono::Local` noon on 2026-11-10 is "Nov 10, 2026").
- [ ] **Step 2:** Run `cargo test -p bf-app --lib store::shortlist_cache store::library`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: shortlist cache, library scan, cover thumbnails`.

### Task 14: Playback maths, preview size and the safe zone

**Files:**
- Create: `src/preview/clock.rs`, `src/preview/scale.rs`

**Interfaces:**
- Consumes: `model::{NF, NA, SAMPLES_PER_FRAME}`.
- Produces:
  ```rust
  // preview::clock: positions are samples of the 48 kHz soundtrack, which is the clock (§7)
  pub fn frame_of(pos: u64) -> u32;                       // min(pos / 1600, 1829)
  pub fn pos_of(frame: u32) -> u64;                       // frame * 1600
  pub fn advance(pos: u64, by: u64, looping: bool) -> (u64, bool);   // new position, still playing; wraps at NA when looping, else stops on NA - 1
  pub fn step(frame: u32, by: i32) -> u32;                // kept in 0..=1829
  pub fn timecode(frame: u32) -> String;                  // "0:31.40": whole hundredths of frame * 100 / 30
  pub const TOTAL: &str = "1:01.00";
  // preview::scale
  pub const SAFE: [i32; 4] = [20, 96, 340, 500];          // x0, y0, x1, y1 (ends excluded)
  pub const SAFE_BUTTONS: [i32; 4] = [300, 290, 360, 640];
  pub fn in_safe_zone(x: i32, y: i32) -> bool;
  pub fn shade() -> [[i32; 4]; 5];                        // the rectangles outside the safe zone, not overlapping
  pub fn preview_size(avail: (f32, f32)) -> (f32, f32);   // physical pixels: 360n x 640n for the largest whole n that fits, else the largest fit
  ```
- [ ] **Step 1: Write the failing tests** (unit): `frame_of(0) == 0`, `frame_of(1_599) == 0`, `frame_of(1_600) ==
  1`, `frame_of(2_928_639) == 1_829`; `pos_of(942) == 1_507_200`; `advance(2_928_000, 1_000, true) == (360,
  true)`; `advance(2_928_000, 1_000, false) == (2_928_639, false)`; `step(0, -1) == 0`, `step(1_829, 1) == 1_829`,
  `step(10, -3) == 7`; `timecode(0) == "0:00.00"`, `timecode(942) == "0:31.40"`, `timecode(1) == "0:00.03"`,
  `timecode(1_829) == "1:00.96"`. `the_shade_is_exactly_the_outside`: for every pixel of 360 x 640, the number of
  `shade()` rectangles holding it is 0 when `in_safe_zone` and 1 when not; `in_safe_zone(20, 96)`,
  `!in_safe_zone(340, 96)`, `in_safe_zone(299, 400)`, `!in_safe_zone(300, 290)`, `in_safe_zone(339, 289)`.
  `preview_size((410.0, 652.0)) == (360.0, 640.0)`; `((820.0, 1300.0)) == (720.0, 1280.0)`; `((410.0, 552.0)) ==
  (310.0, 552.0)`; `((300.0, 652.0)) == (300.0, 533.0)` (a fit keeps 9:16 and rounds down to whole pixels).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib preview::clock preview::scale`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: playback maths, preview size, safe zone`.

### Task 15: The draw-ahead scheduler

**Files:**
- Create: `src/preview/scheduler.rs`

**Interfaces:**
- Consumes: `model::take::{Frame, FrameSource}` (`frames() -> u32`, `draw(f: u32) -> Frame`), `TakeId`, `Waker`, `NF`.
- Produces:
  ```rust
  pub const AHEAD: u32 = 12;   pub const KEEP: usize = 64;
  pub fn pace(frame_ms: f32, threads: usize) -> (u32, u32);   // lead = ceil(frame_ms / 33.33) kept in 1..=12; stride = max(1, ceil(1.25 * frame_ms / (33.33 * threads)))
  pub fn wanted(playhead: u32, playing: bool, looping: bool, lead: u32, stride: u32) -> Vec<u32>;
  pub fn evict(cached: &[u32], playhead: u32, keep: usize) -> Vec<u32>;
  pub struct Scheduler { /* worker threads, a queue, the frame cache */ }
  impl Scheduler {
      pub fn new(threads: usize, waker: Waker) -> Scheduler;
      pub fn set_source(&self, take: TakeId, frames: Option<Arc<dyn FrameSource>>);   // forgets every frame of another take
      pub fn want(&self, playhead: u32, playing: bool, looping: bool);
      pub fn get(&self, f: u32) -> Option<Frame>;
      pub fn latest_at_or_before(&self, f: u32) -> Option<(u32, Frame)>;
      pub fn frame_ms(&self) -> f32;      // mean of the last 30 draw times; 33.0 before the first
      pub fn idle(&self) -> bool; }
  ```
  `wanted` when paused is `[playhead]` (a scrub: drawn before anything else). When playing it is `playhead + lead`,
  then every `stride`-th frame after it up to `playhead + AHEAD`, wrapping past 1,829 to 0 when looping and stopping
  at 1,829 when not. Workers take the paused playhead first, else the earliest wanted frame that is neither kept nor
  being drawn; a new `want` replaces the queue, so older scrub requests are skipped. Each finished frame is kept,
  the waker is called, and `evict` names what to drop beyond `KEEP`: the frames farthest behind the playhead first,
  then the farthest ahead. A frame drawn for a take that is no longer the source is thrown away. Threads end when
  the scheduler is dropped.
- [ ] **Step 1: Write the failing tests** (unit). Pure: `pace(20.0, 7) == (1, 1)`, `pace(100.0, 7) == (3, 1)`,
  `pace(300.0, 7) == (9, 2)`, `pace(26.0, 1) == (1, 1)`, `pace(30.0, 1) == (1, 2)`; `wanted(100, false, true, 1,
  1) == [100]`; `wanted(100, true, true, 1, 1) == (101..=112)`; `wanted(100, true, true, 3, 2) == [103, 105, 107,
  109, 111]`; `wanted(1_825, true, true, 1, 1) == [1826, 1827, 1828, 1829, 0, 1, 2, 3, 4, 5, 6, 7]`;
  `wanted(1_825, true, false, 1, 1) == [1826, 1827, 1828, 1829]`; `evict(&[1, 2, 3, 50, 51], 50, 3) == [1, 2]`.
  Threaded, with a test source whose `draw(f)` returns a frame whose every byte is `(f % 251) as u8` and counts its
  calls, and a helper that waits (at most 5 s) for `idle()`: `it_draws_ahead_while_playing` (after `want(100, true,
  true)`: `get(f)` is some for 101 to 112 and none for 200); `a_scrub_draws_just_that_frame`;
  `scrubs_skip_to_the_newest` (the source sleeps 5 ms; 50 `want(f, false, true)` calls for f = 0 to 49 in a row,
  with 2 threads: `get(49)` is some and the source drew fewer than 20 frames); `frames_are_the_sources_own_bytes`;
  `another_take_forgets_the_old_frames`; `it_keeps_at_most_64`; `dropping_it_stops_the_threads` (returns within 2
  s).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib preview::scheduler`. Expected: FAIL.
- [ ] **Step 3:** Implement with `std::thread`, one `Mutex` around the state and a `Condvar`; no drawing while
  holding the lock.
- [ ] **Step 4:** Run: PASS, ten times in a row.
- [ ] **Step 5:** Commit `bf-app: draw-ahead scheduler`.

### Task 16: Sound output and the player

**Files:**
- Create: `src/preview/audio.rs`, `src/preview/player.rs`

**Interfaces:**
- Consumes: `Scheduler` (Task 15), `clock` (Task 14), `AudioTake`, `Frame`, `FrameSource`, `PlayerCmd`,
  `TransportAction`, `TransportView`, `SoundState`, `Waker`.
- Produces:
  ```rust
  // preview::audio
  pub fn fill(out: &mut [f32], channels: usize, src: &[[f32; 2]], pos: &mut f64, step: f64, looping: bool) -> bool;
      // writes interleaved frames from `src` starting at `pos`, moving `step` source samples per output frame with
      // straight-line interpolation; one channel gets the mean of left and right, channels past the second get 0;
      // wraps to 0 at the end when looping, else writes silence and returns true
  pub trait AudioOut: Send {
      fn start(&mut self, audio: AudioTake, at: u64, looping: bool) -> Result<(), String>;
      fn stop(&mut self);
      fn position(&self) -> Option<u64>;      // the source sample now sounding; None when stopped
      fn set_looping(&mut self, on: bool); }
  pub struct NoAudio(pub String);             // every start fails with its text
  pub struct CpalOut { /* the default output device, a stream, an atomic position */ }
  pub fn open_audio() -> Box<dyn AudioOut>;   // CpalOut, else NoAudio("No sound device was found, so the preview is silent.")
  // preview::player
  pub type Now = Box<dyn Fn() -> std::time::Duration + Send>;
  pub struct PlayerStats { pub shown: u64, pub late: u64 }
  pub struct Player { /* scheduler, clock state, the audio out, the take */ }
  impl Player {
      pub fn new(threads: usize, audio: Box<dyn AudioOut>, now: Now, waker: Waker) -> Player;
      pub fn run(&mut self, cmd: PlayerCmd);
      pub fn tick(&mut self) -> Option<Frame>;    // once per repaint: moves the clock, asks the scheduler, gives the picture when it changed
      pub fn view(&self) -> TransportView;
      pub fn stats(&self) -> PlayerStats;
      pub fn wants_repaint(&self) -> bool; }      // playing, or waiting for a frame
  ```
- **The rules.** `Still(frame)`: `tick` gives it once; the transport is not enabled. `Load`: the scheduler gets the
  source, the position is `pos_of(start)`, the transport is enabled, sound is `Building`, and with `autoplay` it
  plays at once on the wall clock (`now`). `Sound` for the loaded take: sound is `Ready`, and if playing the audio
  starts from the current position; if `start` fails, sound is `NoDevice` and playback stays on the wall clock
  (Review Focus 5). While the audio sounds, the position is `audio.position()`: **the sound is the clock** (§7);
  otherwise it advances by wall time x 48,000. `PlayPause` toggles (from the very end with loop off it starts over).
  `Step(n)` pauses and moves n frames. `ScrubTo(t)` stops the audio, sets the position to `t * 48_000` kept below
  `NA`, and remembers whether it was playing; `ScrubEnd` resumes if it was. `SetLoop` (on at first) and
  `SetSafeZone` set their flags. `tick` asks the scheduler for the current frame: kept and different from the one on
  screen gives it (`shown += 1`); not kept yet counts `late` once per frame and gives `latest_at_or_before` if that
  is newer than what is on screen. `CpalOut` asks the device for its default output configuration, plays `f32`
  frames through `fill` with `step = 48_000 / device rate`, and counts the position in the callback.
- [ ] **Step 1: Write the failing tests** (unit). `fill`: `step_one_copies` (stereo out equals the source);
  `one_channel_gets_the_mean`; `six_channels_get_two_and_silence`; `half_step_interpolates` (source `[[0.0, 0.0], [1.0,
  1.0]]`, step 0.5: left samples 0.0, 0.5, 1.0); `looping_wraps_exactly` (a 4-sample source, 6 frames out: samples 0, 1,
  2, 3, 0, 1 and `false`); `the_end_is_silence` (not looping: 0, 1, 2, 3, then zeros, and `true`). Player, with a `Now`
  the test moves by hand, the sample source of Task 15's tests and a `FakeOut` (records `start` and `stop` calls;
  `position()` is what the test sets): `a_still_is_shown_once_and_disables_the_transport`;
  `it_plays_silently_before_the_sound_arrives` (after `Load { start: 108, autoplay: true }` and 1 s of `now`,
  `view().frame == 138` and `FakeOut` saw no `start`); `the_sound_joins_at_the_playhead` (then `Sound`: `start` was
  called with `at` between `pos_of(138)` and `pos_of(139)`; after setting `FakeOut`'s position to `pos_of(300)`,
  `view().frame == 300`); `no_sound_device_still_plays` (with `NoAudio`: after `Sound`, `view().sound ==
  SoundState::NoDevice` and the frame still advances with `now`); `step_pauses`;
  `scrub_stops_the_sound_and_resumes_after`; `the_loop_wraps` and `without_loop_it_stops_on_the_last_frame`;
  `a_late_frame_counts_once`. And one test marked `#[ignore]` (it needs a sound device; run it by name on this PC):
  `this_pc_plays_silence` (`open_audio()` starts one second of zeros and its position passes 24,000 within 2 s).
- [ ] **Step 2:** Run `cargo test -p bf-app --lib preview::audio preview::player`. Expected: FAIL.
- [ ] **Step 3:** Implement. `cpal`: `cpal::default_host().default_output_device()`, `default_output_config()`,
  `build_output_stream(config.into(), move |out: &mut [f32], _| ..., |err| ..., None)`, `stream.play()`. The stream
  is not `Send`: `CpalOut` keeps it on a thread of its own and talks to it through a channel.
- [ ] **Step 4:** Run: PASS; then `cargo test -p bf-app --lib this_pc_plays_silence -- --ignored`: PASS.
- [ ] **Step 5:** Commit `bf-app: sound output and the player`.

### Task 17: The shell: toolbar, bout bar, status bar, views, keys

**Files:**
- Create: `src/screens/shell.rs`, `src/screens/mod.rs` (the `show` of each screen as an empty function for now),
  `tests/ui/shell.rs`, `tests/snapshots/shell_1440x900.png`
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `AppState`, `ViewModel`, `Action`, `Effect` (Tasks 9, 12); `EngineHandle`, `channel`; `Writer`;
  `Player`, `AudioOut`, `NoAudio`; the widgets; `theme::install`; `fixtures::base::vm`.
- Produces:
  ```rust
  // app
  pub struct Boot { pub engine: EngineHandle, pub writer: Option<Writer>, pub audio: Box<dyn AudioOut>,
      pub settings: Settings, pub queue: Queue, pub notices: Vec<Notice>, pub cores: usize }
  pub struct ShellLayout { pub toolbar: egui::Rect, pub bout: egui::Rect, pub workspace: egui::Rect, pub stage: egui::Rect,
      pub timeline: egui::Rect, pub status: egui::Rect }
  pub struct BfApp { pub state: AppState, pub layout: ShellLayout, pub log: Vec<Action>, /* engine, writer, player, texture */ }
  impl BfApp {
      pub fn new(cc: &eframe::CreationContext<'_>, boot: Boot) -> BfApp;
      pub fn showing(cc: &eframe::CreationContext<'_>, vm: ViewModel) -> BfApp;   // screen tests: an engine that never answers, no sound, no writer
      pub fn sent(&mut self) -> Vec<Command>; }     // in `showing` mode: the commands sent since the last call
  // screens: every screen has this shape; shell.rs calls the one for `vm.view`
  pub fn show(ui: &mut egui::Ui, vm: &ViewModel, out: &mut Vec<Action>);
  // screens::shell
  pub fn toolbar(ui: &mut egui::Ui, vm: &ViewModel, out: &mut Vec<Action>);
  pub fn bout_bar(ui: &mut egui::Ui, vm: &ViewModel);
  pub fn notice(ui: &mut egui::Ui, vm: &ViewModel, out: &mut Vec<Action>);   // the newest notice, 34 tall, RAISED, with a box button "Dismiss"
  pub fn status_bar(ui: &mut egui::Ui, vm: &ViewModel);
  ```
- **How a frame runs.** `logic` (eframe calls it before every `ui`, and also while the window is hidden): take
  every waiting engine event, `state.apply` it, carry out the effects (`Send` to the engine, `Player` to the player,
  `SaveSettings` and `SaveQueue` to the writer); `player.tick()` and, when it gives a picture, load it into the
  preview texture (`TextureOptions::NEAREST`); copy `player.view()` into `state.vm.transport`; when `playing`
  changed, send `Command::PreviewBusy`. `ui`: the keys, then the panels (toolbar 40, bout bar 48, status bar 24 and
  timeline 112 from the bottom, stage 410 on the right, the notice strip if any, the workspace), each screen pushing
  actions; then `state.dispatch` each action in order, log it, and carry out its effects. Ask for a repaint while
  `player.wants_repaint()`. On exit: `Command::Shutdown` and `writer.flush()`.
- **What the shell draws** is in the interface reference, section 4, with these additions: a fourth right-hand
  toolbar button "Settings"; "Queue" shows the count of rendering and waiting videos only when it is above 0 and
  goes to Render; "Update" is `buttons::disabled` with "Updates from inside the app arrive in a later version.";
  the bout bar shows "Handicap" on Seeds, Music and Render only; the right of the status bar is "{f} fighters, {t}
  themes" counting what is available (`plural`). Keys: 1 to 5 go to the five steps; they are taken away from the
  widgets (`consume_key`) and ignored while a text field has the keyboard (`ctx.egui_wants_keyboard_input()`). The
  player gets the settings' `workers` as its thread count when the app starts.
  In every screen test below and in Tasks 18 to 24, "the log" is `harness.state().log`: the actions pushed
  since the app was built.
- [ ] **Step 1: Write the failing tests** (each takes `gpu()`; `app_harness(REF, |cc| BfApp::showing(cc, base::vm()))`):
  `the_frame_has_the_reference_measurements` (`harness.state().layout`: toolbar `[0, 0]` to `[1440, 40]`, bout `[0, 40]`
  to `[1440, 88]`, workspace `[0, 88]` to `[1030, 764]`, stage `[1030, 88]` to `[1440, 764]`, timeline `[0, 764]` to
  `[1440, 876]`, status `[0, 876]` to `[1440, 900]`); Review Focus 5: `the_frame_at_the_minimum_size` (at `MIN`:
  workspace `[0, 88]` to `[870, 664]`, stage 410 wide, and every toolbar button's rect lies inside the window);
  `the_toolbar_names` (every label of section 4 is found, plus "Settings"); `clicking_a_step_goes_there` ("3 Seeds":
  `harness.state().state.vm.view == View::Step(Step::Seeds)`); `keys_1_to_5_switch_steps` (typing into a text field is
  covered in Task 23, the first screen that has one); `update_is_disabled_and_says_why`;
  `the_queue_count_shows_only_above_zero`; `the_bout_bar_reads_the_draft` ("Big top", "133.77 bpm", "not picked yet" are
  on screen; on Seeds with handicap `[2.0, 0.4]` also "×2.00 / ×0.40"); `the_status_bar_counts` ("30 fighters, 1
  theme"); `a_notice_can_be_dismissed`; `events_are_taken_in_logic` (`BfApp::new` with a `channel` whose other side
  emits `Event::Roster(sample::roster())`: after `harness.run()` the roster is in the view model and
  `Command::DescribeFight` arrived on the engine side); `nothing_is_unnamed`; `the_shell_matches`
  (`harness.snapshot("shell_1440x900")`).
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui shell::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after comparing with section 4 of the interface reference; run: PASS. Run
  `ballfight-studio.exe --smoke` once more.
- [ ] **Step 5:** Commit `bf-app: the shell`.

### Task 18: The stage column and the story timeline

**Files:**
- Create: `src/screens/stage.rs`, `src/screens/timeline.rs`, `tests/ui/timeline.rs`,
  `tests/snapshots/timeline_empty.png`, `tests/snapshots/timeline_story.png`
- Modify: `src/screens/shell.rs` (call the two), `src/fixtures/base.rs` (add `pub fn with_take() -> ViewModel`: the
  base on Seeds, handicap `[2.0, 0.4]`, the first sample story chosen, transport enabled at frame 942, loop on)

**Interfaces:**
- Consumes: `ViewModel { take, transport, draft, view }`, `StoryView`, `ribbon::{paint, x_of, t_of}`,
  `clock::{timecode, TOTAL}`, `scale::{preview_size, shade}`, `buttons`, `icons`, `Action::Transport`.
- Produces:
  ```rust
  pub fn stage::show(ui: &mut egui::Ui, vm: &ViewModel, picture: Option<&egui::TextureHandle>);
  pub fn stage::caption(vm: &ViewModel) -> String;
  pub fn timeline::show(ui: &mut egui::Ui, vm: &ViewModel, out: &mut Vec<Action>);
  ```
- **Stage.** The picture is drawn at `preview_size` of the column minus 24 for the caption (in physical pixels,
  divided by `pixels_per_point`), centred, named "Preview"; with the safe zone on, the five `shade()` rectangles are
  painted over it in `GROUND` at 60 % opacity. `caption`: no take, "Title card, also saved as the cover image";
  take loading, "Drawing seed {n}"; a problem, the problem text; a take on Seeds or Music, "Seed {n}, the real
  picture and sound"; elsewhere "{Left} vs {Right}, seed {n}".
- **Timeline.** As the interface reference, section 4. The play button is `icon_button` named "Play" or "Pause";
  after the time, `small` `muted`: "Building the sound" while `sound` is `Building`, "No sound device was found, so
  the preview is silent" for `NoDevice`. The lanes area is one control named "Story timeline" (`WidgetInfo::slider`
  with the time in seconds): pressing or dragging on it pushes `Transport(ScrubTo(t_of(x)))` every frame and
  `Transport(ScrubEnd)` on release. Keys, handled here and taken from the widgets, ignored while a text field has
  the keyboard and while the transport is not enabled: Space pushes `PlayPause`, the left and right arrows
  `Step(-1)` and `Step(1)`.
- [ ] **Step 1: Write the failing tests.** With `base::vm()`: `the_empty_state_says_what_to_do` ("The fight's
  story appears here once you pick a seed." is on screen; "Play", "Loop" and "Safe zone" are disabled; the time
  reads "0:00.00 / 1:01.00"); `the_title_card_caption`; `space_does_nothing_without_a_take`. With
  `base::with_take()`: `the_time_reads_the_frame` ("0:31.40 / 1:01.00"); `the_lanes_are_named` ("Flute", "Who
  leads", "Trombone", and the five ability names are on screen; "Snake dance" starts within 2 of `x_of(30.62, 120.0,
  1320.0) + 7`); `the_playhead_is_at_the_frame` (the pixel column at `x_of(31.4, 120.0, 1320.0)` holds white pixels
  in all three lanes); `clicking_the_story_scrubs` (a click at x = 780 in the lanes logs
  `Transport(ScrubTo(t))` with `(t - 30.5).abs() < 0.05`, then `ScrubEnd`); `space_plays_and_pauses` and
  `arrows_step_a_frame` (the log holds `PlayPause`, `Step(-1)`, `Step(1)`); `loop_and_safe_zone_toggle`;
  `the_preview_is_360x640_at_the_reference_size` and Review Focus 5 `it_fits_at_the_minimum_size` (310 x 552 at
  `MIN`, inside the stage); `the_safe_zone_shades_the_outside` (with a plain white picture and the safe zone on,
  the preview's pixel at internal (10, 10) is darker than `[200; 3]` and the one at (180, 300) is white);
  `nothing_is_unnamed`; `the_timeline_matches` (two snapshots).
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui timeline::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after comparing with section 4; run: PASS.
- [ ] **Step 5:** Commit `bf-app: stage column and story timeline`.

### Task 19: Screen: Match-up

**Files:**
- Create: `src/screens/matchup.rs`, `src/fixtures/matchup.rs`, `tests/ui/matchup.rs`,
  `tests/snapshots/matchup_1440x900.png`

**Interfaces:**
- Consumes: `ViewModel { roster, draft, pair_rules, fight_problem }`, `RosterEntry`, `FighterCard`, `ThemeEntry`,
  `chips::{roster_chip, portrait, fighter_bar}`, `pixel::{pixel_text, fit_scale}`, `buttons`, `slider`, `fields`,
  `Action::{PickFighter, SwapSides, SetTheme, SetBars, Go}`. Layout and wording: interface reference, 5.1.
- Produces: `pub fn show(ui, vm, out)`; `pub fn columns_for(width: f32) -> usize` (`((width + 6.0) / 126.0)` rounded
  down, kept in 2 to 4); `fixtures::matchup::vm()` (the base, with `pair_rules: ["A live lance knocks a risen cobra
  flat."]`).
- **Beyond the mock-up.** The roster's count reads "{n} fighters", or "{a} of {n} ready" when some are not
  available. The roster scrolls when it does not fit. A third row in the bottom strip: "Pair rules" (`muted`, 108
  wide) and the rules' texts, or "None for this pair" (`muted`). The tempo slider moves over the theme's `bars`
  range in whole bars and pushes `SetBars`; the value field shows `draft.tempo_bpm()` with " bpm" and typing a
  tempo picks the nearest bar count; the note reads "{bars} bars fit in 61 seconds". With `fight_problem` set, its
  text stands above the bottom strip in `TEXT_STRONG` and "Balance this match-up" is `disabled` with it. A chip
  click pushes `PickFighter { side: 0 }`, a right-click `side: 1`.
- [ ] **Step 1: Write the failing tests** (`app_harness(REF, |cc| BfApp::showing(cc, matchup::vm()))`):
  `every_word_of_the_mock_up` (each quoted text of 5.1 for both corners, the roster header, the five section names,
  "Theme", "Tempo", "Swap sides", "Balance this match-up", "34 bars fit in 61 seconds", "133.77 bpm");
  `the_roster_has_30_chips_in_5_sections`; `columns_for(498.0) == 4`, `columns_for(338.0) == 2`,
  `columns_for(2000.0) == 4`; `the_corner_cards_are_250_wide`; `a_click_and_a_right_click_pick_corners` (on
  "Oboe": the log is `[PickFighter { side: 0, name: "OBOE" }]`, then `side: 1`); `the_chosen_two_are_marked`
  ("Flute, left corner" and "Trombone, right corner" exist); `a_fighter_not_built_cannot_be_picked`;
  `swap_and_balance_buttons` (`SwapSides`; `Go(View::Step(Step::Balance))`); `the_tempo_slider_moves_in_bars`;
  `pair_rules_are_read_only` (the text is on screen and is not a control; with none: "None for this pair");
  `a_fight_problem_blocks_balancing`; Review Focus 5 `at_the_minimum_size` (at `MIN`: "Swap sides" and "Balance
  this match-up" lie inside the workspace, and the chip "Bass drum" can be brought on screen with
  `scroll_to_me()`); `nothing_is_unnamed`; `match_up_matches` (`harness.snapshot("matchup_1440x900")`).
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui matchup::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after reading the picture next to section 5.1 and listing in the report every difference
  from the mock-up with its reason (section 9 names the wanted ones); run: PASS.
- [ ] **Step 5:** Commit `bf-app: Match-up screen`.

### Task 20: Screen: Balance

**Files:**
- Create: `src/screens/balance.rs`, `src/fixtures/balance.rs`, `tests/ui/balance.rs`,
  `tests/snapshots/balance_1440x900.png`

**Interfaces:**
- Consumes: `ViewModel { draft, balance }`, `BalanceStats` (`win_share`, `ko_median`, `ko_p10`, `ko_p90`,
  `no_ko_share`, `lead_changes_mean`, `fit_rate`, `ko_hist`, `fights`, `in_range()`), `theme::large_fill`,
  `slider`, `fields`, `buttons`, `wording::{percent, seconds, times, thousands}`, `Action::{SetHandicap,
  ResetHandicap, AutoBalance, StopBalance, Go}`. Layout and wording: interface reference, 5.2.
- Produces: `pub fn show(ui, vm, out)`; `pub fn one_in(fit_rate: f64) -> String`; `fixtures::balance::vm()` (Balance,
  handicap `[2.0, 0.4]`, `sample::stats()`) and `fixtures::balance::running()` (`running: Some((9, 29))`, no
  numbers).
- **Beyond the mock-up.** The sliders run from 0.05 to 3.00 in steps of 0.01 (a handicap above 3 pins the knob at the
  end; the value field takes 0.05 to 12.00) and push `SetHandicap { released }`. Under the bar: "In range" when
  `in_range()`, else "{Name} wins too often" for the side above 60 %. `one_in`: "About 1 fight in {n}" with n = `1 /
  fit_rate` rounded, "None in this sample" for 0. While a balance runs: the bar is replaced by a `progress` and "Trying
  handicaps, step {done} of {total}", the two sliders and "Reset both to ×1.00" are disabled, and "Auto-balance" becomes
  the box button "Stop balancing" (`StopBalance`). With no numbers and nothing running: "Not measured yet. Auto-balance
  finds the handicap for you." in place of the bar, and the four numbers read "–". The histogram is 120 tall and shrinks
  to no less than 60 when the workspace is short; a bar's height is its count over the largest count; bars for seconds
  49 to 52 are `TEXT`, the rest `BAR_DIM`.
- [ ] **Step 1: Write the failing tests:** `every_word_of_the_mock_up` (the texts of 5.2 with the sample numbers:
  "Flute wins 47%", "Trombone wins 53%", "3,000 fights, on seeds the search never uses", "40%", "60%", "In range",
  "Damage handicap", "Reset both to ×1.00", "Auto-balance", "Flute damage", "Trombone damage", "×2.00", "×0.40",
  the note, the four titles, numbers and notes, "When the K.O. lands", its note, the five axis labels, the footer
  note, "Search seeds"); `one_in(0.083) == "About 1 fight in 12"`, `one_in(0.5) == "About 1 fight in 2"`,
  `one_in(0.0) == "None in this sample"`; `the_column_is_780_wide_and_centred`; `the_split_bar_follows_the_share`
  (the pixel at 46 % of the bar's width, mid height, is `large_fill` of FLUTE and at 48 % that of TROMBONE; the
  pixel columns at 40 % and 60 % hold white); `out_of_range_names_the_side` (`win_share` 0.71: "Flute wins too
  often"); `the_window_bars_are_lighter` (the bar for second 50 has `[0xe2; 3]` pixels, the one for second 40
  `[0x66; 3]`); `dragging_a_slider_sets_the_handicap_and_releases`; `the_buttons_push_their_actions`;
  `while_balancing_the_controls_wait` (`running()`: "Trying handicaps, step 9 of 29" and "Stop balancing" are on
  screen, "Flute damage" is disabled); `not_measured_yet_says_what_to_do`; Review Focus 5 `at_the_minimum_size` ("Search
  seeds" and "Auto-balance" lie inside the workspace at `MIN`); `nothing_is_unnamed`; `balance_matches`.
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui balance::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after reading the picture next to section 5.2, differences listed in the report; run: PASS.
- [ ] **Step 5:** Commit `bf-app: Balance screen`.

### Task 21: Screen: Seeds

**Files:**
- Create: `src/screens/seeds.rs`, `src/fixtures/seeds.rs`, `tests/ui/seeds.rs`, `tests/snapshots/seeds_1440x900.png`

**Interfaces:**
- Consumes: `ViewModel { filters, shortlist, take, draft, roster }`, `Shortlist { rows, fits_total, checked, state,
  workers }`, `StoryView`, `FilterSpec`, `Finish`, `SearchState`, `ribbon::small`, `chips::{object_row,
  fighter_bar}`, `buttons`, `fields`, `Action::{SetFilters, StopSearch, SearchOn, SelectSeed, UseSeed}`. Layout and
  wording: interface reference, 5.3.
- Produces: `pub fn show(ui, vm, out)`; `pub struct SeedColumns { pub ribbon: f32, pub winner: f32, pub hp_left:
  bool }`; `pub fn seed_columns(row_width: f32) -> SeedColumns`; `pub fn parse_seed(text: &str) -> Option<u64>`;
  `fixtures::seeds::vm()` (`with_take()`, the 14 sample stories, `fits_total: 212`, `checked: 18_240`, `Searching`,
  7 workers) and `fixtures::seeds::empty(state: SearchState)`.
- **Beyond the mock-up.** `seed_columns`: the fixed parts take 348; with what is left `r`, the winner column is 150
  and shows "{n} HP left" when `r >= 350`, else 96 without it; the ribbon is `r - winner`, kept in 120 to 400. The
  rows scroll. A row's accessible name is "Seed {n}, {Winner} wins at {t} s"; clicking it pushes `SelectSeed`. A
  row outside the beat sheet (shown only with the switch on) has "–" as its rank and, in place of the score, its
  first miss in `muted` ("K.O. at 47.2 s"). Each filter control pushes `SetFilters` with the whole new spec; the
  ability dropdown lists "Any ability" and the six ability names of the two fighters' cards and stores the roster
  name. The footer by state: `Searching` "{fits} fights fit the beat sheet, from {checked} checked. Still
  searching." with "Stop search"; `Stopped` "... Search stopped." with "Keep searching" (`SearchOn`); `Full` "200 fights
  fit the beat sheet, from {checked} checked. The shortlist is full."; counts through `thousands`. With no rows:
  "No fights fit the beat sheet yet. The search is still running." while searching, else "No fights fit the beat
  sheet in {checked} checked. Keep searching, or use the switch on the left to show fights outside the beat
  sheet." "Use seed {n}" is
  `disabled` with "Pick a fight first" when no seed is chosen and with the take's problem when it has one. The watch
  box: `parse_seed` accepts whole numbers up to 9,223,372,036,854,775,807; "Watch" pushes `SelectSeed` and is
  `disabled` with "Type a whole number" otherwise.
- [ ] **Step 1: Write the failing tests:** `every_word_of_the_mock_up` (the texts of 5.3; the footer reads "212 fights
  fit the beat sheet, from 18,240 checked. Still searching."); `seed_columns(792.0)` is ribbon 294, winner 150, HP
  shown; `seed_columns(632.0)` is ribbon 188, winner 96, no HP; `seed_columns(1200.0).ribbon == 400.0`;
  `parse_seed("20418") == Some(20418)`, and `None` for "", "abc", "-3", "1.5", "99999999999999999999";
  `the_filters_column_is_230_wide`; `fourteen_rows_ranked` (rank 1 is the chosen row and has the selected look);
  `clicking_a_row_selects_its_seed`; `each_filter_pushes_the_whole_spec` (clicking "Trombone" under Winner logs
  `SetFilters` with `winner: Some(1)` and the other fields unchanged; "Close" gives `finish: Finish::Close`; the switch
  gives `show_outside: true`); `the_footer_by_state` (three states); `empty_states_say_what_to_do` (both texts);
  `a_row_outside_the_beat_sheet_is_marked`; `use_seed_names_the_seed_and_pushes`; `use_seed_waits_for_a_fight`;
  `watching_a_typed_seed`; Review Focus 5 `at_the_minimum_size` (the "Use seed" button, "Stop search" and "Watch" lie
  inside the workspace at `MIN`; the ribbon column is at least 120 wide); `nothing_is_unnamed`; `seeds_matches`.
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui seeds::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after reading the picture next to section 5.3, differences listed in the report; run: PASS.
- [ ] **Step 5:** Commit `bf-app: Seeds screen`.

### Task 22: Screen: Music

**Files:**
- Create: `src/screens/music.rs`, `src/fixtures/music.rs`, `tests/ui/music.rs`, `tests/snapshots/music_1440x900.png`

**Interfaces:**
- Consumes: `ViewModel { draft, roster, take }`, `MusicView { checks, lanes, markers }`, `Lane`, `LaneGroup`,
  `CheckLine`, `ThemeEntry`, `FighterCard { music_title, music_text, can_play_bass }`, `theme::large_fill`,
  `ribbon::x_of`, `buttons`, `fields`, `icons`, `Action::{SetTheme, SetChordLoop, SetBass, Go}`. Layout and wording:
  interface reference, 5.4; what is live: the table "What is live and what waits" at the top of this plan.
- Produces: `pub fn show(ui, vm, out)`; `fixtures::music::vm()` (`with_take()` on Music with `sample::music()`, bass
  `Some(1)`) and `fixtures::music::no_seed()`.
- **Beyond the mock-up.** Live controls: the theme dropdown (`SetTheme`), the chord-loop buttons (`SetChordLoop`),
  "Who plays bass" (`SetBass`; a fighter whose card has `can_play_bass: false` is a segment with the reason "{Name}
  has no bass part in this version"). Stated placeholders, all drawn `disabled`: the key dropdown ("The key follows
  the theme in this version."), the music seed field and "Re-roll music" ("New takes arrive with the music
  system."), with the line under them reading "New takes arrive with the music system. This version plays the
  series' own arrangement." The lanes come from `MusicView.lanes` (fighter lanes in the fighter's large fill with a
  1 px top edge in its colour, band lanes in `BAND_CLIP` under a `MUTED` edge; clips carry no text) and the marker
  lane from `markers`. In place of "What the music listens for": the heading "Loudness", the `small` `muted` line
  "Checked as soon as the take is built, so a problem shows here and not in the queue.", one row per `CheckLine` (a
  `Done` icon and `text` when it passed, a `Todo` icon and `TEXT_STRONG` when it did not, then the note), and the
  `small` `muted` line "Music that follows the fight, with fills on lead changes and fresh lines for every take,
  arrives with the music system." While the sound is not built: "Building the sound for seed {n}" in place of the
  lanes. With no seed: "Pick a seed first. The music is built for the fight you choose." and the box button "Go to
  seeds".
- [ ] **Step 1: Write the failing tests:** `the_words_that_stay` ("Theme", "Key", "harmonic minor, from the theme",
  "Chord loop", "Who plays bass", "The other fighter takes the melody.", "This take", "Music seed", "Re-roll
  music", both personalities, "Tempo is set on Match-up, because it changes the fight.", "Arrangement", "How this
  take follows the fight you picked", "Fighters", "Band", the lane names of the sample, "Sounds right?", "Go to
  render"); `the_controls_column_is_300_wide`; `placeholders_are_disabled_and_say_why` (the key dropdown, the music
  seed and "Re-roll music" are disabled; the stated line is on screen; "What the music listens for" is not);
  `the_flute_cannot_take_the_bass` (clicking the "Flute" segment logs nothing; "Trombone" is the chosen one);
  `choosing_a_chord_loop_pushes` (`SetChordLoop(1)`); `loudness_lines_show_pass_and_fail` (a failing `CheckLine {
  title: "Loudness", pass: false, note: "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB."
  }` shows its note); `clips_sit_at_their_times` (a clip from 3.6 s to 20 s starts within 1 of `x_of(3.6, lanes
  left, lanes width)`); `without_a_seed_it_says_what_to_do` ("Go to seeds" logs `Go(View::Step(Step::Seeds))`);
  `go_to_render_pushes`; Review Focus 5 `at_the_minimum_size` ("Go to render" inside the workspace; "Re-roll music"
  can be scrolled to); `nothing_is_unnamed`; `music_matches`.
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui music::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after reading the picture next to section 5.4, differences listed in the report; run: PASS.
- [ ] **Step 5:** Commit `bf-app: Music screen`.

### Task 23: Screen: Render

**Files:**
- Create: `src/screens/render.rs`, `src/fixtures/render.rs`, `tests/ui/render.rs`,
  `tests/snapshots/render_1440x900.png`

**Interfaces:**
- Consumes: `ViewModel { draft, caption_tab, folder, ffmpeg, settings, take, queue, selected_job }`, `Queue::{shown,
  counts, job, rendering}`, `Job`, `JobState`, `StepLine`, `StepKey`, `StepState`, `FailHint`, `STEP_ORDER`,
  `chips::{fighter_bar, object_row}`, `buttons`, `fields`, `icons`, `wording`, `Action::{SetCaptionTab, SetCaption,
  ChangeFolder, CreateFolder, AddToQueue, Queue}`. Layout and wording: interface reference, 5.5.
- Produces: `pub fn show(ui, vm, out)`; `pub fn step_title(key: StepKey) -> &'static str` and `pub fn
  step_note(key: StepKey) -> &'static str` (the table of 5.5; no note is `""`); `pub fn why_not_add(vm: &ViewModel)
  -> Option<String>`; `fixtures::render::vm()` (`with_take()` on Render, `sample::captions()`, folder `Ok { taken:
  false }`, ffmpeg found, and the mock-up's four jobs: TUBA vs BASS DRUM rendering at frame 1,171 with its first
  seven steps done, ALTO SAX vs TENOR SAX waiting, OBOE vs BASSOON failed at Loudness with the mock-up's two texts
  and `FailHint::OpenInMusic`, CELLO vs CONTRABASS done in 48 s).
- **Beyond the mock-up.** The text box shows the caption of `caption_tab` and pushes `SetCaption` on every edit; its
  label reads "Text and tags for {Instagram | TikTok | Shorts}". "Saves to" shows `<output folder>\<folder_stem>\`;
  with `Ok { taken: true }` a `small` `muted` line adds "That folder exists, so this one will be named
  {stem}_seed{n}." `why_not_add`, in this order: no seed, "Pick a seed first."; a take problem, that text; folder
  `Missing`, "The output folder {path} is missing. Create it, or choose another folder." (with a box button "Create
  folder"); `NotWritable`, "The output folder {path} cannot be written to. Choose another folder."; ffmpeg not
  found, "ffmpeg was not found on this PC, so videos cannot be saved yet. Install ffmpeg, then start the app
  again."; the text stands in `TEXT_STRONG` beside the button, which is then `disabled`. The queue's header:
  "{r} rendering, {w} waiting", "Paused. {w} waiting" or "Nothing is queued. Add this fight to render it."; the
  button is "Pause queue" or "Resume queue"; with two or more waiting, `small` `muted` "Drag a waiting video to
  change the order". Rows follow `queue.shown()` and scroll. A row by state: rendering, the progress bar and
  "{step_title of the step in progress} {percent of frames}" in mono (the percent only for Frames), "Cancel";
  waiting, "Waiting" (or "Waiting. It was interrupted when the app closed."), "Remove", and it can be dragged onto
  another waiting row (`Move`) or moved from its right-click menu ("Move up", "Move down", "Render next"); failed,
  the headline in `TEXT_STRONG`, the detail in `muted`, then "Queue again", the hint's button ("Open in Seeds",
  "Open in Music" or "Change folder"; none for the others) and "Remove"; cancelled, "Cancelled. Nothing was saved.",
  "Queue again", "Remove"; done, "Done in {s} s. All checks passed.", "Open folder". Clicking a row selects it. The
  check list shows the selected job, else the rendering one, else "Nothing is rendering. A video's checks show
  here while it renders."; each line's note is the job's own when it has one, else `step_note`; Frames reads
  "{done} of {total}" (`thousands`).
- [ ] **Step 1: Write the failing tests:** `every_word_of_the_mock_up` (the texts of 5.5 as the fixture gives them,
  among them "Drawing frames 64%", "1,171 of 1,830", "Stopped at the loudness check", "Done in 48 s. All checks
  passed.", "1 rendering, 1 waiting"); `the_check_list_is_330_wide_with_ten_lines`; `the_caption_tabs_switch_the_text`
  (`SetCaptionTab(Platform::TikTok)`; the label follows `caption_tab`); `typing_edits_the_caption` (type "!" at the end:
  the log ends with `SetCaption { platform: Instagram, text }` whose text ends with "!");
  `keys_do_not_fire_while_typing` (with the box focused, pressing 3 and Space logs no `Go` and no `Transport`);
  `why_not_add` for each of the five cases, in that order; `add_to_queue_pushes` and
  `add_to_queue_waits_with_the_reason`; Review Focus 1 `a_missing_folder_offers_to_create_it`;
  `each_job_state_has_its_words_and_buttons` (the four fixture rows, plus a cancelled one and an interrupted one);
  `the_failed_job_names_the_check_and_the_numbers` (Review Focus 4: the fixture's two texts are on screen, "Open in
  Music" logs `Queue(OpenIn { step: Step::Music, .. })`, "Queue again" logs `Requeue`); `pause_and_resume`;
  `the_menu_moves_a_waiting_job`; `dragging_reorders` (with three waiting jobs, dragging the third onto the first logs
  `Queue(Move { to: 0, .. })`); Review Focus 5 `at_the_minimum_size` ("Add to queue" and "Pause queue" inside the
  workspace; every row's buttons can be scrolled to); `nothing_is_unnamed`; `render_matches`.
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui render::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless after reading the picture next to section 5.5, differences listed in the report; run: PASS.
- [ ] **Step 5:** Commit `bf-app: Render screen`.

### Task 24: Views: Library and Settings

**Files:**
- Create: `src/screens/library.rs`, `src/screens/settings.rs`, `src/fixtures/library.rs`,
  `src/fixtures/settings.rs`, `tests/ui/library.rs`, `tests/ui/settings.rs`,
  `tests/snapshots/library_1440x900.png`, `tests/snapshots/settings_1440x900.png`

**Interfaces:**
- Consumes: `ViewModel { library, settings, cores, ffmpeg, folder, roster }`, `LibraryView`, `LibraryEntry`,
  `store::library::local_date`, `chips::{object_row, picture}`, `buttons`, `slider`, `fields`,
  `Action::{SelectVideo, Reopen, OpenVideoFolder, RefreshLibrary, ChangeFolder, SetWorkers}`.
- Produces: `library::show`, `settings::show`; `fixtures::library::vm()` (six entries with drawn covers, the second
  selected, one with the problem "Made by a newer version of the app."), `fixtures::library::empty()`,
  `fixtures::library::missing()`; `fixtures::settings::vm()` (8 cores, 7 workers, ffmpeg "ffmpeg 9.0.2").
- **Library** (§4): a header 40 tall, "Library" (`section`), `small` `muted` "{n} videos in {folder}" (`plural`),
  a spacer, box button "Look again" (`RefreshLibrary`). Then a scrolling grid of tiles 16 apart, padded 16: each
  tile an `object_row` 196 wide holding the cover at 180 x 320 (`chips::picture`; a `RAISED` block until the cover
  arrives), the title (`body`) and the date (`small` `muted`, `local_date`), named "{title}, {date}"; a tile with a
  problem shows the problem in `small` `muted` under the date. Clicking pushes `SelectVideo`. A footer 46 tall: the
  selected video's title, date and "seed {n}", a spacer, box button "Open folder" (`OpenVideoFolder`) and primary
  button "Reopen" (`Reopen`; `disabled` with the entry's problem). Empty: "No videos yet. Render one and it appears
  here." With `library.problem` (Review Focus 1): its text and "Choose another folder in Settings, or render a
  video to create it."
- **Settings** (§4), one column 560 wide, padded 28, rows 16 apart, each a `muted` label 150 wide: "Output folder"
  (a `read_field` and box button "Change folder"); "Worker threads" (a slider from 1 to `cores` in whole steps, a
  value field, then `small` `muted` "One core stays free for the preview. This PC has {cores}."); "Video encoder"
  ("{version}, found on this PC", or "ffmpeg was not found. Install ffmpeg, then start the app again. A later
  version brings its own."); "Version" ("{app_version}. The Update button starts working in a later version.").
- [ ] **Step 1: Write the failing tests.** Library: `six_tiles_newest_first`; `a_tile_is_named_by_title_and_date`;
  `clicking_selects_and_the_footer_follows`; `reopen_and_open_folder_push`;
  `a_video_from_a_newer_version_cannot_be_reopened` (its "Reopen" is disabled and the reason is on screen);
  `the_empty_library_says_what_to_do`; `a_missing_folder_says_what_to_do`; `nothing_is_unnamed`; `library_matches`.
  Settings: `the_four_rows`; `the_worker_slider_stops_at_the_cores` (dragging it to the right end logs `SetWorkers(8)`);
  `change_folder_pushes`; `ffmpeg_missing_says_what_to_do`; `nothing_is_unnamed`; `settings_matches`. Review Focus 5:
  `both_views_at_the_minimum_size` ("Reopen" and "Change folder" inside the workspace at `MIN`).
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui library:: settings::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Bless both pictures; run: PASS.
- [ ] **Step 5:** Commit `bf-app: Library and Settings views`.

### Task 25: The sample engine, `--fixtures`, whole flows and the final pictures

**Files:**
- Create: `src/seam/fake.rs`, `tests/ui/flows.rs`, `tests/ui/wording.rs`, `tests/ui/small_window.rs`,
  `tests/snapshots/*_1280x800.png` (seven: matchup, balance, seeds, music, render, library, settings)
- Modify: `src/main.rs`, `README.md`

**Interfaces:**
- Consumes: everything above.
- Produces:
  ```rust
  pub enum Pace { Instant, Live }
  pub fn spawn(waker: Waker, pace: Pace) -> EngineHandle;   // seam::fake: answers every Command from seam::sample
  ```
  The sample engine, per command: `LoadRoster`, the sample roster. `DescribeFight`, the pair rule "A live lance knocks a
  risen cobra flat." when FLUTE meets TROMBONE (none otherwise), `sample::captions()`, the sample title card.
  `AutoBalance`, 29 `BalanceProgress` then `Balanced { handicap: [2.0, 0.4], stats: sample::stats() }`. `Measure`,
  `Measured` with the sample numbers and `win_share` moved by `0.1 * (handicap[0] / handicap[1]).ln()` from 0.47, kept
  in 0.05 to 0.95. `StartSearch`, fourteen `Shortlist` events, each one story longer, `checked` rising by 1,303 each
  time, the last one `Stopped`. `SetFilters`, the list again, filtered here (winner, K.O. bounds, lead changes, `Close`
  = winner at 12 HP or less; `Comeback` keeps none). `LoadTake`, `TakePicture` with `sample::story(seed)` and
  `sample::frames`, then `TakeSound` with `sample::audio()` and `sample::music()`; a seed ending in 13 gives `TakeFailed
  { problem: "Seed {n} has no K.O. before 58 s, so it cannot be made into a video. Pick another seed." }`. `Enqueue`,
  `Enqueued` with a `JobSpec` built from the draft. `RunJob`, the ten steps in order with `Frames` every 61 frames
  (`seconds` rising by 0.2 each time), then `Done`; a job whose seed ends in 2 fails at Loudness with the Render
  mock-up's two texts and `FailHint::OpenInMusic`. `CancelJob`, `Cancelled`. `ScanLibrary`, six sample entries and their
  covers. `OpenFight` and `OpenFightText`, `FightOpened` with the base draft and seed 20418. `CheckTools`, `Ok("ffmpeg
  (sample)")`. `CheckFolder`, `Ok { taken: false }`. `CreateFolder`, the same. `PickFolder`, `FolderPicked(None)`. The
  rest, nothing. With `Pace::Instant` every answer is sent at once; with `Pace::Live` progress events are 40 ms apart
  and a job takes 6 s. A command that carries a generation is answered with it. `main.rs`: `--fixtures` runs the app on
  `fake::spawn(.., Pace::Live)` with real sound out (`open_audio()`), storage under a temp folder, and the status bar's
  right side reading "Sample data, not a real fight"; without the flag the app says in a window "The engine is not
  connected in this build. Start it with --fixtures to see the interface." (Task 35 replaces this). `--data-dir
  <folder>` puts settings, queue and cache there.
- [ ] **Step 1: Write the failing tests.** `flows.rs` (`BfApp::new` on `fake::spawn(.., Pace::Instant)`, storage under a
  temp folder, `NoAudio`; drive by clicks only): `a_whole_video_in_clicks` ("Balance this match-up"; the numbers appear
  and the handicap reads "×2.00"; "Search seeds"; fourteen rows; click row 1; "Use seed {n}"; "Go to render"; "Add to
  queue"; the row reads "Done in"; "Queue" shows no count); `the_bad_job_stops_and_the_next_one_runs` (queue seed 902,
  then another: the first reads "Stopped at the loudness check", the second "Done in"; Review Focus 4);
  `a_late_shortlist_is_not_shown` (Review Focus 3, on a `seam::channel` the test answers by hand: go to Seeds, note the
  generation of the `StartSearch` that arrives, press "Swap sides" on Match-up, then emit a `Shortlist` with the noted
  generation: back on Seeds no row of it is shown); `a_seed_with_no_ko_says_so` (watch seed 13: the stage reads the
  problem and "Use seed" is disabled); `the_queue_survives_a_restart` (on a hand-answered channel that sends `Enqueued`
  twice and never answers `RunJob`: drop the app after `writer.flush()`, load settings and queue from the same folder
  into a second app: one video waits, the interrupted one waits too, the notice about the interrupted video shows, and
  "Resume queue" sends `RunJob`); `reopening_from_the_library`. `wording.rs`: `sentence_case_everywhere` (for each of
  the fixture view models of Tasks 17 to 24, every accessible label and every text on screen passes this rule: after its
  first word, a word may start with a capital letter only if it follows ". " or ": ", or holds a digit, a backslash or
  an underscore, or is a note or chord name (`[A-G]` then an optional `#` or `b`, an optional `m`, `7`, `m7` or `maj7`,
  an optional octave digit), or is one of: BALLFIGHT, Studio, K.O., HP, PC, Instagram, TikTok, Shorts, LV2, LV3, LOW,
  the step and view names Match-up, Balance, Seeds, Music, Render, Library, Settings and Queue, a three-letter month, or
  a word of a fighter's or theme's name as shown); `no_retired_fighter_anywhere` (no label holds the word GUITAR, and
  none is exactly "Sax" or "SAX"); `no_button_is_vague` (no button is labelled "OK", "Yes", "No", "Submit" or "Apply").
  `small_window.rs` (Review Focus 5): for each of the seven fixtures at `MIN`, `harness.unnamed_widgets()` is empty, no
  control's rect lies outside the window, and `harness.snapshot("<screen>_1280x800")`.
- [ ] **Step 2:** Run `cargo test -p bf-app --test ui flows:: wording:: small_window::`. Expected: FAIL.
- [ ] **Step 3:** Implement. Write in `README.md`: how to run `--fixtures`; **how to bless pictures**
  (`UPDATE_SNAPSHOTS=true cargo test -p bf-app --test ui <module>::` rewrites only the pictures that differ; a
  failed comparison leaves `<name>.new.png` and `<name>.diff.png` beside the reference; a picture is blessed only
  after it has been read next to its section of `docs/studio/interface-reference.md`, and the differences from the
  mock-up are written in the task report); that drawing tests take the lock and why.
- [ ] **Step 4:** Run the whole crate ten times in a row (`cargo test -p bf-app`): PASS each time. Run
  `ballfight-studio.exe --fixtures --smoke`. `cargo clippy -p bf-app --all-targets -- -D warnings` and `cargo fmt
  --check` clean.
- [ ] **Step 5:** Commit `bf-app: sample engine, whole flows, final pictures`. Block A is done: the whole interface
  runs on sample data.

---

# Block B: the real engine behind the seam

Block B starts when milestone 1's Task 30 is merged. The signatures under "Consumes" are copied from the M1 plan and,
where the code was already merged or on a lane when this plan was written, from the code: `bf_core::fightfile` (the
folder `bf-core/src/fightfile/`), `bf_core::show` (`Show`, `Cap`, `captions`, `Show::solo_times`), the registry in
`bf-content`, `bf_sim::{run, setup_from, Rules, Setup, SimError}`, and `bf_story::{story, score, Story, Verdict,
BeatSheet}` with `BeatSheet::series(fighters: [&FighterCfg; 2]) -> BeatSheet`. The dev crate `bf-testdata`
(`reference(rel)`, `read_png`, `write_png_rgb`) is there for any test that needs a reference file; add it under
`[dev-dependencies]` when used. **Step 1 of every task begins the same way: open the named files, confirm each signature
against the code as merged, and where one differs, adapt this task's code to the merged one (never the other way round)
and say so in the report.** `bf-cli` is the worked example of calling the engine: where this plan and
`studio/crates/bf-cli/src/cmd/*.rs` disagree about how a function is called, do what `bf-cli` does. Engine tests live in
`#[cfg(test)]` modules of `src/engine/*.rs`, are run with `cargo test -p bf-app --release --lib engine::`, and wait for
events with one helper, `wait_for(handle, seconds, |event| ...) -> Event`, which polls every 10 ms and panics with the
events it saw when the time runs out.

### Task 26: What the interface shows about fighters and themes

**Files:**
- Create: `studio/crates/bf-content/src/cards.rs`, `studio/crates/bf-content/src/starter.rs`
- Modify: `studio/crates/bf-content/src/lib.rs` (declare and re-export)

**Interfaces:**
- Consumes (`bf-content` and `bf-core` as merged): `roster() -> &'static [FighterCfg]`, `available() -> Vec<&'static
  str>`, `PORTED: [&str; 2]`, `pair_rules(a: &str, b: &str) -> BTreeMap<String, Value>`, `theme_names() ->
  Vec<&'static str>`, `validate(ff: &FightFile) -> Result<(), ContentError>`; `FightFile::parse(text) ->
  Result<FightFile, FightFileError>`, `FightFile { fighters, handicap, seed, series_tag, pair_rules, picture,
  captions, app_version, music, .. }`, `Captions { caption: String, instagram: String, tiktok: String, shorts:
  Vec<String> }`; `studio/fights/flute_vs_trombone.toml`.
- Produces:
  ```rust
  pub enum Section { Woodwinds, Brass, Strings, Keys, Percussion }
  pub struct FighterCard { pub weapon: &'static str, pub text: &'static str, pub abilities: [&'static str; 3],
      pub colour: &'static str, pub usually_plays: &'static str, pub music_title: &'static str,
      pub music_text: &'static str, pub can_play_bass: bool, pub emoji: &'static str, pub tags: &'static [&'static str] }
  pub struct ThemeCard { pub display: &'static str, pub text: &'static str, pub key: &'static str, pub scale: &'static str,
      pub chord_loops: &'static [[&'static str; 8]], pub bars: (u32, u32) }
  pub fn section(name: &str) -> Option<Section>;                // every name in roster()
  pub fn shown_order() -> Vec<&'static str>;                    // every roster name once, in score order
  pub fn card(name: &str) -> Option<&'static FighterCard>;      // every name in PORTED
  pub fn theme_card(name: &str) -> Option<&'static ThemeCard>;  // every name in theme_names()
  pub fn pair_rule_text(key: &str) -> Option<&'static str>;
  pub fn note_name(midi: i32) -> String;                        // 72 -> "C5"
  pub fn starter(left: &str, right: &str) -> Result<FightFile, ContentError>;
  pub fn draft_captions(left: &str, right: &str) -> Captions;
  ```
- **The content.** Sections and score order: Woodwinds FLUTE, OBOE, CLARINET, BASS CLARINET, BASSOON, ALTO SAX,
  TENOR SAX; Brass TRUMPET, HORN, TROMBONE, EUPHONIUM, TUBA, SOUSAPHONE; Strings CELLO, CONTRABASS, ACOUSTIC,
  ELECTRIC; Keys PIANO, ORGAN; Percussion BASS DRUM. The two cards carry the texts of the interface reference, 5.1
  and 5.4 (`abilities` are the three lines under the ability names, in the order unlock 1, unlock 2, low);
  `can_play_bass` is false for FLUTE and true for TROMBONE; emoji U+1FA88 for FLUTE and U+1F3BA for TROMBONE (the
  series uses the trumpet; there is no trombone emoji); tags `["#bandkids", "#marchingband"]` for both. `big_top`:
  "Big top", "A sinister circus in a minor key. Leaves the bass and the top line to the fighters.", key "E", scale
  "harmonic minor", one loop `Em Am B7 Em C Am B7 B7`, bars (24, 42). `pair_rule_text("swat_cobras")` is "A live
  lance knocks a risen cobra flat."
- **The starter** is the fight a new match-up begins from (§4: the fight file is the whole recipe). For FLUTE and
  TROMBONE in either order: the embedded `flute_vs_trombone.toml`, with the fighters in the asked order, handicap
  `[1.0, 1.0]`, no seed, no `series_tag` (§3: no episode number), `picture.fx_seed` `None`, `pair_rules =
  pair_rules(left, right)`, captions from `draft_captions` and this crate's version as `app_version`. Any other
  pair is an error whose text is "No music is written for {Left} vs {Right} yet." (names in sentence case).
- **Caption drafts** (§9). A name's tag is "#" and the name in lower case without spaces. `caption`: "{Left} vs
  {Right} {left emoji}{right emoji} Comment the next match-up 👇". `instagram`: "#ballfight", the two name tags, the
  two fighters' own tags without repeats, then "#musicmemes #musician #pixelart #lowpoly #gamedev #satisfying
  #music". `tiktok`: "#fyp #ballfight", the two name tags, then "#bandkid #musictok #pixelart #satisfying #bandtok
  #music". `shorts`: "{Left} vs {Right}"; "{Left} vs {Right}, every hit in key. Comment the next match-up";
  "#shorts #ballfight", the two name tags, "#music #pixelart".
- [ ] **Step 1: Confirm the interfaces, then write the failing tests** (unit, in the two files):
  `every_roster_fighter_has_a_section` (20 names; 7, 6, 4, 2 and 1 per section; `shown_order()` starts FLUTE, OBOE
  and ends BASS DRUM); `ported_fighters_have_cards` and `nobody_else_does`; `retired_fighters_have_nothing`
  (`section` and `card` of GUITAR and SAX are `None`); `note_name` of 72, 93, 40, 65 and 61 is "C5", "A6", "E2",
  "F4", "C#4"; `the_starter_is_a_valid_fight` (both orders pass `validate`; handicap `[1.0, 1.0]`; `seed` and
  `series_tag` are `None`; the `swat_cobras` rule is on); `the_two_orders_are_different_fights` (their
  `search_key(&[false, false])` differ); `a_pair_without_music_says_so` (`starter("FLUTE", "PIANO")` fails with "No
  music is written for Flute vs Piano yet."); `captions_are_drafted_from_emoji_and_tags`: for FLUTE and TROMBONE the
  caption is "Flute vs Trombone 🪈🎺 Comment the next match-up 👇", Instagram is "#ballfight #flute #trombone
  #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly #gamedev #satisfying #music", TikTok is "#fyp
  #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music" and Shorts is `["Flute vs
  Trombone", "Flute vs Trombone, every hit in key. Comment the next match-up", "#shorts #ballfight #flute #trombone
  #music #pixelart"]`.
- [ ] **Step 2:** Run `cargo test -p bf-content cards starter`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS; `cargo test -p bf-content` and the std-maths scan (`cargo test -p bf-core --test
  no_std_maths`) still pass.
- [ ] **Step 5:** Commit `bf-content: fighter and theme cards, starter fights, caption drafts`.

### Task 27: Three small additions to the engine

**Files:**
- Modify: `studio/crates/bf-core/src/show.rs`, `studio/crates/bf-story/src/stats.rs`, the file of
  `bf_picture::Picture`, `studio/crates/bf-content/src/lib.rs` (or the file `build_picture` lives in)

**Interfaces:**
- Consumes: `Show { grid, names, win, lose, t_ko, t_hs, t_solo, caps_ability, cap_ko, cap_wins, cap_comment,
  captions }` and `HITSTOP` (`bf_core::show`, as merged); `Grid::next_bar(&self, t: f64) -> f64`; `TitleCard::new(show:
  &Show, colors: [[f64; 3]; 2], darks: [[f64; 3]; 2], looks: ..., series_tag: Option<&str>) -> TitleCard`,
  `TitleCard::draw(&self, img: &mut Rgb8, t: f64)`, `TitleCard::cover_frame(&self) -> usize`; `look_for(name: &str) ->
  Box<dyn Look>`; `bf_story::Stats { n, decided, win_share, ko_median, ko_p10, ko_p90, no_ko_share,
  lead_changes_mean, fit_rate }` and `stats(setup_for, sheet: &BeatSheet, seeds: Range<u64>, workers: usize) ->
  Stats`; `Picture`.
- Produces:
  ```rust
  impl Show { pub fn placeholder(ff: &FightFile) -> Show; }   // no recording: grid and names from the file, win 0, lose 1,
                                                              // t_ko 50.0, t_hs 50.0 + HITSTOP, t_solo grid.next_bar(51.6), no captions
  pub fn title_preview(ff: &FightFile) -> Result<Rgb8, ContentError>;   // bf_content: the VS card on its cover frame, over a black 360 x 640 picture
  pub ko_hist: [u32; 28],                                     // new field of bf_story::Stats: K.O.s per second, 30 s (bin 0) to 58 s
  impl Picture { pub fn cover_frame(&self) -> usize; }        // only if the merged code has no such accessor
  ```
  The title card shows the same picture on Match-up and Balance, before any fight exists (§11, touch 2).
- [ ] **Step 1: Confirm the interfaces, then write the failing tests:** `a_placeholder_show_has_the_files_grid`
  (`placeholder(&ff).grid == ff.grid()`, the names are the file's, every caption list is empty);
  `the_title_preview_is_the_card` (for `starter("FLUTE", "TROMBONE")`: 360 x 640; two calls give the same bytes; the
  swapped order gives different bytes; fewer than 10 % of its pixels are black); `the_histogram_counts_ko_seconds`
  (for a run of 400 balance seeds: `ko_hist.iter().sum::<u32>() <= decided as u32`, and a fight whose K.O. is at 50.4
  s is counted in bin 20: check with a hand-built list through the helper that fills the bins).
- [ ] **Step 2:** Run `cargo test -p bf-core show`, `-p bf-story stats`, `-p bf-content title_preview`. Expected:
  FAIL.
- [ ] **Step 3:** Implement. Every other place that builds a `Stats` gets the new field.
- [ ] **Step 4:** Run: PASS; `cargo test --workspace` has no new failure; the std-maths scan passes.
- [ ] **Step 5:** Commit `engine: title preview without a fight, K.O. histogram, cover frame`.

### Task 28: The engine thread, the fight file behind a draft, the roster

**Files:**
- Create: `src/engine/mod.rs`, `src/engine/convert.rs`, `src/engine/content.rs`
- Modify: `src/lib.rs` (`pub mod engine;`), `studio/crates/bf-app/Cargo.toml` (add `bf-sim`, `bf-story`,
  `bf-picture`, `bf-sound`, `bf-content`, `bf-export`, all `.workspace = true`)

**Interfaces:**
- Consumes: Tasks 26 and 27; `seam::{Command, Event, EngineHandle, EngineSide, channel, Waker}`; `store::Paths`;
  `FightFile::{parse, to_toml, validate, grid, search_key}`; `Grid { n_bars, .. }`; `bf_content::{validate,
  music_feeds_fight(name) -> bool}`; `bf_picture::font::text_mask(s: &str, scale: u32) -> Img<u8>`;
  `bf_picture::sprite::{Mesh, render_sprite(mesh: &Mesh, radius_px: u32, yaw_deg: f64, roll_deg: f64) -> Img<u8>}`
  (RGBA) and the function `bf-content` uses to load a fighter's mesh; `bf_picture::{SAFE, SAFE_BUTTONS}`;
  `bf_export::folder_name(names: [&str; 2], seed: u64, root: &Path) -> PathBuf`.
- Produces:
  ```rust
  // engine
  pub struct EngineConfig { pub workers: usize, pub paths: Paths, pub clip: Option<std::ops::Range<usize>> }   // clip: tests render only these frames
  pub fn spawn(waker: Waker, config: EngineConfig) -> EngineHandle;
  // engine::convert
  pub fn fight_file(draft: &Draft) -> Result<FightFile, String>;
  pub fn draft_of(ff: &FightFile, toml: &str) -> Draft;
  pub fn caption_texts(c: &Captions) -> CaptionTexts;
  pub fn captions(t: &CaptionTexts) -> Captions;
  pub fn title_of(ff: &FightFile) -> String;           // "Flute vs Trombone, seed 20418", or without the seed
  pub fn plain(e: &dyn std::fmt::Display) -> String;   // an engine error as a sentence for the screen
  // engine::content
  pub fn roster() -> Roster;
  pub fn describe(gen: Gen, draft: &Draft) -> Event;   // Event::FightInfo
  ```
- **The rules.** One coordinator thread takes commands and never does long work itself: balance, search, takes,
  jobs, library scans and the folder dialog each run on their own thread with a cancel flag, and send their events
  through a clone of the `EngineSide` sender. This task answers `LoadRoster`, `DescribeFight`, `SetWorkers` and
  `Shutdown`; every other command is accepted and ignored until its task.
  - `fight_file`: start from `FightFile::parse(draft.base)` if there is one, else `starter(left, right)`; then set
    the fighters in the draft's order, `handicap`, `tempo_bpm = draft.tempo_bpm()`, `pair_rules =
    bf_content::pair_rules(left, right)`, `seed`, `music.theme`, `music.chord_loop` (the theme card's loop number
    `draft.chord_loop`, when it has one), the roles (when `draft.bass` is `Some(i)` and fighter i's card has
    `can_play_bass`: i plays "bass", the other "melody"), `music.music_seed`, the captions and `app_version`; then
    `bf_content::validate`. An error becomes `plain(..)`: "This fight cannot be used: {field}: {message}."
  - `draft_of`: the reverse; `bars` is `ff.grid().n_bars`; `chord_loop` is the place of the file's loop among the
    theme card's loops (0 if absent); `bass` is the fighter whose role is "bass"; `captions_edited` true; `base`
    is `toml`.
  - Captions both ways: the Instagram text is `caption`, a line break, `instagram`; the TikTok text is `caption`, a
    line break, `tiktok`; the Shorts text is `shorts` joined by line breaks. Back: `caption` is the first line of
    the Instagram text and `instagram` the rest; `tiktok` is the TikTok text after its first line; `shorts` is the
    Shorts text's lines (three at most; an empty text gives an empty list).
  - `roster()`: one entry per `shown_order()` name: `display` through `wording::display_name`, the section, the
    colour, `available`, `music_feeds_fight`, the mask from `text_mask(name, 1)` (a pixel is on where the mask is
    not 0), and for available fighters the card: the weapon and texts from `card(name)`; the abilities tagged
    "LV2", "LV3", "LOW" with the roster's ability names in sentence case; `range` is `note_name` of the two ends of
    `register` joined by " to "; `usually_plays`, `colour_name`, the music lines, `can_play_bass`; `portrait` is
    `render_sprite` at radius 22, yaw 30, roll 0. Themes from `theme_names()` and `theme_card`. `vs` is the mask of
    "VS"; `start` is `["FLUTE", "TROMBONE"]`; `app_version` is this crate's.
  - `describe`: with `fight_file(draft)` `Ok`: the pair rules as texts (`pair_rule_text`, or the key itself), the
    captions from `draft_captions`, the title card from `title_preview`, no problem. With an error: no rules, empty
    captions, no card, and the problem; for a pair with no starter the problem reads "No music is written for {Left}
    vs {Right} yet, so it cannot be balanced or rendered. Pick another pair."
- [ ] **Step 1: Confirm the interfaces, then write the failing tests:** `the_real_roster` (20 fighters in score
  order; FLUTE and TROMBONE available, the other 18 not; none is GUITAR or SAX; FLUTE's card: weapon "Snake
  charmer", range "C5 to A6", abilities "Flutter tongue", "Snake dance", "Piccolo" tagged LV2, LV3, LOW;
  TROMBONE's range "E2 to F4" and `can_play_bass`; one theme, "Big top", bars (24, 42));
  `the_sample_font_is_the_series_font` (`roster()`'s masks for FLUTE and TROMBONE and `vs` equal
  `seam::sample::mask` of the same words); `safe_zone_numbers_agree` (`preview::scale::SAFE` and `SAFE_BUTTONS`
  equal `bf_picture::SAFE` and `SAFE_BUTTONS` as four numbers each); `folder_names_agree` (`wording::folder_stem`
  equals the last part of `bf_export::folder_name` for FLUTE and TROMBONE in an empty root);
  `a_draft_becomes_a_valid_fight_file` (the starting draft with seed 7: `fight_file` is `Ok`, validates, has
  `tempo_bpm == 34.0 * 4.0 * 60.0 / 61.0`, the cobra rule and `seed == Some(7)`);
  `swapping_sides_swaps_the_file` (fighters and handicap); `music_and_captions_keep_the_search_key` (changing the
  caption, the theme field and the music seed leaves `search_key(&[false, false])` equal; changing `bars` or a
  handicap does not); `a_file_round_trips_through_a_draft` (for the episode file `studio/fights/
  flute_vs_trombone.toml`: `fight_file(&draft_of(&ff, text))` equals `ff` except `tempo_bpm` (snapped),
  `app_version` and `pair_rules` order; its lines, solos and title hits are kept, which proves the base carries
  them); `captions_map_both_ways`; `a_pair_without_music_is_a_plain_problem`; and through `spawn`:
  `the_engine_answers_the_roster_and_the_fight` (`LoadRoster` gives `Event::Roster` within 5 s; `DescribeFight {
  gen: 4, .. }` gives `FightInfo` with `gen == 4`, the cobra rule's sentence, the drafted caption and a 360 x 640
  title card).
- [ ] **Step 2:** Run `cargo test -p bf-app --release --lib engine::`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS; `cargo test -p bf-app` (Block A's tests) still passes.
- [ ] **Step 5:** Commit `bf-app: engine thread, fight files behind drafts, the real roster`.

### Task 29: Balance through the seam

**Files:**
- Create: `src/engine/balance.rs`
- Modify: `src/engine/mod.rs` (route `AutoBalance`, `Measure`, `StopBalance`)

**Interfaces:**
- Consumes: `bf_sim::setup_from<'a>(ff: &FightFile, roster: &'a [FighterCfg], seed: u64) -> Result<Setup<'a>,
  SimError>`; `bf_story::{BeatSheet, BeatSheet::series(fighters: [&FighterCfg; 2]) -> BeatSheet, BALANCE_SEED0 /* 1 <<
  40 */, Stats, stats(setup_for, sheet: &BeatSheet, seeds: Range<u64>, workers: usize) -> Stats, BalanceResult {
  handicap: [f64; 2], confirm: Stats, in_band: bool, steps: Vec<BalanceStep> }, auto_balance(setup_for: &(dyn Fn([f64;
  2], u64) -> Setup + Sync), sheet: &BeatSheet, start: [f64; 2], workers: usize, progress: ..., cancel: &AtomicBool) ->
  BalanceResult}`; the set-up closures, built the way `bf-cli`'s `cmd/balance.rs` builds them (if it uses a private
  helper, move the helper into `bf-story` as a public function and call it from both); `convert::fight_file`;
  `model::BalanceStats`.
- Produces: `pub fn stats_view(s: &Stats, seconds: f64) -> BalanceStats`; the three commands' behaviour.
  `AutoBalance` runs `auto_balance` from the draft's handicap and sends `BalanceProgress { done, total }` as it
  goes (`total` is 29: the 28 evaluations and the confirmation; if the merged `progress` reports otherwise, count
  what it gives) and then `Balanced { gen, handicap, stats: stats_view(&result.confirm, ..) }`. `Measure` runs
  `stats` on the 3,000 confirmation seeds, `BALANCE_SEED0 + 1_000_000` onward, so a measured handicap and a
  balanced one are judged on the same fights, and sends `Measured`. `win_share` in `BalanceStats` is the left
  fighter's share of decided fights: convert if `Stats` means something else. A new `AutoBalance` or `Measure`, or
  `StopBalance`, sets the running one's cancel flag; a cancelled run sends nothing (Review Focus 3).
- [ ] **Step 1: Confirm the interfaces, then write the failing tests:** `stats_view_carries_every_number` (unit);
  `the_episodes_handicap_measures_in_range` (`Measure` for handicap `[2.0, 0.4]`: `Measured` with `fights ==
  3_000`, `in_range()`, 28 histogram bins, `seconds > 0.0`); `auto_balance_reports_and_lands` (from `[1.0, 1.0]`:
  at least one `BalanceProgress`, then `Balanced` whose numbers are `in_range()` and whose `handicap[0] /
  handicap[1]` is between 2.5 and 10, the neighbourhood milestone 1 found); `a_newer_request_silences_the_older`
  (`AutoBalance { gen: 1 }` then at once `Measure { gen: 2 }`: `Measured { gen: 2 }` arrives and no `Balanced { gen:
  1 }` within 3 s after it); `stop_silences_it`.
- [ ] **Step 2:** Run `cargo test -p bf-app --release --lib engine::balance`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS. Note the seconds an auto-balance and a measurement took for `m2-measurements.md`.
- [ ] **Step 5:** Commit `bf-app: balance through the seam`.

### Task 30: Search, filters and the shortlist cache

**Files:**
- Create: `src/engine/search.rs`
- Modify: `src/engine/mod.rs` (route `StartSearch`, `StopSearch`, `SetFilters`, `PreviewBusy`)

**Interfaces:**
- Consumes: `bf_story::{BeatSheet, Story { seed, winner: Option<usize>, t_ko:
  Option<f64>, lead_changes: u32, lc_t: Vec<f64>, winner_hp: f64, firsts: Vec<(f64, usize, String)>, caps, max_gap,
  lows, lead: Vec<(f64, f64)> }, Verdict { fits: bool, score: f64, misses: Vec<String> }, Candidate { seed: u64, story:
  Story, verdict: Verdict }, SearchOpts { from: u64, stop_fits: usize, max_seeds: Option<u64>, workers: usize },
  search(setup_for: &(dyn Fn(u64) -> Setup + Sync), sheet: &BeatSheet, opts: &SearchOpts, progress: &(dyn
  Fn(SearchProgress) + Sync), cancel: &AtomicBool) -> Vec<Candidate>, Filters { winner: Option<usize>, ko: Option<(f64,
  f64)>, min_lead_changes: Option<u32>, close_finish: bool, comeback: bool, ability: Option<String>, show_misfits: bool
  }}`, `Filters::keep(&self, c: &Candidate) -> bool`, the beat sheet from `BeatSheet::series`;
  `FightFile::search_key(&self, feeds_fight: &[bool]) -> String`;
  `bf_content::music_feeds_fight`; `store::shortlist_cache::{CachedShortlist, load_shortlist, save_shortlist}`;
  `model::{StoryView, AbilityMark, Shortlist, SearchState, FilterSpec, Finish}`.
- Produces:
  ```rust
  pub const CHUNK: u64 = 512;  pub const FITS: usize = 200;  pub const OTHERS: usize = 300;
  pub fn story_view(c: &Candidate, t_fight: f64) -> StoryView;   // ability names in sentence case
  pub fn filters(spec: &FilterSpec) -> bf_story::Filters;        // Close -> close_finish, Comeback -> comeback, show_outside -> show_misfits
  ```
- **The rules.** `StartSearch`: the key is `ff.search_key(&feeds)`; a cached shortlist for this app version and key is
  loaded and its seeds are fought again to rebuild the candidates (fights are reproducible), then sent. Then, in a loop,
  `search` is called for the next `CHUNK` seeds (`from: next_seed`, `stop_fits: FITS - fits so far`, `max_seeds:
  Some(CHUNK)`), with `workers` threads, or half of them (at least 1) while `PreviewBusy(true)` holds, so the preview
  keeps its frames. The results are merged: every fit is kept and the best `OTHERS` of the rest by score. `next_seed` is
  one past the last seed that call checked (if the merged `search` does not say how far it got, have it say so). After
  each chunk a `Shortlist` goes out: the rows are the candidates that pass `filters(spec).keep`, fits first by score
  downward, then by seed upward, as `story_view`; `fits_total`; `checked = next_seed - 1`; the state. The cache is saved
  after every eighth chunk and when the search ends. It ends `Full` at `FITS` fits, `Stopped` on `StopSearch`, and
  silently when a `StartSearch` for another generation arrives (Review Focus 3). `SetFilters` sends the list again from
  memory. The set of seeds checked is always 1 to `next_seed - 1`, so the shortlist is the same for any number of
  workers.
- [ ] **Step 1: Confirm the interfaces, then write the failing tests:** `story_view_maps_a_candidate` and
  `filters_map_the_spec` (unit); `the_shortlist_fills_ranked` (handicap `[2.0, 0.4]`; wait for a `Shortlist` with
  at least 3 rows: scores do not rise down the list, every row `fits`, every `t_ko` is within 49 to 53, `lead`
  starts at or after `t_fight`, the first uses are named in sentence case); `filters_are_the_engines`
  (`SetFilters { winner: Some(1) }`: every row's winner is 1; `show_outside: true`: some row has `fits == false`
  once 512 seeds are checked); `stopping_saves_and_starting_again_resumes` (after `StopSearch` the cache file
  exists; a new engine on the same folder answers `StartSearch` first with a `Shortlist` whose `checked` is the
  saved one, not 0); `a_cache_from_another_version_is_ignored` (write the cache under another version string: the
  first `Shortlist` has `checked == 0` or comes after a fresh chunk); `a_new_fight_silences_the_old_search`
  (`StartSearch { gen: 1 }`, then `StartSearch { gen: 2 }` with the sides swapped: after the first event of
  generation 2, none of generation 1 arrives within 3 s); `the_shortlist_does_not_depend_on_the_workers` (two
  engines with 1 and 8 workers, stopped at `checked == 1024` by a test hook that ends the loop there: the same
  seeds in the same order).
- [ ] **Step 2:** Run `cargo test -p bf-app --release --lib engine::search`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS. Note the fights per second for `m2-measurements.md`.
- [ ] **Step 5:** Commit `bf-app: search, filters and the shortlist cache through the seam`.

### Task 31: Takes: the fight, its picture, its sound

**Files:**
- Create: `src/engine/takes.rs`
- Modify: `src/engine/mod.rs` (route `LoadTake`)

**Interfaces:**
- Consumes: `bf_sim::{setup_from, run(setup: &Setup, record: bool) -> Recording}`; `bf_core::show::{Show,
  ShowError::NoKo, Show::new(rec: &Recording, ff: &FightFile) -> Result<Show, ShowError>}`;
  `bf_story::{BeatSheet::series, story(rec: &Recording, grid: &Grid, sheet: &BeatSheet, low_names: [&str; 2]) -> Story,
  score(st: &Story, sheet: &BeatSheet) -> Verdict}`; `bf_content::{build_picture(rec: Arc<Recording>, ff: &FightFile) ->
  Result<Picture, ContentError>, build_soundtrack(rec: &Recording, ff: &FightFile) -> Result<Soundtrack,
  ContentError>}`; `Picture::frame(&self, f: usize) -> Rgb8` (360 x 640, `Img<u8>` with `data`), `Picture::nf()`;
  `Soundtrack { master: Vec<[f64; 2]>, stems: BTreeMap<String, Vec<[f64; 2]>> }`; `bf_export::checks::{CheckResult {
  name: &'static str, pass: bool, summary: String, numbers }, levels(master: &[[f64; 2]]) -> CheckResult,
  loudness(master: &[[f64; 2]], show: &Show) -> CheckResult, band(st: &Soundtrack, show: &Show, sounds: [(&str, (f64,
  f64)); 2], bed_buses: &[&str]) -> CheckResult}` (call `band` the way `bf_export::checks::pre_render` does); `Grid {
  t_fight, beat, t_reassemble, .. }`; `engine::search::story_view`.
- Produces:
  ```rust
  pub struct PictureSource(pub Arc<Picture>);                  // impl model::take::FrameSource
  pub fn sounding(stem: &[[f64; 2]], beat_samples: usize) -> Vec<(f32, f32)>;   // seconds; beats louder than -50 dBFS (RMS), neighbours joined
  pub fn lanes(stems: &BTreeMap<String, Vec<[f64; 2]>>, names: [&str; 2], colors: [[u8; 3]; 2], beat: f64) -> Vec<Lane>;
  pub fn markers(story: &Story, show: &Show) -> Vec<(f32, String)>;
  pub fn check_lines(results: &[CheckResult]) -> Vec<CheckLine>;
  ```
- **The rules.** `LoadTake`: the newest request wins. In order: `fight_file`, simulate the draft's seed with
  recording on, `Show::new` (`NoKo` ends it with `TakeFailed { problem: "Seed {n} has no K.O. before 58 s, so it
  cannot be made into a video. Pick another seed." }`; any other error ends it with `plain(..)`), story and
  verdict, `build_picture`, then `TakePicture { story: story_view(..), frames: Arc::new(PictureSource(..)) }`; then
  `build_soundtrack`, the three checks, and `TakeSound { audio, music }` with the master as `f32`. Before each
  stage, if a newer `LoadTake` has arrived, this one stops and sends nothing more (a soundtrack being built cannot
  be interrupted; it is finished and thrown away). The last three takes are kept, keyed by the fight file's text,
  so going back to a seed answers at once. A lane per stem: `Fighters` (the fighter's colour, its name as shown)
  when the stem's name is one of the two fighters' names, compared without case; else `Band`, named in sentence
  case; `clips` from `sounding` with `beat_samples = round(beat * 48_000)`. Markers: (0, "Title card"), (`t_fight`,
  "Fight"), each time in `story.lc_t` "Lead change", (`t_ko`, "K.O."), (`t_reassemble`, "Reset"). Check lines:
  "Loudness" for `levels`, "Perceived loudness" for `loudness`, "Fighters above the music" for `band`; the note is
  the result's `summary` (§8: a failing take is seen on Music, not in the queue).
- [ ] **Step 1: Confirm the interfaces, then write the failing tests:** unit: `sounding_joins_neighbouring_beats`
  (a stem of 16 beats of 100 samples, silent but for a 0.1 sine over beats 4 to 7: one stretch from `400 / 48_000`
  s to `800 / 48_000` s); `markers_follow_the_story`; `a_fighters_stem_is_a_fighter_lane`. Through `spawn` (pick
  the seed with a short search first): `a_take_arrives_picture_first` (`TakePicture` then `TakeSound` for the same
  take; `frames.frames() == 1_830`; `frames.draw(0)` is 360 x 640 with 691,200 bytes; `audio.samples.len() ==
  2_928_640`; three check lines; a lane for each fighter); `frames_are_the_engines_own` (`frames.draw(500).rgb`
  equals `build_picture(..).frame(500).data`); `a_seed_with_no_ko_fails_in_words` (handicap `[0.05, 0.05]`, the
  first seed whose fight has no winner: `TakeFailed` with the sentence above); Review Focus 3
  `the_newest_take_wins` (two `LoadTake` in a row for different seeds: once any event of the second arrives, none
  of the first arrives within 3 s); `going_back_is_instant` (asking again for the first seed gives both events
  within 1 s).
- [ ] **Step 2:** Run `cargo test -p bf-app --release --lib engine::takes`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS. Note the time from `LoadTake` to `TakePicture` and to `TakeSound` for
  `m2-measurements.md`.
- [ ] **Step 5:** Commit `bf-app: takes through the seam`.

### Task 32: Preview speed against the budget

**Files:**
- Create: `docs/studio/m2-measurements.md`, `src/preview/bench.rs` (declared in `src/preview/mod.rs`)
- Modify: `src/main.rs` (`--preview-bench`), `src/preview/scheduler.rs` and `player.rs` only if the numbers ask for it

**Interfaces:**
- Consumes: `docs/studio/m1-measurements.md` (`frame_ms_p95`, `soundtrack_build_s`); `Player`, `Scheduler::frame_ms`,
  `pace`, `PlayerStats`, `NoAudio`; `engine::takes::PictureSource`; the budget table at the top of this plan.
- Produces:
  ```rust
  pub struct BenchReport { pub frame_ms: f32, pub threads: usize, pub lead: u32, pub stride: u32, pub shown: u64,
      pub late: u64, pub on_time: f32, pub scrub_median_ms: f32, pub scrub_worst_ms: f32 }
  pub fn run(frames: Arc<dyn FrameSource>, start: u32, seconds: f32, threads: usize) -> BenchReport;
  ```
  `ballfight-studio --preview-bench [--seed N] [--seconds 20] [--threads T]` builds the take of the starter fight at
  handicap `[2.0, 0.4]` (the first fitting seed unless one is given), runs `run` with no window and no sound (the
  wall clock, `tick()` every 4 ms, from FIGHT!), then asks for 50 random frames one after another through `ScrubTo`
  and times each until `tick()` gives it, and prints the report as one line per field.
- [ ] **Step 1: Write the failing tests:** `the_bench_counts_frames` (unit, on the test source of Task 15, 2 s, 2
  threads: `shown + late` is between 55 and 61 and `on_time == shown / (shown + late)`); and, marked `#[ignore]`
  and run by name in release, `playback_meets_the_budget` (the real take, 20 s, `threads = workers`: when `P =
  ceil(1.25 * frame_ms / 33.3)` is at most `workers`, `on_time >= 0.99` and `stride == 1`; when it is more,
  `stride == pace(frame_ms, workers).1` and the run still ends on time, which shows the clock does not wait for
  frames) and `scrubbing_meets_the_budget` (`scrub_worst_ms <= frame_ms_p95 + 20.0`, with `frame_ms_p95` read from
  this run's own draw times).
- [ ] **Step 2:** Run `cargo test -p bf-app --release --lib preview::bench`. Expected: FAIL.
- [ ] **Step 3:** Implement. Run `ballfight-studio --preview-bench` three times on this PC with nothing else
  running, and once more while `ballfight search` runs beside it.
- [ ] **Step 4:** Run the two ignored tests by name: PASS. Write `m2-measurements.md`: the reports beside the
  budget table, `frame_ms_p95` from milestone 1 beside the one measured here, `P`, and the time to first picture
  and to sound from Task 31. If `P > workers`, name the stages milestone 1's "Speed plan" lists and state that the
  preview draws every second frame until they are done; change nothing that alters a frame's bytes.
- [ ] **Step 5:** Commit `bf-app: preview bench and measurements`.

### Task 33: Render jobs through the seam

**Files:**
- Create: `src/engine/jobs.rs`
- Modify: `src/engine/mod.rs` (route `Enqueue`, `RunJob`, `CancelJob`, `CheckTools`)

**Interfaces:**
- Consumes: `bf_export::{find_ffmpeg() -> Result<Ffmpeg, ExportError>, JobOpts { out_root: PathBuf, workers: usize,
  frames: Option<Range<usize>> }, JobEvent { Stage(&'static str), Progress { done: usize, total: usize },
  Check(CheckResult), Done(PathBuf) }, run_job(ff: &FightFile, rec: Arc<Recording>, opts: &JobOpts, on: &(dyn
  Fn(JobEvent) + Sync), cancel: &AtomicBool) -> Result<PathBuf, ExportError>, ExportError}`; the `name` of every
  check in `bf_export::checks` and the stage names `run_job` sends (read them in `job.rs`); `bf_sim::{setup_from,
  run}`; `convert::{fight_file, plain}`; `model::{JobSpec, JobUpdate, StepKey, StepState, FailHint}`.
- Produces:
  ```rust
  pub fn step_of_check(name: &str) -> Option<StepKey>;    // levels and loudness -> Loudness; band -> Band; flash -> Flashes; ...
  pub fn step_of_stage(stage: &str) -> Option<StepKey>;   // the soundtrack, frame, encode and verify stages
  pub fn check_title(key: StepKey) -> &'static str;       // "the safe zone check", "the focus check", "the loudness check",
      // "the fighters-above-the-music check", "the flash check", "the loop check", "the check of the finished video"
  pub fn failure(error: &ExportError, failed: Option<&CheckResult>) -> JobUpdate;
  ```
- **The rules.** `Enqueue`: `fight_file(draft)`, which must have a seed; `Enqueued { spec }` with the title
  (`wording::versus`), the two colours, the theme as shown, the seed, `fight_toml: ff.to_toml()` and the root; an
  error gives `EnqueueFailed`. `RunJob`: parse the text, simulate with recording on, `run_job` with the config's
  `clip` as `frames`. Its events become updates: a stage makes its step `Now` and the step before it `Done`; a
  check makes its step `Done` or `Failed` with the check's `summary` as the note (Loudness is done when both
  `levels` and `loudness` passed, and carries the `levels` summary); `Progress` becomes `Frames`, ten a second at
  most; success makes every step `Done` and sends `Done { folder, seconds }`. `failure`: a failed check gives the
  headline "Stopped at {check_title}" and the detail "{summary} Nothing was saved. {advice}", with, for Safe zone,
  Focus, Flashes and Loop point, "Try another seed, then queue it again." and `OpenInSeeds`; for Loudness and
  Band, "Open it in Music to see the levels, or try another seed, then queue it again." and `OpenInMusic`; for
  Verify, "Queue it again; if it stops here twice, try another seed." and `None`. ffmpeg not found: "ffmpeg was
  not found", "Videos cannot be saved without it. Install ffmpeg, start the app again, then queue it again.",
  `InstallFfmpeg`. The folder missing or not writable: "The output folder cannot be used", "{the error's text}
  Nothing was saved. Choose another folder, then queue it again.", `CheckFolder`. Anything else: "The video could
  not be made", "{the error's text} Nothing was saved.", `None`. `CancelJob` sets the flag and the job answers
  `Cancelled`. `CheckTools` answers `Tools` with ffmpeg's version line ("ffmpeg 9.0.2") or "ffmpeg was not found on
  this PC."
- [ ] **Step 1: Confirm the interfaces, then write the failing tests** (those that encode print a notice and pass
  when ffmpeg is absent; those that set `BALLFIGHT_FFMPEG` share one lock, since the variable belongs to the whole
  process; `clip` is `Some(0..90)`): `every_check_has_a_step` (for each name in `bf_export::checks`,
  `step_of_check` is `Some`); Review Focus 4 `a_failed_check_is_named_with_its_numbers` (unit: for a failed
  `levels` result whose summary is "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB.",
  `failure` gives the headline "Stopped at the loudness check", a detail that starts with that summary and holds
  "Nothing was saved.", and `OpenInMusic`); `a_queued_fight_renders_to_a_folder` (`Enqueue` gives `Enqueued`;
  `RunJob` gives steps in `STEP_ORDER`'s order, at least one `Frames`, then `Done`; the folder holds one `.mp4`,
  `cover.png`, `caption.txt` and one `.toml`); Review Focus 1 `a_root_that_cannot_be_written_fails_in_words` (the
  root is a path under a plain file: `Failed` with the headline "The output folder cannot be used" and
  `CheckFolder`; nothing is left behind); `missing_ffmpeg_fails_in_words` (`BALLFIGHT_FFMPEG=/nonexistent`:
  `CheckTools` gives `Err`, `RunJob` gives `Failed` with `InstallFfmpeg`); `cancel_leaves_nothing` (`CancelJob`
  after the first `Frames`: `Cancelled`, and the root holds no folder); `enqueue_needs_a_seed`.
- [ ] **Step 2:** Run `cargo test -p bf-app --release --lib engine::jobs`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: render jobs through the seam`.

### Task 34: Library, reopening and folders through the seam

**Files:**
- Create: `src/engine/files.rs`
- Modify: `src/engine/mod.rs` (route `ScanLibrary`, `OpenFight`, `OpenFightText`, `CheckFolder`, `CreateFolder`,
  `PickFolder`, `RevealFolder`)

**Interfaces:**
- Consumes: `store::library::{scan, thumbnail, folder_state, FightSummary}`; `FightFile::parse`,
  `bf_content::validate`; `convert::{draft_of, title_of, plain}`; `engine::content::roster` (colours);
  `wording::versus`; the `rfd` crate.
- Produces: the seven commands' behaviour. `ScanLibrary`: `scan(root, read)`, where `read` parses the text as a
  fight file and gives its title (`versus`), colours and seed, or the parse error as `plain` text; then `Library`;
  then, for each entry, its `cover.png` through `thumbnail` as `LibraryCover`. A missing root gives `Library` with
  no entries and the problem (Review Focus 1). `OpenFight`: read, parse, validate; `FightOpened { draft:
  draft_of(..), title: title_of(..) }`, or `Problem { text: "{file name} cannot be opened: {reason}" }`.
  `OpenFightText`: the same from text. `CheckFolder` and, after `create_dir_all`, `CreateFolder`: `Folder { state:
  folder_state(..) }`. `PickFolder`: on its own thread, `rfd::FileDialog::new().set_title("Choose the output
  folder").set_directory(start).pick_folder()`, then `FolderPicked`. `RevealFolder`: start `explorer <path>` on
  Windows and `xdg-open <path>` elsewhere, without waiting for it.
- [ ] **Step 1: Confirm the interfaces, then write the failing tests** (temp folders; a cover is a 1080 x 1920 PNG made
  with the `png` crate; a fight file is `starter(..)` with a seed, written with `to_toml`):
  `a_rendered_folder_shows_in_the_library` (one entry: "Flute vs Trombone", its seed, the two colours; then one
  `LibraryCover` of 180 x 320); Review Focus 1 `a_missing_output_folder_is_a_problem_not_a_crash`;
  `a_fight_from_a_newer_version_is_named` (`format = 2`: the entry's problem holds "this app reads fight files up to
  format 1"); `reopening_gives_the_fight_back` (`OpenFight` gives `FightOpened` whose draft has the file's fighters,
  handicap and seed and `base` set, titled "Flute vs Trombone, seed 7"); `a_broken_fight_file_is_a_problem` (a file
  naming KAZOO: `Problem` whose text holds the file's name and "KAZOO"); `a_created_folder_is_ok`;
  `a_taken_name_is_reported`. `PickFolder` and `RevealFolder` open windows and are not run by tests: keep each to a few
  lines.
- [ ] **Step 2:** Run `cargo test -p bf-app --release --lib engine::files`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-app: library, reopening and folders through the seam`.

### Task 35: The real app end to end, CI, docs, and closing the milestone

**Files:**
- Create: `tests/ui/real.rs`, `tests/snapshots/real_matchup_1440x900.png`
- Modify: `src/main.rs`, `src/app.rs` (the slowest-update counter), `studio/crates/bf-app/README.md`,
  `studio/README.md`, `.github/workflows/studio.yml`, `docs/studio/HANDOFF.md`, `docs/studio/m2-measurements.md`

**Interfaces:**
- Consumes: everything; `engine::spawn`, `EngineConfig`; `store::{Paths, Writer, load_settings, load_queue}`;
  `open_audio`.
- Produces: `ballfight-studio` with no flag runs the real engine: storage from `Paths::system()` (or `--data-dir`),
  settings and queue loaded with their notices, `engine::spawn` with the settings' workers, `open_audio()`, the
  writer. `--fixtures` stays as it is. `BfApp` gains `pub slowest_update_ms: f32`, the longest time one `logic`
  call or one action's dispatch has taken.
- [ ] **Step 1: Write the failing tests** in `tests/ui/real.rs` (release; `BfApp::new` on `engine::spawn` with a
  temp storage folder and `NoAudio`; a helper runs the harness and sleeps 20 ms until a label appears or a time
  limit passes): `the_real_match_up_screen` ("Snake charmer", "Slide lance", "C5 to A6", "E2 to F4", "2 of 20
  ready", "A live lance knocks a risen cobra flat." and "2 fighters, 1 theme" appear within 10 s; 18 chips are
  disabled; `harness.unnamed_widgets()` is empty; `harness.snapshot("real_matchup_1440x900")`); and, marked
  `#[ignore]` and run by name, `a_whole_video_with_the_real_engine` (by clicks: "Balance this match-up", wait for
  "In range"; "Search seeds", wait for a row; click row 1, wait for the caption "Seed {n}, the real picture and
  sound"; "Use seed {n}"; wait for "Loudness" lines on Music; "Go to render"; "Add to queue"; wait up to 10 minutes
  for "Done in"; then the folder under the temp output root holds `flute_vs_trombone.mp4`, `cover.png`,
  `caption.txt` whose first line is the drafted caption, and a fight file that `FightFile::parse` reads with the
  chosen seed; `ffprobe` reports 61.01 s; `slowest_update_ms < 20.0` over the whole run, which is the test's bound
  for the 4 ms budget under a software renderer).
- [ ] **Step 2:** Run `cargo test -p bf-app --release --test ui real::`. Expected: FAIL.
- [ ] **Step 3:** Implement. CI: in the Ubuntu job add, before the build, `sudo apt-get update && sudo apt-get
  install -y libasound2-dev pkg-config mesa-vulkan-drivers`; in the Windows job add `cargo build --release -p
  bf-app` and `cargo test --release -p bf-app`. If the Ubuntu job reports picture differences against the
  references (blessed on Windows), add `[linux] max_failed_pixels = N` to `kittest.toml` with the smallest N that
  passes, 200 at most; more than that is a real difference to find. Docs: `studio/README.md` (the `bf-app` row, how
  to start the app), `HANDOFF.md` (the app exists, where the exe is built, `--fixtures`), `m2-measurements.md`
  (test counts, the slowest update, the run time of the whole-video test), the crate's `README.md`.
- [ ] **Step 4:** `cargo test --workspace --release`, then the ignored tests by name
  (`a_whole_video_with_the_real_engine`, the two of Task 32, `this_pc_plays_silence`): PASS. `cargo build --release -p
  bf-app`; start `studio/target/release/ballfight-studio.exe --smoke`: it opens on the real Match-up screen and closes
  by itself. `cargo fmt --check` and clippy clean for the workspace. `git status --short projects reelkit series` is
  empty.
- [ ] **Step 5:** Commit `milestone 2: the app runs on the real engine; CI, measurements and handoff notes`. The
  controller pushes and confirms both CI jobs when pushing is possible. Then write the milestone 3 plan (the kit
  roster).

---

## Decisions taken while planning

Nobody was available to ask, so these were decided here. Each says what it costs if it is wrong.

| # | Decision | Cost if wrong |
|---|---|---|
| 1 | The interface reaches the engine only through `Command` and `Event`, and a sample engine is built into the exe behind `--fixtures`. | A little code that ships unused; remove the flag. |
| 2 | `eframe`, `egui` and `egui_kittest` are pinned at 0.37.0 (the newest; they bring `wgpu` 30.0.1). Proven on this PC in a throwaway project: dev and release builds, a snapshot test on the software adapter, sound device opened, folders found. | A later egui changes these calls; the pins hold the build until someone chooses to move. |
| 3 | All egui tests are one test program and every drawing test takes one lock. Without it the test process crashed in about half of the runs. With it, one unexplained abort at exit was seen in 225 runs. | A rare failed test run that passes when run again; if it recurs, give drawing tests their own process each. |
| 4 | Reference pictures are blessed on this Windows PC (the reference platform by ruling R1) and Linux CI compares them with a small pixel allowance if it needs one. | Linux draws differently by more than the allowance: keep a second set of pictures for Linux. |
| 5 | Noto Emoji (monochrome) is built in so captions show their emoji on any PC and in tests, in grey like the rest of the interface; the PC's own fonts are only a last resort. | 2 MB in the exe; an owner who wants colour emoji in the caption box gets them by dropping the font from the families. |
| 6 | A match-up starts from a "starter" fight file in the content (for now only FLUTE vs TROMBONE: the series' own lines and solos); a draft is a thin layer of edits over a fight file. | Pairs added in milestone 3 each need a starter, or the composer of milestone 5; the seam does not change. |
| 7 | Tempo is chosen as a number of bars (24 to 42 for `big_top`) and written to the fight file already snapped, so the same fight always has the same search key. | The episode's own file (132 bpm) and an app-made one (133.77) are the same fight under two keys: one extra search. |
| 8 | Handicap sliders run 0.05 to 3.00; the value field takes up to 12.00. | Auto-balance lands above 3 and the knob sits at the end: widen the slider. |
| 9 | Opening Balance with an untouched handicap starts auto-balance by itself; otherwise it measures the handicap as it stands. | An owner who wanted to look first waits a few seconds, or presses "Stop balancing". |
| 10 | Choosing a fight on Seeds plays it from FIGHT!, looping; the title card is the same for every seed. | One click on the story to hear the title card. |
| 11 | Playback may start before the sound is built: silent on the wall clock, and the sound joins at the playhead. | If builds are quick this is never seen; if joining is audible as a jump, wait for the sound instead. |
| 12 | Space, the arrows and 1 to 5 are taken from the widgets, so Space never presses a focused button; Enter does. | A keyboard user expects Space on buttons: give the transport another key. |
| 13 | "Pause queue" lets the video being rendered finish and holds the next. "Cancel" stops a video now. | An owner who wants to freeze a render midway cannot; it needs a pause flag in `bf-export`. |
| 14 | After a restart a queue with waiting videos is paused and says so; closing the app during a render asks nothing and the video starts over next time. | One click to resume; up to a minute of rendering lost on close. |
| 15 | The queue lives in the interface's state and each job carries its fight file as text; a job simulates its fight again when it runs. | None expected: fights are reproducible within one app version; a job queued by an older version may render another fight, so the queue file records the version. |
| 16 | The shortlist cache keeps seed numbers only, keyed by app version and search key, and the engine fights them again on load; 200 fits and the 300 best others are kept. | A second or so on reopening a big shortlist. |
| 17 | Filters are applied by the engine (`bf_story::Filters`), not by the interface. | A filter click costs one round trip of a few milliseconds. |
| 18 | Settings is a fourth button at the right of the toolbar; Library and Settings replace only the workspace. | The owner wants it elsewhere: move one button. |
| 19 | One text box per platform, as drawn; the first line (the caption) is shared by Instagram and TikTok because the fight file and `caption.txt` hold one caption; Shorts is three lines. | Per-platform captions would need a new field in the fight file. |
| 20 | Caption drafts follow the delivered episode's pattern: names, both emoji, "Comment the next match-up", the fighters' tags and a fixed tail per platform. | Wording to taste; one function in `bf-content`. |
| 21 | Music shows what the classic arrangement can do and draws the rest disabled with the reason; the lanes show where each part of the real soundtrack sounds. | The owner would rather hide what does not work yet: delete four controls. |
| 22 | The working fight is not saved between runs; queued and rendered fights are. | Reopening the app means balancing and picking again (the shortlist is cached): add a session file like the settings. |
| 23 | The preview uses the largest whole-number scale that fits and a plain fit otherwise; the safe-zone tint is `ground` at 60 %. | At odd window sizes the picture is small: allow fractional scales sooner. |
| 24 | The title card on Match-up and Balance is drawn without a fight, over black; the saved cover comes from the real frame, where 6 % of the arena shows through. | A faint difference between preview and cover; draw the preview from a simulated fight instead. |
| 25 | A corner card's portrait is the fighter's real sprite (radius 22, turned 30 degrees). | A different pose is wanted: two numbers. |
| 26 | The roster shows all 20 kit fighters; the 18 not built yet are disabled and say so. | Hide them until built: one filter. |
| 27 | Waiting videos are reordered by dragging or from a right-click menu. | Add visible up and down buttons. |
| 28 | The preview's sound is resampled in a simple way when the PC's device is not at 48 kHz. | Slightly dull preview sound on such devices; the rendered video is untouched. |
| 29 | The job list's "Loudness" line covers both the level check and the perceived-loudness check, as drawn. | Split it into two lines. |
| 30 | Reopening a video from the Library lands on Match-up with its seed loaded. | Land on Render instead: one line. |
| 31 | `bf-app` depends on `bf-core` from the start, for the tempo rule only. | None; the alternative was a second copy of the rule. |
| 32 | Library dates are local (the `chrono` crate) and covers are shown at 180 x 320. | A dependency that could be replaced by 20 lines if it ever troubles the build. |

## Spec coverage

| Spec requirement for milestone 2 | Task |
|---|---|
| §4 five steps left to right, any revisited; Library as a sixth view | 17, 19 to 24 |
| §4 Match-up: two fighters, sides, theme, tempo snapped by `loop_bpm`, pair rules read-only | 8 (`Draft::tempo_bpm`), 9, 19, 26, 28 |
| §4 Balance: auto-balance, win split with the 40 and 60 marks, K.O. timing, share that fits, two sliders only | 9, 20, 27, 29 |
| §4 Seeds: background search, ranked shortlist, filters, click to play with sound and scrubbing, watch any seed | 9, 15, 16, 18, 21, 30, 31 |
| §4 Music: theme, key, chord loop, who plays bass, music seed and re-roll (live or stated placeholder) | 22, 31, the table "What is live and what waits" |
| §4 Render: caption per platform, output folder, add to queue | 12, 23, 33 |
| §4 what feeds the fight; the search key; a fighter whose notes are its weapon | 9, 28, 30 |
| §4 the fight file as the whole recipe | 26 (starter), 28 (`fight_file`, `draft_of`), 33 |
| §4 Library: covers, match-up, date, reopen | 12, 13, 24, 34 |
| §4 Settings: output folder (default `Videos\BALLFIGHT`), worker threads (default cores minus one) | 10, 12, 24, 34 |
| §5 `bf-app` on eframe, egui and wgpu; the interface never blocks; work on threads with progress through channels; one core kept for the preview | 1, 8, 17, 28 to 34, 35 (`slowest_update_ms`) |
| §6 search runs until stopped or 200 fits; results cached per search key | 13, 30 |
| §6 filters, and the switch for fights outside the beat sheet, marked as such | 21, 30 |
| §7 live preview: drawn ahead on worker threads into a small buffer; nearest filtering at a whole-number scale; sound is the clock; scrubbing on demand; safe-zone tint | 14, 15, 16, 18, 31 |
| §7 speed target: 30 fps with headroom | the budget table, 15, 32 |
| §8 loudness checked on the Music step | 22, 31 |
| §9 a job's stages and checks shown; a failed check named with its numbers; the queue continues | 11, 12, 23, 33 |
| §9 queue: one at a time, reorder, pause, cancel, remove, survives a restart | 11, 12, 23, 25 |
| §9 output folder and the `_seed<N>` name | 13 (`folder_state`), 23, 28 (`folder_names_agree`), 33 |
| §9 captions drafted from emoji and tags plus the series line, then editable | 9, 23, 26 |
| §10 the app reads the roster and theme list from the registry; GUITAR and SAX never appear | 8, 25 (`no_retired_fighter_anywhere`), 26, 28 |
| §11 the design system, fonts built in, 26 px controls, 2 px corners, 1 px dividers, inverted fill | 2, 3, 4, 5 |
| §11 colour belongs to fighters; errors in words; 22 % fills with a full-colour edge | 3, 6, 20, 22, 23 |
| §11 touch 1: names in the 5 x 7 pixel font at whole-number scales | 6, 8, 17, 19, 28 |
| §11 touch 2: the VS title card on Match-up and Balance | 9, 18, 27, 28 |
| §11 touch 3: the lead ribbon in rows and as the scrub bar with ability lanes | 7, 18, 21 |
| §11 layout: 1440 x 900 reference, 1280 x 800 minimum, the five heights and widths | 1, 3, 17, 18, 19 to 25 |
| §11 screens as drawn | 19 to 24, the interface reference |
| §11 wording; empty and failed states say what to do next | 19 to 25 (`wording.rs`) |
| §11 keys: Space, arrows, 1 to 5 | 17, 18, 23 |
| §13 unit tests written first; interface snapshot tests of each screen against the mock-ups on a software adapter | every task; 1, 4 to 7, 17 to 25, 35 |
| §13 CI: Linux runs everything; Windows builds the app and runs the tests | 35 |
| §14 milestone 2: five steps, preview, shortlist, queue, Library, settings, on the one match-up | 25 (on sample data), 35 (on the real engine) |
| M1 plan's leftovers: queue persistence and reordering, Library, shortlist cache, caption drafting, live preview and sound | 11, 12, 13, 14 to 16, 24, 26, 30, 31 |
| Update button and ffmpeg: a place now, their function in milestone 6 | 17, 23, 24, 33 |

Not in this plan, by design: more fighters and themes (milestones 3 and 4); the composer, personalities, layered
arrangements and the full Music screen (milestone 5); ffmpeg download, the Update button's function, the user guide
and an installer (milestone 6); saving the working fight between runs (decision 22).
