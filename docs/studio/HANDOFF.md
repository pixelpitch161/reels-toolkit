# BALLFIGHT Studio: handoff brief (updated Nov 24)

## Status (Nov 24, version 1.9.0): the packaged Studio, 42 fighters, 62 themes, the music system, the bold effects

**Rust is now the source of truth for BALLFIGHT. The Python kit in `projects/ballfight/` is a frozen archive: it is
kept so old episodes replay, and no new work goes there.** Its README is not edited by this build (nothing under
`projects/` is). If the owner asks, a session may add this one line to the top of `projects/ballfight/README.md`, and
nothing else:

> This kit is a frozen archive: BALLFIGHT is now made with BALLFIGHT Studio (`studio/`), and no new work goes here.

**What the owner has.** BALLFIGHT Studio, a Windows program installed in a plain folder with a desktop and a Start
menu shortcut (no installer, no admin prompt). It opens on its five steps (Match-up, Balance, Seeds, Music, Render),
with a live preview with sound, the shortlist of seeds, the render queue, the Library of finished videos and
Settings. Everything it shows and saves is made by the real engine. It comes with its own ffmpeg and a one-page
guide (`guide\index.html` in the program folder, written with the app's own pictures). It updates itself: the
Update button in the toolbar, then "Build version X", then "Restart now", and "Go back" to the version before.
Removal is a button in Settings; it keeps settings, the queue, logs and every video.

**What it can make.** Any of the 1,722 pairings (42 fighters, each against each of the other 41, either side), on any
theme the pair is offered: pick the two fighters, balance, pick a seed from the shortlist with its picture and
sound, choose the music, and render a checked 61 s video with its cover and caption into `Videos\BALLFIGHT`.

**The roster** (counted from the registry: `bf_content::registry::PORTED` holds 42 names,
`bf_content::themes::PORTED_THEMES` 62).

- **The 20 kit fighters are exact** against the Python kit: ACOUSTIC, ALTO SAX, BASS CLARINET, BASS DRUM, BASSOON,
  CELLO, CLARINET, CONTRABASS, ELECTRIC, EUPHONIUM, FLUTE, HORN, OBOE, ORGAN, PIANO, SOUSAPHONE, TENOR SAX, TROMBONE,
  TRUMPET, TUBA.
- **The 10 older fighters were ported "close enough for the eye test"**, as the owner decided on Nov 14: ACCORDION,
  BAGPIPES, BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE, VIOLA, VIOLIN. They fight, look and sound like
  themselves, but are not checked bit for bit against their old episodes. Each starts on its own home theme.
- **The 12 Studio fighters** were designed in the Studio itself (Nov 16 to 21), each with its own move set and VS
  card: CORNET, CONTRABASS CLARINET, MARIMBA, PICCOLO, SOPRANO SAX, MELLOPHONE, TIMPANI, CONTRABASSOON, FLUGELHORN,
  SNARE DRUM, OCTOBASS, BASS GUITAR.
- **The 14 kit themes are exact**: baroque storm, big top, bossa noir, brass trailer, dark disco, dark drumline, dark
  tango, heist, horror trap, hot jazz, mambo, prog duel, second line, stadium rock.
- **The 12 older themes were ported the same way, by eye and ear**: Balkan fire, brass house, dark breakbeat, drift
  phonk, gothic horror, industrial, noir jazz, orchestral drill, sea shanty, ska-punk, spaghetti western, villain
  march.
- **36 more themes were made in the Studio** (Nov 16 to 18), sorted into the theme picker's categories.
- A pair is offered only the themes whose music has every hit its fighters play. When a pairing is uneven,
  Auto-balance sets the handicap so the fight is fair. GUITAR and SAX stay retired: their roster entries are kept so
  old episodes replay, and they are never offered.

**The music system** (milestone 5, `docs/studio/2026-11-15-ballfight-studio-plan-m5-music.md`). Each fight has an
arrangement, switched on the Music screen:

- **Composed**, the default for every new pairing: the theme's layered arrangement follows the fight's story (fills
  at lead changes, a breakdown at a desperation move, everything in for the final exchange, the tape stop at the
  K.O., the victory section, the build back into frame 0), and each fighter's line and victory solo are written by
  the composer from its musical personality and the music seed. The Music screen re-rolls the music seed and can
  keep one fighter's line as it is.
  Since the sound upgrade's stage 2 (Nov 25, 2026) every theme's Composed band plays its own patterns: a `GROOVE`
  beside the theme's `TABLE` (`bf-sound/src/music/groove.rs` explains the step strings), with its swing, its bus
  levels and the buses that get a phone's overtones. **A new theme needs a groove**: `bf-harness/tests/bands.rs`
  fails a theme that plays the shared patterns or whose band loses more than 5 dB on a phone. Measure and tune it with
  `cargo run --release -p bf-harness --example bands -- <theme>` (it renders, it is not a test run).
  The Composed master keeps each fighter `polish::OVER_BAND` dB over its band by ear: 3 dB since Nov 26, 2026 (it was
  5, which Aedan heard as a band "way too quiet"; 1 dB suited some fights and not others, so the default is halfway).
  The Music screen's **Band** slider moves it for one fight: `music.band_level`, -6 to +6 dB in half dB (0, the usual
  mix, is not written), each fighter standing `OVER_BAND - band_level` over the band; `ballfight sound --band-level`
  does the same. A louder band lowers the render band check's floor by as much, so its fix never undoes the slider.
  After changing `OVER_BAND`, write the level table again (`ballfight levels fights/dev/bold_<fighter>.toml ...`,
  one per fighter, `--write crates/bf-content/src/levels_table.rs`).
  Stage 3 (S1) gives fighters a voice toward the real instrument, under Composed only (Classic keeps the ported
  tone bit for bit): the saxes first (`bari_tone_real`, the alto's and soprano's `tone_real`, `tenor_tone_real`).
  A voice that plays through `ctx.note` opts in with `Voice::reworked` and `Voice::note_real`; parts with their own
  tone functions (alto, bari, tenor) choose the real tone under `ctx.composed_on()`. Repeats of one note take turns
  among three variants (`voice::real::Repeats`). The shared pieces are in `bf-sound/src/voice/real.rs` (`levels`,
  `reshape`, `buzz`, `wobble`). From the low instruments on, a voice is reworked from the outside instead: its own
  note through an overtone `Correction` (a peak or dip at each of the first seven harmonics, `real::correct`), a
  slower start and a quicker stop (`real::Rework`, `Voice::rework`). `references -- correct "<FIGHTER>"` writes
  the table, refining it until the corrected notes match the real ones. A peak can only lift an overtone that is
  there, so overtones a voice barely has (15 dB or more under the real balance) are first added (`real::Fill`,
  `real::fill`: the note's own fundamental as an analytic signal, raised to each harmonic, so the added partials
  follow its vibrato and bends); the reworked note keeps the voice's own loudness. A note still over 5 dB off whose
  own fundamental is weak (the electric's pickup notch at 63) has the overtones it lacks against its strongest added
  too, at the real balance beside its fundamental, so a `Fill` level can be over the fundamental. A plucked or
  struck voice sets `Rework::keep_shape` (`real::follow`: its own loudness moment by moment), because the narrow
  peaks rebuild its low harmonics over tens of milliseconds and would soften its snap into a swell. Parts that make
  their own notes (piccolo, oboe, clarinet, bass clarinet, bassoon, marimba, the bagpipes' chanter, the accordion's
  reeds) keep their Classic function and add a `played` (or `ch`, `bsn`, `chord`) that reworks it under Composed
  through `real::RealNotes`. A fighter measured against a relative FluidR3
  lacks is aimed at it shaded (`references.rs` `tilt`: cornet 0.5 and flugelhorn 1 dB per harmonic mellower than the
  trumpet, mellophone 0.5 brighter than the horn), so a family keeps its voices apart; the euphonium is aimed 1.5 dB
  per harmonic brighter than FluidR3's tuba, which is nearly pure at the euphonium's pitches and left it muffled. A
  `Fill` lays overtones in only up to the eighth, so a filled voice's brightness stopped dead there (the final
  spectrogram review, Nov 26: the brass and clarinets 19 to 31 dB darker above the eighth than the real ones, with
  only breath noise left). `Rework::tail` (`real::tail`, tables from `references -- tail "<FIGHTER>"`) carries the
  trumpet, cornet, flugelhorn, horn, the three clarinets and the chanter on above it: the real 9th to 12th (beside
  the 5th to 8th together), then falling on to the 24th, adding only what a note's own harmonics lack.
  `references -- compare` prints it ("above the eighth") and `real_voices.rs` holds those voices to at most 12 dB
  darker there than the real ones. Drums (timpani, bass drum, the kit, the
  snare drum) and the synth get only the repeats' differences: a drum's overtones are not a note's harmonics, and
  the synth has no real instrument (the snare's strokes take turns through its own `played`). The turntables are
  left as they are: every scratch already follows the floor's own curve and noise. FluidR3 is silent for the tuba above 72 and
  the contrabass above 57; those notes keep the table's last row. The real instruments' fingerprints are
  `bf-harness/tests/reference/fluidr3.json`, made on the owner's PC (FluidR3 through fluidsynth, tools in
  `F:\bf-refs`, `references -- render`); `references -- profile` prints a voice's overtone table from them and
  `references -- compare "<FIGHTER>"` measures a reworked voice note by note. `tests/real_voices.rs` is S1's test:
  every note nearer its own real instrument than its rivals, and at most 6 dB from it.
