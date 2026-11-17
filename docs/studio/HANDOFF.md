# BALLFIGHT Studio: handoff brief (updated Nov 15)

> **Engine 2 (Nov 17):** a new fight engine and GPU renderer are being built beside the current one. Read
> `docs/studio/engine2/README.md` before touching how fights are fought or drawn.

## Status (Nov 15): the packaged Studio, 30 fighters, 26 themes, the music system

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

**What it can make.** Any of the 870 pairings (30 fighters, each against each of the other 29, either side), on any
theme the pair is offered: pick the two fighters, balance, pick a seed from the shortlist with its picture and
sound, choose the music, and render a checked 61 s video with its cover and caption into `Videos\BALLFIGHT`.

**The roster** (counted from the registry: `bf_content::registry::PORTED` holds 30 names,
`bf_content::themes::PORTED_THEMES` 26).

- **The 20 kit fighters are exact** against the Python kit: ACOUSTIC, ALTO SAX, BASS CLARINET, BASS DRUM, BASSOON,
  CELLO, CLARINET, CONTRABASS, ELECTRIC, EUPHONIUM, FLUTE, HORN, OBOE, ORGAN, PIANO, SOUSAPHONE, TENOR SAX, TROMBONE,
  TRUMPET, TUBA.
- **The 10 older fighters were ported "close enough for the eye test"**, as the owner decided on Nov 14: ACCORDION,
  BAGPIPES, BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE, VIOLA, VIOLIN. They fight, look and sound like
  themselves, but are not checked bit for bit against their old episodes. Each starts on its own home theme.
- **The 14 kit themes are exact**: baroque storm, big top, bossa noir, brass trailer, dark disco, dark drumline, dark
  tango, heist, horror trap, hot jazz, mambo, prog duel, second line, stadium rock.
- **The 12 older themes were ported the same way, by eye and ear**: Balkan fire, brass house, dark breakbeat, drift
  phonk, gothic horror, industrial, noir jazz, orchestral drill, sea shanty, ska-punk, spaghetti western, villain
  march.
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
   it runs the full suite in the background, and anything it catches is fixed in the next version.
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
  warnings`, `cargo test --workspace` (GitHub CI runs these). An install runs only the quick install check above.
- **Feature and fighter work tests only what is needed** (the owner, Nov 16, 2026: "For adding features, make the
  tests only test what is necessary."). While building, and before merging, run only the tests of the crates and
  fighters you changed (`cargo test -p <crate>`, `--test fighter_<name>`), plus `cargo check --workspace
  --all-targets` so nothing else breaks to compile, and the count snapshots in `bf-app/tests/ui` when a count
  changes. Don't run the whole suite; GitHub CI does that.
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

## How to add a fighter

Engine 2 fighters follow `docs/studio/engine2/FIGHTER-GUIDE.md`; the steps below are for the current engine.

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
   the new name). Then from `studio/`, with a fight file for the new pairing:
   `ballfight search <fight file> --stop 10`, then `ballfight render <fight file> --out <folder> --seed N` with one
   of its fits. Look at a few frames (the card, mid-fight with an ability, the K.O.; `ballfight frame <fight file>
   --frame N --out <file.png>`) next to the old episode's look; the render's own checks pass, or the report says
   which one fails and why that is acceptable.
5. `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, then the release
   steps above.

**The exact route (optional)**, used by the 20 kit fighters in milestone 3, only when a fighter must match a Python
reference bit for bit: export its references from the frozen kit (`python -m tools.studio_export fighter_<slug>`,
then `cache fighters/<slug>`, with the Python and environment of `studio/README.md`; an export reads the kit and
never writes to it), then pass the gates of
`docs/studio/2026-11-11-ballfight-studio-plan-m3-kit-roster.md` Task 6 step 5: `cargo test -p bf-sim --test
fighter_<slug>` (and with `-- --include-ignored`), `cargo test --release -p bf-sim --test fighter_<slug> stats`,
`cargo test -p bf-content --test look_<slug> --test voice_<slug> --test sound_<slug>` (and with
`--include-ignored`), and milestone 1's `replay`, `golden_frames` and `stems` unchanged.

## What is left

1. **Milestone 6's last steps:** the user guide and its pictures (Task 17, being written now) and the release
   itself with its checklist (Task 19, version 1.0.0), both in
   `docs/studio/2026-11-11-ballfight-studio-plan-m6-packaging.md`.
2. **Milestone 3's last two tasks** (62 and 63): the long measurement runs and the tidy list. The measurements file
   says which sections are still empty and why.

## Known limits

- **TROMBONE vs HORN on brass trailer has no seed that passes every check.** The kit's own fight (seed 140) and its
  first ten fits all fail the loudness check: after a late K.O. the brass trailer ending is too quiet. The kit gives
  the same verdict. The rematch (HORN vs TROMBONE, same theme) passes. Pick another theme for that pair. The app does
  not yet suggest another theme by itself.
- **The seed search does not listen to the soundtrack**, so a seed from the shortlist can still fail the loudness
  check at render time. The render job then refuses it and says why; pick the next seed.
- **The FIGHT! caption fails the safe zone at 138 BPM**, for any pairing. Avoid 138 BPM until it is tidied.
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
