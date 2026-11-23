# BALLFIGHT Studio

A native Rust engine for BALLFIGHT episodes: the fight, the picture, the sound and the export, ported from the Python
kit in `projects/ballfight/` and checked against it. Design: `docs/studio/2026-11-10-ballfight-studio-design.md`.

**Rust is now the source of truth for BALLFIGHT. The Python kit in `projects/ballfight/` is a frozen archive: it is
kept so old episodes replay, and no new work goes there.** The kit's README is left as it is; `docs/studio/HANDOFF.md`
gives the one line the owner may ask a session to add to it.

## Crates

| Crate | Job | Depends on |
|---|---|---|
| `bf-core` | Shared types, the bar and beat grid, deterministic maths (`dmath`), the numpy-compatible random generator | nothing |
| `bf-sim` | The fight engine | core |
| `bf-story` | Story analysis, scoring, seed search, balance, filters | core, sim |
| `bf-picture` | The renderer | core |
| `bf-sound` | Synthesis and mixing | core |
| `bf-content` | Fighters, themes, the registry and the pair rules | sim, picture, sound |
| `bf-export` | Checks, encode, render job, output folders | story, picture, sound, content |
| `bf-cli` | The `ballfight` command line | all |
| `bf-pack` | Packaging: the pinned ffmpeg and its checked download, the package folder, the install and its shortcuts, removal, `update.toml` and version numbers; the `ballfight-pack` command | nothing |
| `bf-app` | BALLFIGHT Studio, the desktop app (egui): the five steps, the live preview with sound, the shortlist, the render queue, the Library and Settings, on the real engine (`--fixtures` runs it on sample data), its start-up checks, fault reports and updates | core, story, sim, picture, sound, content, export, pack |

## Building and testing

Run everything from this folder (`studio/`). The toolchain is pinned in `rust-toolchain.toml`.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Running the tests

- `cargo test --workspace` runs every crate's unit tests and the parity tests. The parity tests read `reference/`
  through the dev-only crate `bf-testdata` and need nothing else: no Python, no network. `cargo test -p <crate>` runs
  one crate, `cargo test -p bf-core --test rng_vectors` one test file. CI
  (`.github/workflows/studio.yml`, one Windows job) builds `bf-cli`, tests `bf-core`, `bf-sim` and `bf-pack`, fetches
  the app's own pinned ffmpeg with `ballfight-pack fetch-ffmpeg` (the archive named in `packaging/ffmpeg.toml`, about
  258 MB, downloaded only when the runner's cache does not hold it; every checksum is checked on each run), checks
  that it has the `libx264` and `aac` encoders, runs the command line's `render_is_reproducible` and, with
  `BF_NEED_PINNED_FFMPEG=1`, `the_encode_is_pinned` and `the_faster_clip_is_pinned` (the 3 s clip with each encoder
  setting must be byte for byte its line in `replay/encode_hashes.txt` for the number of threads x264 used there),
  renders a 3 s clip of the example episode,
  builds the app (`bf-app`) and runs the app's
  tests that need no window: `cargo test --release -p bf-app --lib --bins --test compat` (the unit tests inside each
  file, the real engine's among them, the command line's and the saved-file tests). The app's interface tests
  (`--test ui`) are not run on CI: they draw through DirectX 12 only, and a hosted runner is not known to offer a
  DirectX 12 adapter. They run on the owner's PC, with the ignored ones below. The app's two steps add about 11 minutes or more to
  the job: the first run on GitHub took 6 min 44 s to build the app, and its tests ran 4 min 23 s before they failed.
  Last, it makes the package with `ballfight-pack package` in the runner's temp folder and checks every file in it
  against its `MANIFEST.txt`, the exe against the build and the ffmpeg files against the pin; the package (about
  600 MB) is not uploaded anywhere.
  When the app's tests fail, a step after them repeats the failed tests, the panics, compile errors and the last 30
  lines of their output as annotations on the run, so the failure can be read without opening the log; a second such
  step does the same for the `bf-core`/`bf-sim`/`bf-pack` tests, the ffmpeg fetch, the pinned encode and the package,
  titled with the failed step's id. A test that
  needs something a hosted runner may lack says "not run here: <reason>" in its output instead of passing quietly
  (today: the render tests when no usable ffmpeg is found, and the two `store::library` tests of a folder that
  cannot be read, `a_folder_that_cannot_be_read_is_named` and `an_output_folder_that_cannot_be_read_is_named`, when
  the account can still list a folder everyone was denied, as an administrator account may; on the owner's PC
  they run fully).