- **Classic**: each theme's ported bed with the fighters' kit lines and solos. The series' own match-ups (a kit pair,
  an older episode's pair, and every imported episode) keep Classic, so they sound as they always did. Classic is
  one click away for any other fight.

**Proof that it matches the kit** (`docs/studio/m3-measurements.md`). Every kit pairing passed its pair gate (17
pairs plus milestone 1's FLUTE vs TROMBONE). Rendered from the kit's own fights, 17 of the 18 kit episodes pass all
15 checks; the 18th is TROMBONE vs HORN (see the limits below).

**How fast it is** (this PC, `docs/studio/m6-measurements.md`): the seed search runs about 6,000 to 7,000 fights a
second; a whole 61 s video takes about 82 seconds with "Best quality" (most of it is the video encoder) and less
with "Faster" (Settings).

## Where the program and its data live

The installed program (`bf-pack`, `studio/crates/bf-pack/README.md`):

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

The app's data:

```
%APPDATA%\BALLFIGHT Studio\config\     settings.toml  queue.json
%LOCALAPPDATA%\BALLFIGHT Studio\cache\  shortlists\
%LOCALAPPDATA%\BALLFIGHT Studio\data\   logs\  source\  build\  downloads\
```

`logs\` holds `start.txt` (what the last start found: version, graphics card, ffmpeg, output folder, any error) and
`crash-YYYYMMDD-HHMMSS.txt` fault reports (the newest 20). When something goes wrong, ask the owner for those files.
`source\` is the app's own copy of the repo for updates, `build\` its build folder (several GB), `downloads\` the
ffmpeg archive. Videos go to `Videos\BALLFIGHT\<a>_vs_<b>\` unless Settings says otherwise.

**How the owner starts it:** the owner double-clicks the app. Build sessions use the `ballfight` command line and
the app's options as `studio/README.md` and `studio/crates/bf-app/README.md` describe.

## How a version reaches the owner

Updates follow release tags, not a branch (route A of the packaging plan's decision D1: the app builds the new
version on this PC; no download, no token in the app).

1. Bump `[workspace.package] version` in `studio/Cargo.toml` (and let cargo update `Cargo.lock`).
2. Add the version's frozen saved files (the rule for `tests/compat/<version>/`, below).
3. Run the **quick install check** (below), commit and push. Don't wait for CI (`.github/workflows/studio.yml`):
   it runs in the background, and anything it catches is fixed in the next version. Before tagging, open the last
   GitHub run of the previous tag (Actions, workflow `studio`) and fix what it caught: nobody else looks (lesson of
   1.8.0 and 1.8.1, whose checks were red for two releases). The two jobs run on this PC's runner when the
   repository variable `BF_PC_RUNNER` is `on` (`tools/pc-runner/README.md`), otherwise on GitHub's Windows machines.
   The first builds the
   command line and the app, tests bf-core, bf-sim and bf-pack, renders and encodes the example episode, tests the
   app without a window and checks the package (each step runs even when an earlier one failed, unless it needs
   that step's output); the second runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
   warnings` and the tests of bf-story, bf-sound, bf-picture, bf-content and bf-export. bf-harness (which needs
   git-ignored reference files) and the interface tests (which need a DirectX 12 adapter) run only on this PC.
4. Push the tag `studio-v<version>` on that commit (`studio-v<version>-rc.1` for a candidate).
5. The owner clicks Update in the app. The app fetches the newest `studio-v*` tag into `data\source`, runs
   `cargo build --release --locked -p bf-app` there at below-normal priority, checks that the new exe names the
   tag's version and commit, and offers "Restart now".

### Quick install check (the default for every install)

The owner, Nov 16, 2026: speed over exhaustive checks. The full release suite (3,679 tests in 170 groups) took about
77 minutes on this PC: about 42 minutes to compile the test programs and about 32 minutes to run them. Installs don't
run it. From `studio/`, in the main folder:

1. `cargo build --release -p bf-app -p bf-pack` (the two exes the install uses).
2. `cargo test --release -p bf-app` (the app's own tests and every screen and flow, including the real-roster check
   of the fighter and theme counts in `tests/ui/matchup.rs`; about 5 minutes of running).
3. `cargo test --release -p bf-cli --test e2e` (one whole video renders end to end; about 1 minute).
4. Only the tests for what changed since the last tag (`git diff --stat studio-v<last>..HEAD`): for a new or changed
   fighter, its `cargo test --release -p bf-harness --test fighter_<name>`; for a changed crate, `cargo test
   --release -p <crate>`. Skip the other fighters' match checks, the theme parity checks and the composer sweeps.

Then install with `studio\target\release\ballfight-pack.exe install --exe studio\target\release\ballfight-studio.exe
--from <repo root>`. On this PC, five `bf-app` update/search tests can time out when many tests run at once; rerun
them alone before calling them failures. The fighter tests for bagpipes, bari_sax, synth, turntables and viola fail
here only because their git-ignored cache files are missing.

The GUITAR and SAX guard test (`bf-app/tests/retired.rs`) was removed on the owner's word (Nov 16, 2026: "The
checks for Guitar and Sax are unnecessary and you can remove those altogether"). The two stay retired; only the
test is gone.

A tag's version must equal the workspace version. A version tagged twice is not offered again (it is not newer). A
pre-release is offered only to an app that is itself a pre-release of that version or older.

**The rule for `tests/compat/<version>/`.** Every release adds a folder for its own version, made once with the
app's own code and then frozen; no folder is ever rewritten. From `studio/`:
`BF_BLESS=1 cargo test --release -p bf-app --test compat make_this_versions_files` (it refuses a folder that already
holds files), then add the new folder's SHA-256s to `FROZEN` in `crates/bf-app/tests/compat.rs`, as
`tests/compat/0.1.0/README.md` describes.

## What a build session needs on this PC

- Git, signed in to GitHub (the credential manager is already set up), and rustup. The toolchain is pinned in
  `studio/rust-toolchain.toml` (1.99.1); rustup fetches it by itself.
- Work in `studio/`. The gates before any commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and the tests of what changed (below). GitHub CI runs fmt, clippy and the tests of every crate but
  bf-harness (see the release steps above). An install runs only the quick install check above.
- **Feature and fighter work tests only what is needed** (the owner, Nov 16, 2026: "For adding features, make the
  tests only test what is necessary."). While building, and before merging, run only the tests of the crates and
  fighters you changed (`cargo test -p <crate>`, `--test fighter_<name>`), plus `cargo check --workspace
  --all-targets` so nothing else breaks to compile, and the count snapshots in `bf-app/tests/ui` when a count
  changes. Don't run the whole suite; GitHub CI does that.
- **Tests run on this PC, never in a cloud container** (the owner, Nov 23, 2026). A thread that works on this PC
  (Remote Control) runs its tests here as above. A thread in a cloud container builds and tests nothing there: it
  pushes, and names the tests it needs in its last commit's message, one line each, `PC-Tests: -p bf-sim --test
  violin_quick` (what follows `cargo test --release --locked` in `studio/`), at most 10 lines; one line can name
  several test files (`-p bf-harness --test a --test b`) or filters (`--lib -- music:: polish`). The `pc-tests`
  workflow runs them on this PC's runner, with the reference cache and DirectX 12, so bf-harness and the interface
  tests run too; the result is on that run's page (Actions, `pc-tests`). The same rules choose the tests: only what
  changed, none for a small change. Every push that touches `studio/` also runs the `studio` checks on this PC.
  Setup, the switch (`BF_PC_RUNNER`) and fixes: `tools/pc-runner/README.md`. The runner keeps its own build folder in
  `F:\bf-runner\target` (30 to 95 GB), which counts toward the 150 GB to keep free on F:.
- The reference Python (`.studio-tools\py313`) only for the exact route below.
- **Nothing under `projects/`, `reelkit/` or `series/` is ever edited.** `git status --short projects reelkit series`
  must be empty before every commit.

### Disk space: build folders (read before making a worktree)

One Rust build folder (`target`) of this repo grows to 30-95 GB, and the owner's F: drive filled twice (Nov 14 and
Nov 16, 2026) from threads each keeping their own. The rules:

- **Default: no worktree.** Build in the main folder, so its one `studio\target` is shared. (One shared
  `CARGO_TARGET_DIR` for several checkouts building at the same time gave stale-crate errors, so sharing is only for
  one checkout.)
- **If a thread needs its own checkout**, make it as a git worktree at `.wt\<name>` (git-excluded) and point its
  builds at `.wt\<name>-target`: `$env:CARGO_TARGET_DIR = "<repo>\.wt\<name>-target"`, and
  `$env:CARGO_PROFILE_DEV_DEBUG = "line-tables-only"` to roughly halve its debug build. Never put clones, worktrees or
  `target-*` folders anywhere else (not next to the repo, not in `studio\`), so the cleanup script finds them.
- **At most three own build folders at once.** Before making one, run the cleanup script's report and check free
  space on F: (keep at least 150 GB free).
- **When the work is merged and pushed, clean up before ending the thread:**
  `powershell -ExecutionPolicy Bypass -File studio\scripts\clean-builds.ps1 -Apply`. It lists every checkout with
  its size; it deletes only checkouts that are clean, pushed and merged into the build branch, and never one a program
  is running from. `-TrimMain` also drops the main folder's debug build when nothing uses it. Without `-Apply` it only
  reports.
- Keep `studio\reference\.cache` (pinned test data, slow to remake); the script never touches `studio\reference`.

## The TARGET DUMMY (developer fighter, Nov 18)

Engine 2 was scrapped on Nov 18 (the owner's call: Engine 1's look and characters stay; only cooler effects were ever
wanted, to be shown visually before they are built). The one thing kept from it is the idea of a **TARGET DUMMY**:
a straw training dummy that never attacks, drifts round the ring and can still be knocked out, for example renders of
another fighter's moves. It is in the roster (`bf-content/data/roster_dev.json`, `fighters::DEV`) so a fight file can
name it, but never offered in the app. Its weapon and abilities are kind `dummy` (`bf_core::roster::DUMMY`), which the
engine lets in and skips. Example: `ballfight render fights/dev/trumpet_vs_target_dummy.toml --out <folder>`. Its
fights never "fit the series" (no lead changes); that is expected, and the render's own checks pass.

## The bold effects (attack upgrades, Nov 24)

The owner picked upgraded attack visuals from a review page of all 42 fighters: twelve shared upgrades (hits in
three sizes, weapons that stop on contact, wind-up warnings, a speed-up look, a low-HP spotlight, shields that show
the block, livelier ropes, a level-up pop, status marks, less floor mess), a new look for all four moves of every
fighter, and the special-move names on **side banners** (each fighter's own ribbon from its own side, so two names
can show at once; the 4 s gap between names counts per fighter).

- **Opt-in, look-only.** A fight file asks for them with `picture.effects = "bold"` (a format 2 key; the default
  `classic` is every frame exactly as before, which is what the series' videos and the reference pictures use). The
  app sets `bold` for every fight it builds (`bf-app/src/engine/convert.rs`), except an imported series episode.
  Nothing of the fight, its timing or its sound changes.
- **Where:** the toolkit and the ring's own upgrades in `bf-picture/src/bold/`; each fighter's moves in
  `bf-content/src/fighters/<slug>/bold.rs`, behind `if p.bold` in its look.
- **How to build or change a fighter's bold moves:** `docs/studio/bold-effects.md`. A new fighter needs a `bold.rs`
  too (without one its moves are drawn classic inside a bold fight, which works but looks plainer).
- **Looking at moves without rendering:** `ballfight moves <fight file> --out <new folder> [--zoom] [--kinds ...]`
  draws contact sheets of the frames round each kind of event. `fights/dev/bold_<slug>.toml` (against the target
  dummy) and `bold_<slug>_low.toml` (against a real fighter with the odds against it, for the low-HP move) exist
  for every fighter.

## Voice Engine 2 (realistic instruments, from Nov 26)

Voice Engine 2 is the Studio's own instrument synthesizer. Since stage 1b each instrument family has its **own built
engine**, modelled from how the instrument physically works and judged by ear, with real recordings as the reference
for facts (the owner's rule, 2026-11-26): the piano is the struck-string engine (`strings.rs` with `board.rs`, the
owner's pick P16), the organ the pipe engine (`pipes.rs`, O8). Every instrument plays all its techniques (pedals,
harmonics, stops, swell box...), and the engines run one note at a time at any sample rate (a plugin could use them
later). Stage 1's **recipe** engine (overtones measured from recordings) is kept but played by no fighter. It all
plays only under the Composed arrangement; Classic is untouched, byte for byte. The families follow one stage at a
time, each with its own engine and voice lab, stopping at a listening page for the owner's review
(`docs/studio/2026-11-26-voice-engine-2-design.md`, "Revision: built engines").

- **Where:** the engines in `bf-sound/src/voice/engine2/` (`strings.rs`, `board.rs`, `pipes.rs`, `eq.rs`; `play.rs`
  for how a note is played; `space.rs` for width and an instrument's own room; the kept recipe engine's format,
  player and parts); each fighter's instrument in `bf-content/src/fighters/<slug>/voice.rs` (`ENGINE2`, built with
  `Instrument::built`; `RECIPE` the kept recipe voice); parts ask for notes with `MusicCtx::note_played`. The built
  voices' yardstick is `bf-harness/tests/engine2_built.rs` (the listening riffs against the real instruments' octave
  bands and the owner's picks as built); each technique has its own unit test beside its engine. The recordings
  tool is `cargo run --release -p bf-harness --example recipes -- <fetch|study|measure|check|reference|sets>`, the
  measuring `bf-harness/src/e2measure.rs`.
- **The licence rule:** a shipped recipe is measured only from CC0 or "free for any use" recordings. CC BY and
  CC BY-SA sets may be studied (general facts about the instrument), never shipped from; non-commercial and
  all-rights-reserved sets are not used at all. Every set is listed in `docs/studio/voice-engine-2-sources.md`.
  Recordings are never committed (they live in the git-ignored `studio/reference/.cache/`).
- **Adding a family** (a later stage): `recipes fetch` and `recipes study` its sets (PC-only sets by Remote
  Control) and write its sheet in `docs/studio/voice-engine-2-instruments.md` (working parts, every technique, the
  spread across the sources, the published acoustics); sketch its engine from its parts in a voice lab and let the
  owner pick by ear (rounds of named variants); port the pick into `engine2/` (one sample or one short block at a
  time, the sample rate passed in, never a fixed 48 kHz) with a test per technique; give the fighter its `ENGINE2`
  (`Instrument::built`) and add it to `engine2_built.rs`; then `ballfight levels <fights> --write
  crates/bf-content/src/levels_table.rs` for its row, and the speed check (at most 25% slower than before on its
  fights).
- **Playing:** `docs/studio/voice-engine-2-playing.md` lists what a note can ask for (accent, swell, pedals, mute,
  technique, stops, swell box, chord size...) and each instrument's techniques, for the owner's composition system.

## How to add a fighter

The default is the **quick route** the 10 older fighters used (the owner's Nov 14 decision, ruling M4-R16): a
faithful-looking, faithful-sounding port, judged by eye and ear. The seed finder handicaps pairs to be fair, so a
new fighter's fights need not match anything bit for bit. A theme is added the same way (its folder in
`bf-content/src/themes/`, its bed, its arrangement table, its card, `PORTED_THEMES`).

1. **Kinds.** Any new weapon or ability is a new module (`bf-core/src/roster/kinds|abilities/` and
   `bf-sim/src/kinds|abilities/`); existing ones are not changed, so no existing fight changes.
2. **Its folder** in `studio/crates/bf-content/src/fighters/<slug>/`, shaped like a merged one (`organ`, `contrabass`,
   `bagpipes`, `bari_sax`, `turntables`): its look, voice and part, card, title pattern, and its musical personality
   (`personality.rs`, 2 to 4 signature motifs). Its colour is a new, unique one in the table of
   `docs/ballfight/RULES.md`.
3. **Offer it:** add the name to `PORTED` in `bf-content/src/registry.rs`, and give it a starter pairing and a home
   theme in `bf-content/src/starter.rs`, as the older fighters have.
4. **Proof, small:** a short test file like `studio/crates/bf-sim/tests/violin_quick.rs` (its fights against 3 or 4
   different fighters run to the end and a K.O. happens in a normal share of seeds), run with
   `cargo test -p bf-sim --test <slug>_quick`; `cargo test -p bf-content` (the registry and personality tests take
   the new name); and `cargo test --release -p bf-sim --test all_pairings` (about 10 to 20 seconds): every
   pairing, from both corners, fights 3 seeds to the end with no panic, no NaN and nobody leaving the ring. It
   reads the rosters, so a new fighter is in it as soon as its roster entry is; after a new weapon kind, run it
   once with `ALL_PAIRINGS_SEEDS=40` too (about 2.5 minutes). It caught TUBA v VIOLIN and BASS GUITAR's runaway
   dash in 1.8.1. Then from `studio/`, with a fight file for the new pairing:
   `ballfight search <fight file> --stop 10`, then `ballfight render <fight file> --out <folder> --seed N` with one
   of its fits. Look at a few frames (the card, mid-fight with an ability, the K.O.; `ballfight frame <fight file>
   --frame N --out <file.png>`) next to the old episode's look; the render's own checks pass, or the report says
   which one fails and why that is acceptable.
5. `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace --all-targets`
   and the tests above (not the whole suite: CI runs that), on this PC (from the cloud: `PC-Tests:` lines, above),
   then the release steps above.

**The exact route (optional)**, used by the 20 kit fighters in milestone 3, only when a fighter must match a Python
reference bit for bit: export its references from the frozen kit (`python -m tools.studio_export fighter_<slug>`,
then `cache fighters/<slug>`, with the Python and environment of `studio/README.md`; an export reads the kit and
never writes to it), then pass the gates of
`docs/studio/2026-11-11-ballfight-studio-plan-m3-kit-roster.md` Task 6 step 5: `cargo test -p bf-sim --test
fighter_<slug>` (and with `-- --include-ignored`), `cargo test --release -p bf-sim --test fighter_<slug> stats`,
`cargo test -p bf-content --test look_<slug> --test voice_<slug> --test sound_<slug>` (and with
`--include-ignored`), and milestone 1's `replay`, `golden_frames` and `stems` unchanged.

## What is left

1. **Milestone 6 is finished:** the user guide and version 1.0.0 shipped, and every version since has its
   `studio-v*` tag. New fighters and themes follow the steps in this file.
2. **Milestone 3's last two tasks** (62 and 63): the long measurement runs and the tidy list. The measurements file
   says which sections are still empty and why.

## Known limits

- **The render fixes what a check would fail instead of stopping** (Aedan, Nov 18, 2026: "All tests that would
  make the render fail should be fixed rather than just stopping the render"). `bf-export/src/fix.rs` holds the
  fixes and its table: the lens's sharp zone is widened where a fighter would be blurred (focus), when the safe-zone
  check fails the count-in is kept inside the safe zone and the checks run again (this cures FIGHT! at 138 BPM; anything
  still sticking out is only noted: the safe zone never stops a render, Aedan, Nov 22, 2026), a fighter covered by the music is turned up there (band), the loudness is evened out (levels), the
  loop point's sound and picture are joined, flashes are softened (on the checked stretch and, by a guard, on every
  frame given to ffmpeg), and a finished file that fails its own checks (or an ffmpeg that stops) is made once more
  with the flashes held one under the rule. Each fix leaves a passing video exactly as it was and adds a sentence
  to its check's summary. Only a check that still fails after its fix, or one that cannot measure at all (an app
  fault), stops the render. Under the Composed arrangement (sound upgrade, Nov 25, 2026) a measured sound check
  (band, levels, loudness, loop sound) that still fails after its fix is only noted and never stops the render;
  the band check reads the fighters over the band by ear (`checks::band::band_heard`), and after any sound fix the
  loudness is brought back to -14 LUFS. `ballfight check` still reports the checks before any fix (for
  diagnosis); the Music view of the app shows them after the sound fixes, as the render does.
- **TROMBONE vs HORN on brass trailer** fails the levels check before the fix (the brass trailer ending is too quiet
  after a late K.O.); the render evens it out.
- **The band check ignores an early K.O.** It looks at 32 to 44 s, so a fight with a K.O. at 31 s would fail it. The
  seed finder only offers K.O.s between 49 and 53 s, so this does not affect real videos.
- **Four kit crossovers have no fit at an even handicap** (CELLO vs FLUTE, BASS DRUM vs HORN, EUPHONIUM vs CLARINET,
  ELECTRIC vs BASSOON; the kit finds none either). Use Auto-balance.
- **Some older fighters need a big handicap** against some kit fighters (VIOLIN against TROMBONE, for example).
  Auto-balance handles it.
- **A very late K.O. can leave the winner no solo.**
- **A fight the Studio runs itself is its own fight**: it matches the kit's fight for at least the first 25 seconds,
  then tiny rounding differences take over (by design).
- **The preview's speed tests can fail when the PC is busy**; they pass on a quiet PC.
- **The folder dialog may open behind the app** (it has no parent window; `studio/crates/bf-app/README.md`).
- **The exe's own icon in Explorer** is Windows' generic one; the shortcuts and the window have the app's icon.

## Background from before the port (history)

The sections below were written before the Studio existed, when the plan was a front end on top of the Python kit.
They are kept for the series rules and the reading list; where they speak of the Python kit as the engine, the
Studio has replaced it.

## 1. Read these (in order, ~600 lines total)

1. `docs/studio/PLAN.md`: the product spec (what the app does, technical shape, Windows notes, build steps).
2. `projects/ballfight/README.md`: the episode kit: the commands (`python -m ballfight new|tune|search|preview|sheet|
   audio|check|render|verify|post <episode>`), the config format, where fighters and moods live, porting steps, timings.
3. `docs/ballfight/RULES.md`: the series rules every video must follow (title card, HUD, captions, colours table,
   crossover rules, retired fighters, no episode number, no approval waits).
4. `docs/PLAYBOOK.md`: the video rules behind them (61.0 s, 1080x1920, 30 fps, safe zone, loop point, loudness, flashes).
5. One finished kit episode end to end: `projects/ballfight/examples/flute_vs_trombone/` (a ~60-line config) and its
   episode folder, to see a config to mp4 run.

Don't read the other ~55 idea files, logs or old episode folders unless a task needs one (they're history).

## 2. What exists today

- **Engine:** `projects/ballfight/fight.py` (honest, deterministic 2D sim), `fighters.py` (the roster: numbers per
  fighter), `projects/ballfight/kit/` (picture, sound, HUD, title card, tune, search, check, encode), `reelkit/` (the
  shared renderer, effects, UI, checks).
- **In the kit (usable in the app now), 20 fighters:** ACOUSTIC, ALTO SAX, BASS CLARINET, BASS DRUM, BASSOON, CELLO,
  CLARINET, CONTRABASS, ELECTRIC, EUPHONIUM, FLUTE, HORN, OBOE, ORGAN, PIANO, SOUSAPHONE, TENOR SAX, TROMBONE, TRUMPET,
  TUBA. **14 moods:** baroque_storm, big_top, bossa_noir, brass_trailer, dark_disco,
  dark_drumline, dark_tango, heist, horror_trap, hot_jazz, mambo, prog_duel, second_line, stadium_rock.
- **Not in the kit (old episode folders only), but in the Studio since milestone 4:** fighters ACCORDION, BAGPIPES,
  BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE, VIOLA, VIOLIN; their moods (drift phonk, Balkan fire, noir jazz,
  sea shanty, dark breakbeat, villain march, spaghetti western, gothic horror, brass house, ska-punk, industrial,
  orchestral drill). They live only in the Studio; the kit is not changed for them. Progress:
  `docs/log/ballfight_studio.md`. **GUITAR and SAX are retired**: keep their roster entries
  (old episodes replay), never offer them in the app.
- **Checks that must keep passing:** `tools/ballfight_replay_hash.py` (every finished episode replays identically), the
  kit's `check` (safe zone, focus, levels / consistent loudness, band, flash, loop) and `verify`.

## 3. Non-negotiables for the software

- **Honest fights:** the app runs the real simulation; the user picks a seed from the search, never edits a fight.
  Balance only through the per-fighter damage handicap (the crossover rules).
- **Determinism:** `PYTHONHASHSEED=0` before Python starts (the launcher sets it); same config + seed = same video.
- **Don't change finished episodes:** engine / kit / title changes are additive and opt-in; the replay guard stays
  green; every earlier title card and HUD draws exactly as before.
- **Video spec:** 61.0 s, 1080x1920, 30 fps, h264 + aac; loop point at the file boundary; the safe zone; consistent
  loudness from frame 0 (`reelkit.levels`); no full-screen flashes (`reelkit.flashcheck`); no episode number on screen;
  colours from the table in `docs/ballfight/RULES.md` (one unique colour per instrument).
- **Windows:** the kit's process pools use `fork` (`kit/preview.py`, `kit/search.py`, `kit/tuning.py`, `kit/encode.py`);
  Windows needs `spawn` (module-level workers, `if __name__ == "__main__"` guards), with identical results. Paths via
  `pathlib`; ffmpeg on PATH or bundled. This dev container has 2 cores; the user's PC will be faster.

## 4. Ask the user (things only they can decide)

- PC specs: Windows version, CPU cores, RAM, free disk (a render keeps ~1.2 GB of raw frames), GPU (not needed).
- Where outputs go (default `Videos\BALLFIGHT\<a>_vs_<b>\`) and what each export includes (mp4, caption.txt, config).
- How much control they want in v1: fighter pick, sides, mood, tempo, handicap slider / auto-balance, seed browser,
  schematic preview, history (PLAN.md's list), and anything later (arenas, 3 or 4 fighter brawls, tournaments, the
  "all instruments vs the band director" boss fight from `docs/ideas/requests.md`).
- Whether the app stays in this repo (recommended: it imports the kit directly) and how they'll get updates (git pull).
- Whether new fighters / moods will still come from building sessions (yes so far: the ideas chat drafts, a session builds,
  the app picks them up after an update).

## 5. Useful references

- Viewer requests and feedback: `docs/ideas/requests.md` (e.g. the TUBA is the most criticised fighter).
- Fighter redesign history: `docs/ballfight/redesign-2026-10-24.md`.
- Each episode's design: `docs/ideas/NN-*.md` (linked from the board, `docs/IDEAS.md`).