- Ignored tests, run by name on the owner's PC (each needs minutes, ffmpeg, a sound device or a quiet PC):
  `cargo test -p bf-app --release --test ui real::a_whole_video_with_the_real_engine -- --ignored` (a whole video,
  by clicks, on the real engine), `cargo test -p bf-app --release --lib preview:: -- --ignored` (the preview's
  budgets, `playback_meets_the_budget` and `scrubbing_meets_the_budget`, and `this_pc_plays_silence`, which opens the
  sound device).
- `cargo test -p bf-testdata --test reads_reference -- --nocapture` opens every `.npz` under `reference/` with its
  sidecar and lists what it found: the quick check that a newly exported vector file is readable from Rust.
- Before a commit, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean and the
  test output has no warnings.
- The Python side checks itself: every export reads its files back and checks them before it lists them in the
  manifest (the sound exports replay the recorded noise states and compare every note), and
  `python -m tools.studio_export verify` re-hashes the whole set. `python tools/ballfight_replay_hash.py` is the
  kit's own replay guard: its output must not change, because nothing in `projects/`, `reelkit/` or `series/` is
  ever edited from here.

## The command line

`ballfight` (crate `bf-cli`; `cargo build --release -p bf-cli` makes `target/release/ballfight.exe`) runs every step
of a video without the app. Every command takes a fight file; `--rec <file.bfrec>` uses a recording instead of
fighting the seed again, `--seed N` fights another seed than the file's, `--workers N` sets the threads of search,
balance, check, render and bench (default: all cores but one; 1 to 256) and `--json` prints one JSON value on stdout.
Progress goes to stderr.

```
ballfight sim     fights/flute_vs_trombone.toml --seed 7 --out fight7.bfrec
ballfight story   fights/flute_vs_trombone.toml [--seed N | --rec F]
ballfight search  fights/flute_vs_trombone.toml [--from 1] [--stop 200] [--max N] [--out shortlist.json]
ballfight balance fights/flute_vs_trombone.toml [--write]     # --write stores only the handicap in the file
ballfight frame   fights/flute_vs_trombone.toml --frame 500 --out f500.png [--up] [--rec F]
ballfight sound   fights/flute_vs_trombone.toml --out audio.wav [--stems DIR] [--rec F]
ballfight check   fights/flute_vs_trombone.toml [--rec F]
ballfight render  fights/flute_vs_trombone.toml --out <folder> [--seed N] [--rec F] [--frames A:B]
ballfight hash    fights/flute_vs_trombone.toml --seeds 1,2,3
ballfight bench   fights/flute_vs_trombone.toml [--out <folder>] [--only fights,frame,stages,draw,sound,render]
```

- `bench` measures this PC: fights a second (record off, seeds 1 to 2,000, all workers and one), one frame on one
  thread (every 10th frame: mean, 95th percentile, slowest), the frame by stage, all 1,830 frames on all workers
  without the encode, and the soundtrack build; with `--out` also a whole render (with the encode) into a new
  folder there, by stage. Each part says how busy the PC was the second before it. Without `--out` it writes
  nothing. `--seeds`, `--every` and `--stage-every` make it quicker. Milestone 1's numbers are in
  `docs/studio/m1-measurements.md`.

- Exit code 0 on success; 1 on bad input, a fight it refuses (for example "seed N has no K.O. before 58 s"), a
  failed check or a run stopped with Ctrl+C, with the reason as one line on stderr (`ballfight: ...`).
- Ctrl+C stops a command (exit 1): `render` removes its temporary folder, `search` prints what it found, `balance`
  and `check` stop within about a second; `sim`, `story`, `frame` and `sound` stop between
  their steps (the soundtrack build itself, about 5 s, always runs to its end). A second Ctrl+C ends the program
  at once.
- Files are never replaced: a command refuses an output path that exists, and writes each file whole through a
  temporary file beside it. `render` goes through the render job (`bf_export::run_job`): a new folder
  `<a>_vs_<b>` (or `<a>_vs_<b>_seed<N>`) inside `--out` with the mp4, `cover.png`, `caption.txt` and the fight file;
  a clip (`--frames A:B`) is `<a>_vs_<b>_frames<A>-<B>` holding only its mp4.
- `crates/bf-cli/tests/e2e.rs` runs the program end to end. `render_is_reproducible` compares the sound and three
  frames of the example episode with `replay/render_hashes.txt` (`BF_BLESS=1` rewrites it). The two whole renders
  are ignored by default: `cargo test --release -p bf-cli --test e2e -- --ignored --nocapture`.

## The app

`bf-app` is BALLFIGHT Studio itself. `cargo build --release -p bf-app` makes `target/release/ballfight-studio.exe`;
double-click it (or start it with no option) for the real app on the real engine: pick the match-up, balance it,
search seeds, choose one with its picture and sound, then queue the video. Videos go to `Videos\BALLFIGHT` unless
Settings says otherwise. Today (1.8.2) it offers 42 fighters and 62 themes, and every one of the 1,722 pairings
starts: the 20 kit fighters and 14 kit themes are exact against the Python kit, the 10 older fighters (ACCORDION,
BAGPIPES, BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE, VIOLA, VIOLIN) and 12 older themes are ported close
enough for the eye and the ear (the owner's decision of Nov 14), each older fighter starting on its own home theme,
and 12 fighters and 36 themes were made in the Studio itself (Nov 16 to 21). A pair is
offered only the themes whose music has every hit its fighters' parts play (no Dark disco or Heist for a pair with
the TROMBONE, ORGAN, TUBA, EUPHONIUM or HORN, which play a big drum hit those two lack). The seed finder balances an
uneven pairing (Auto-balance). New pairings start on the Composed music (the story-driven arrangement with the
composer's lines and solos); the series' own match-ups and imported episodes keep Classic, and the Music screen
switches between them. Known limits: `docs/studio/HANDOFF.md`. The owner double-clicks the installed copy (see
Packaging below); a build in `target\` is a build session's own copy.
`--fixtures` runs the same window on sample data, `--data-dir <folder>` keeps its settings, queue and cache in
another folder, and `--smoke` opens it, paints 30 frames and closes it. Details: `crates/bf-app/README.md`.

## Packaging

The owner's copy is installed by `ballfight-pack` (crate `bf-pack`; details in `crates/bf-pack/README.md`).

- **The package** is the program folder as installed: `ballfight-studio.exe`, `ballfight.ico`, `ffmpeg\` (the pinned
  ffmpeg with its licence and `ABOUT.txt`), `guide\` (from `packaging/guide`) and `MANIFEST.txt` (every file's size
  and SHA-256). `ballfight-pack package --exe <ballfight-studio.exe> --out <new folder>` makes one;
  `ballfight-pack install --exe <ballfight-studio.exe> --from <the owner's clone>` installs it into
  `%LOCALAPPDATA%\Programs\BALLFIGHT Studio` with a desktop and a Start menu shortcut (no installer, no admin rights,
  nothing in the registry).
- **The pin.** `packaging/ffmpeg.toml` names the ffmpeg archive (address, size, SHA-256) and each file taken from it.
  `ballfight-pack fetch-ffmpeg --cache <folder> --out <folder>` downloads and checks it. Nothing downloaded is ever
  committed.
- **The release job.** CI (`.github/workflows/studio.yml`) makes the package on every push and checks every file
  against its manifest and the pin. It uploads nothing: with the update route in use, the owner's PC builds its own.
- **The tags.** A version reaches the owner only through a tag `studio-v<version>` (or `studio-v<version>-rc.N`)
  pushed on a green commit whose `[workspace.package] version` is that version. The steps are in
  `docs/studio/HANDOFF.md` ("How a version reaches the owner").

**The update route in use: A, build on this PC** (decision D1 of
`docs/studio/2026-11-11-ballfight-studio-plan-m6-packaging.md`). The Update button fetches the newest release tag into
the app's own copy of the repo (`%LOCALAPPDATA%\BALLFIGHT Studio\data\source`), builds it with `cargo build --release
--locked -p bf-app` at below-normal priority, checks that the new exe names the tag's version and commit, and swaps
it in on "Restart now". The app never handles a credential: git signs in by itself.

**Switching to route B** (build in the cloud, download): add the download source (`bf-app/src/update/download.rs`,
the packaging plan's Task B1), a release job that attaches the package and its `SHA256SUMS.txt` to a GitHub Release
for each tag, and `route = "download"` in `update.toml`. Everything after "a new exe exists" (staging, the check,
the swap, "Go back") stays as it is.

## Determinism

Results must be bit-identical on every PC: fights, frames and sound. The encoded mp4 is the one exception: x264
picks its thread count from the PC's logical processors and its bytes depend on it, so the same fight file and seed
give the same mp4 on the same PC with the same ffmpeg, and on another PC the same frames and sound before encoding
(`docs/studio/m6-measurements.md` section 4). The engine crates use `f64` in the Python operation order, never fused
multiply-add, and call every transcendental through `bf_core::dmath`; `crates/bf-core/tests/no_std_maths.rs` scans
the sources and fails on any direct call such as `x.sin()` or `x.mul_add(a, b)`. No result may depend on hash-map
order or on the number of threads.

## Reference data

`reference/` holds what the Rust code is checked against: vectors, recordings, frames, stems and check numbers
exported from the Python kit by `tools/studio_export/` (and a few files the Rust tests bless themselves). Each file
is listed in `reference/manifest.json` with its SHA-256, the commit, and the Python and library versions that made
it; `tools/studio_export/README.md` describes every file. A vector file is an `.npz` of plain numeric arrays with a
sidecar `.json` of the same name that lists its cases (`id`, `call`, `args`, `arrays`); numbers that must be matched
bit for bit are `float.hex()` strings.

### Regenerating the reference set

The reference numbers are whatever the kit gives on the reference PC (the manifest's `platform` field) with CPython
3.13.18, numpy 2.5.3, scipy 1.18.1 and pillow 12.3.0; `tools/studio_export/README.md` says how that environment is
made. From the repo root, with that Python (PowerShell shown; in Git Bash use `export NAME=value`):

```
$env:PYTHONHASHSEED = "0"
$env:PYTHONPATH = (Get-Location).Path
.studio-tools\py313\Scripts\python.exe -m tools.studio_export list       # the exports, in the order `all` runs them
.studio-tools\py313\Scripts\python.exe -m tools.studio_export all        # every export
.studio-tools\py313\Scripts\python.exe -m tools.studio_export dsp voices stems checks    # or some, in this order
.studio-tools\py313\Scripts\python.exe -m tools.studio_export verify     # re-hash every file in the manifest
git status --short studio/reference projects reelkit series
```

- An export reads the kit and never writes to it, so `git status --short projects reelkit series` stays empty. The
  kit's own caches (run pickles, sprites, `audio.wav`, stem wavs) land in `reference/.cache/`, which is git-ignored
  and can be deleted at any time.
- Running an export again on an unchanged kit gives byte-identical files: `git status --short studio/reference`
  shows nothing. A file that does change means the kit, a library or the exporter changed. Read the difference
  before committing it, and expect the Rust parity test that reads the file to need the same change.
- The sound exports (`dsp`, `voices`, `stems`, `checks`) each run in an interpreter of their own, because the Pixel
  Pitch voices share one noise generator that every noisy note advances. `stems` reads what `voices` wrote and
  `checks` compares its soundtrack with the `audio.wav` that `stems` wrote: keep that order (`all` does). On the
  reference PC they take about 15 s, 12 s, 45 s and 3 minutes (`checks` draws 673 frames in 5 worker
  processes).
- `verify` only reads files and runs on any Python. It exits 1 if a listed file is missing or changed, and prints
  a note for a file under `reference/` that the manifest does not list.
- A reference file written by a Rust test (for example `reference/dmath/bits.json`) is listed with
  `python -m tools.studio_export adopt <path>` after it is blessed.
- The set stays under 150 MB (`du -sh studio/reference`, with `.cache/` deleted or left out). Past that it moves to
  Git LFS (design, section 12).

### The export and porting commands that still matter

Since the owner's decision of Nov 14 (ruling M4-R16), a new fighter or theme is ported for the eye and the ear: its
fights run without errors, a few rendered frames and a short clip look and sound right, and the app offers it. No
new kit-parity gate or `origin` gate is made for it. These commands keep the existing reference set honest (from
the repo root, with the Python above; `tools/studio_export/README.md` has the details):

```
.studio-tools\py313\Scripts\python.exe -m tools.studio_export budget                   # sizes against the budgets (under 147 MB in all)
.studio-tools\py313\Scripts\python.exe -m tools.studio_export verify                   # re-hash every file in the manifest
.studio-tools\py313\Scripts\python.exe -m tools.studio_export.unit_checks.m3refs_checks   # every folder still reproduces its inputs
.studio-tools\py313\Scripts\python.exe -m tools.studio_export remake fighters/flute    # re-make one kit folder whole, when the kit side changes
.studio-tools\py313\Scripts\python.exe -m tools.studio_export cache all                # re-make the pinned cache (5 to 15 minutes a fighter)
```

A quick check of a new port from `studio/`: `ballfight search <fight file> --stop 10`, then
`ballfight render <fight file> --out <folder> --seed N` with one of its fits.
