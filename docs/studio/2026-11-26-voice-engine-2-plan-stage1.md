# Voice Engine 2, stage 1 (pilot: the engine, piano and organ) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Under the Composed arrangement, the PIANO and ORGAN fighters (and the mambo and noir jazz band pianos, which
share the PIANO voice) play notes rebuilt from recipes measured off real CC0 recordings, with modelled working parts,
a playing interface, width and a light organ room. The stage ends on a listening page and stops for Aedan's review.

**Architecture:** A new `bf_sound::voice::engine2` module holds the recipe format, the player (recipe to note), the
piano's and organ's working parts, the `Play` type and the bus `Space` (width, home room). A `Voice` says it has an
engine 2 instrument through `Voice::engine2()`; `MusicCtx::note_in` plays it under Composed in place of
`note_real`. A `bf-harness` example, `recipes`, fetches recordings, studies every source of an instrument, measures
the shipped recipes into `bf-content/data/voices/` and checks them. Classic never reaches any of it.

**Tech Stack:** Rust (workspace `studio/`), `bf_core::dmath` maths, `bf_core::rng::Generator`, ffmpeg (decoding
recordings in the tool only), git (fetching recording sets), the PC runner for every test.

**Spec:** `docs/studio/2026-11-26-voice-engine-2-design.md` (approved 2026-11-26 08:56Z). Read it first; this plan
argues from it.

## Global Constraints

- Composed only: everything new runs only when `ctx.composed_on()`; Classic stays bit-exact (`classic_golden`, kit
  parity, every `fighter_*` Classic gate unchanged).
- Deterministic: maths through `bf_core::dmath`, randomness through seeded `bf_core::rng::Generator`; the same note
  twice is the same samples, on every PC.
- Notes stay mono and exactly `int(cap * SR)` samples (through `fit`), `SR` = 48,000.
- No overtone above 0.45 of the sample rate.
- Recipes: at most 1.5 MB per instrument, 40 MB for all, in `studio/crates/bf-content/data/voices/`, compiled in
  with `include_bytes!`. No recorded audio in the repository; recordings live in the git-ignored
  `studio/reference/.cache/recordings/`.
- Shipped recipes only from CC0 or "free for any use" sets (piano: VCSL Steinway B; organ: VSCO 2 CE). Study may also
  use CC BY and CC BY-SA sets; never non-commercial or "all rights reserved" sets.
- Speed: a Composed `ballfight sound` build at most 25% slower than before, same machine.
- Tests run only on Aedan's PC: `PC-Tests:` lines in the last commit message before a push
  (`tools/pc-runner/README.md`). In the cloud: build, `cargo test --no-run` (compile only), fmt and clippy; never run
  tests. Test only what changed.
- Branch `claude/project-thread-bkwt2k`, PR #29 into `claude/project-thread-h102zt`. Never install. The stage
  stops at its listening page until Aedan has reviewed it; it merges only after his review and green PC tests.
- Comments and docs in the project's plain style (see any `voice.rs`): what a thing is and why, no jargon without
  its meaning.

## Review Focus

1. **A cap shorter than the note's start** (TOCCATA's 16ths: `dur` 0.11, cap 0.33; a stab of 0.08 s): the note must
   still be exactly cap long, finite, and fade cleanly. Test in Task 8 (`a_note_shorter_than_its_attack_fits_its_cap`).
2. **Pitches outside the recordings** (the organ's stomp plays `key_root - 12`, down to MIDI 24; piano notes past 96):
   the nearest recorded pitch's tone, moved, finite, never NaN or a panic. Test in Task 5
   (`pitches_past_the_recordings_take_the_nearest_tone`).
3. **A note longer than its recording** (a held CATHEDRAL, a 1.8 s piano solo chord, any `dur` past the recipe's
   last time point): it carries on (piano: the late decay; organ: the steady sound with its wind), never loops, never
   drops to silence. Test in Task 5 (`a_note_longer_than_its_recipe_carries_on`).
4. **Two builds in one process, or two threads** (the app renders previews while a render runs): the same note gives
   the same samples (the parsed recipes are shared read-only). Test in Task 5 (`the_same_note_is_the_same_everywhere`).
5. **A band piano's level** (mambo, noir jazz: no level table for theme buses): the engine 2 piano must sit where
   the old piano sat in the band. Test in Task 8 (`an_engine2_voice_keeps_the_old_voices_level`).

---

## File Structure

| File | Responsibility |
| --- | --- |
| `bf-sound/src/voice/engine2/mod.rs` (create) | The module's notes; `Instrument`, `Parts`, `play()` (dispatch to the parts) |
| `bf-sound/src/voice/engine2/recipe.rs` (create) | The recipe format: `RecipeSet`, `Note`, `Partial`, the time grid, `to_bytes`/`parse` |
| `bf-sound/src/voice/engine2/player.rs` (create) | A recipe set to samples: interpolation, oscillators, noise, length, variants |
| `bf-sound/src/voice/engine2/piano.rs` (create) | The piano's working parts: damper, pedal, strength |
| `bf-sound/src/voice/engine2/organ.rs` (create) | The organ's working parts: registration, wind, sag, release |
| `bf-sound/src/voice/engine2/play.rs` (create) | `Play` and its fields (accent, slur, bend, swell, vibrato, pedal, mute, together) |
| `bf-sound/src/voice/engine2/space.rs` (create) | `Space`: width, bass-left/treble-right tilt, the home room |
| `bf-sound/src/voice/mod.rs` (modify) | `Voice::engine2()`; `pub mod engine2;` |
| `bf-sound/src/music/mod.rs` (modify) | `note_in`'s engine 2 path, `note_played`, `engine2_of`, the level match |
| `bf-sound/src/polish/mod.rs`, `polish/room.rs` (modify) | `Part::space` applied in `treated`; `room::wet_with` |
| `bf-harness/src/e2measure.rs` (create) | Measuring a recording: decode, onset, pitch and stretch, partial tracks, noise, release, yardstick |
| `bf-harness/examples/recipes.rs` (create) | `fetch`, `study`, `measure`, `check`, `reference` |
| `bf-content/data/voices/PIANO.ve2`, `ORGAN.ve2` (create, by the tool) | The shipped recipes |
| `bf-content/src/fighters/piano/voice.rs`, `organ/voice.rs` (modify) | `engine2()`; the stage 3 rework tables removed |
| `bf-content/src/fighters/organ/sound.rs` (modify) | Composed chords through `note_played` (chord size; the big moves' swell) |
| `bf-content/src/soundtrack.rs` (modify) | `polish::Part::space` from the fighter's instrument |
| `bf-content/src/levels_table.rs` (rewritten by `ballfight levels --write`) | PIANO and ORGAN rows |
| `bf-harness/tests/engine2_voices.rs` (create) | The yardstick: against the recordings and the spread of every source |
| `bf-harness/tests/reference/engine2/PIANO.json`, `ORGAN.json` (create, by the tool) | The recordings' yardstick numbers and the study spread |
| `bf-harness/tests/real_voices.rs`, `examples/references.rs` (modify) | PIANO and ORGAN out of the stage 3 checks |
| `docs/studio/voice-engine-2-instruments.md`, `voice-engine-2-sources.md`, `voice-engine-2-playing.md` (create) | Instrument sheets, sources and licences, the playing fields |
| `studio/packaging/guide/index.html` (modify) | "About the sounds" (credits as a courtesy) |
| `docs/studio/HANDOFF.md` (modify) | One paragraph: Voice Engine 2, where things live, how to add an instrument |

All paths below are under `studio/crates/` unless they start with `docs/` or `studio/`.

---

### Task 0: Start from the sound upgrade's last work

**Files:** none changed by hand.

- [ ] **Step 1:** If PR #28 has merged: `git fetch origin claude/project-thread-h102zt && git merge --no-edit FETCH_HEAD`.
  If not yet: `git fetch origin claude/project-thread-2oi97f && git merge --no-edit FETCH_HEAD` (its head was
  `8ec50959`); merge the build branch again once #28 merges, before Task 9.
- [ ] **Step 2:** `cd studio && cargo build --release -p bf-cli` succeeds. Keep this binary as
  `/tmp/claude-0/ballfight-before` (the "today" side of the speed ratio and the listening page).
- [ ] **Step 3:** Commit nothing; push the merge with the next task.

### Task 1: The recipe format

**Files:**
- Create: `bf-sound/src/voice/engine2/mod.rs`, `bf-sound/src/voice/engine2/recipe.rs`
- Modify: `bf-sound/src/voice/mod.rs` (`pub mod engine2;`)

**Interfaces:**
- Produces (in `engine2::recipe`):
  - `pub const MAGIC: [u8; 4] = *b"VE2R"; pub const VERSION: u16 = 1;`
  - `pub const BANDS: usize = 24;` and `pub fn band_edges() -> [f64; 25]` (log-spaced, 60 Hz to 16 kHz)
  - `pub fn grid(end: f64) -> Vec<f64>`: 0 to 0.1 s every 2.5 ms, then each step 6% longer than the last, at most
    0.1 s, until `end` (inclusive of a point at or past `end`)
  - `pub struct Layer { pub name: String, pub strength: f64, pub gain_db: f64 }`
  - `pub struct Partial { pub ratio: f64, pub level: Vec<f32>, pub cents: Vec<f32> }` (one value per grid point;
    `level` in dB, `f32::NEG_INFINITY` silent; `ratio` = the partial's frequency over the note's measured `f0`)
  - `pub struct Note { pub layer: usize, pub midi: f64, pub f0: f64, pub b: f64, pub release: f64, pub partials:
    Vec<Partial>, pub noise: Vec<[f32; BANDS]> }` (`midi` measured, not rounded; `release` the time constant in s
    of the instrument's own stop; `noise` one row per grid point, dB)
  - `pub struct RecipeSet { pub name: String, pub grid: Vec<f64>, pub layers: Vec<Layer>, pub notes: Vec<Note> }`
  - `impl RecipeSet { pub fn to_bytes(&self) -> Vec<u8>; pub fn parse(b: &[u8]) -> Result<RecipeSet, RecipeError>;
    pub fn nearest(&self, midi: f64, strength: f64) -> Corners }` where `Corners` holds up to four `(note index,
    weight)` pairs (two pitches times two layers, weights summing to 1)
  - `pub struct RecipeError(pub String)` with `Display`
- Format (little-endian, documented at the top of `recipe.rs`): magic, version `u16`, name (`u8` length + UTF-8),
  grid (`u16` count + `f32`s), layers (`u8` count; each name, `f32` strength, `f32` gain), notes (`u16` count; each
  `u8` layer, `f32` midi, f0, b, release, `u16` partial count; each partial `f32` ratio, then levels as `i16`
  centi-dB (`i16::MIN` silent), then cents as `i16` tenths; then the noise as `i16` centi-dB, grid-major).

- [ ] **Step 1: Write the failing tests** (in `recipe.rs`):
  - `a_set_survives_the_round_trip`: a two-layer, three-note set with silent and loud points; `parse(to_bytes(s))`
    equals `s` within 0.005 dB and 0.05 cents.
  - `the_grid_is_fine_early_and_coarse_late`: `grid(6.0)` starts `[0.0, 0.0025, 0.005]`, has 41 points up to 0.1 s,
    no step over 0.1 s, last point >= 6.0, under 140 points.
  - `a_wrong_file_is_refused`: wrong magic, version 2, and a file cut short each give `Err` naming the problem.
  - `the_nearest_corners_interpolate`: notes at MIDI 48 and 54 on two layers (0.5, 1.0): `nearest(50.0, 0.75)` has
    four corners with weights (2/3 of 48, 1/3 of 54) times (1/2, 1/2); `nearest(40.0, 2.0)` is the one corner
    (48, layer 1.0) at weight 1.
- [ ] **Step 2:** `cargo test --no-run -p bf-sound` compiles; the tests can't pass yet (stubs `unimplemented!()`).
- [ ] **Step 3:** Implement the types and functions above in `recipe.rs`; `mod.rs` holds the module notes (what
  engine 2 is, with a pointer to the design) and `pub mod recipe;`.
- [ ] **Step 4:** `cargo test --no-run -p bf-sound` compiles; `cargo clippy -p bf-sound` is clean.
- [ ] **Step 5: Commit** `Voice Engine 2: the recipe format` (tests run on the PC with Task 5's push).

### Task 2: Measuring a recording

**Files:**
- Create: `bf-harness/src/e2measure.rs`; Modify: `bf-harness/src/lib.rs` (`pub mod e2measure;`)

**Interfaces:**
- Consumes: `engine2::recipe::{grid, band_edges, Note, Partial, BANDS}`.
- Produces (in `bf_harness::e2measure`):
  - `pub fn decode(path: &Path, ffmpeg: &Path) -> Vec<f64>`: the file as mono (channels averaged) at 48 kHz, through
    `ffmpeg -v error -i <path> -f f64le -ac 1 -ar 48000 -`
  - `pub fn onset(x: &[f64]) -> usize`: the first sample above -40 dB of the peak, less 1 ms
  - `pub fn pitch(x: &[f64], nominal: f64, stretched: bool) -> (f64, f64)`: `(f0, b)` fitted on the first 20 partials
    of a 0.68 s frame from 50 ms (peaks within 2%/sqrt(k) of `k f0 sqrt(1 + b k^2)`, least squares on
    `(f_k/k)^2 = f0^2 + f0^2 b k^2`, three rounds); `b` = 0 when not `stretched`
  - `pub struct Opts { pub stretched: bool, pub sub: bool, pub max_partials: usize, pub end: f64, pub floor_db: f64 }`
    (`sub`: partials on the 16' sub-octave grid, f0 / 2, as the organ's chorus has; `floor_db`: partials quieter than
    this under the note's strongest at every point are dropped)
  - `pub fn measure(x: &[f64], nominal_midi: f64, layer: usize, o: &Opts) -> Note`: the partial tracks on `grid(o.end)`
    (each point: the windowed sum of `x e^(-i 2 pi f_k t)` under a Blackman window of 6 / f0 s centred on the point,
    times 2 over the window's sum, for the level; the phase change to a second sum 1 ms later for the cents), the
    noise bands (each point: a 2,048-sample Hann frame, the bins within f0 / 4 of any partial left out, each band's
    mean power of the rest times its bin count; a band with no bins left takes its neighbours' mean), the steady hiss
    removed (each band's power over the last 0.5 s, when that is 60 dB under the peak, subtracted), `release` 0 (set
    by `release`)
  - `pub fn release(x: &[f64], f0: f64, from: Option<f64>) -> Option<f64>`: the time constant of the fall at the key's
    release: the level at `f0` (20 ms window) fitted over 20 to 150 ms after `from` (or after the first point it falls
    under 0.7 of its median between 1 s and that point, when `from` is `None`); `None` if no fall is found
  - `pub struct Yard { pub levels: [[f64; 16]; 5], pub start: f64, pub stop: Option<f64>, pub noise_early: f64,
    pub noise_steady: f64, pub bright_early: f64, pub bright_late: f64, pub fall: f64 }`; `pub const YARD_TIMES:
    [f64; 5] = [0.03, 0.1, 0.3, 1.0, 2.5];` `pub fn yard(x: &[f64], f0: f64, b: f64, release_at: Option<f64>) -> Yard`
    (levels: the first 16 partials at each time, dB under the strongest at 30 ms; `start`: time to 90% of the peak
    5 ms level; `stop`: time to fall 40 dB after `release_at`; noise shares: energy between partials over all energy,
    0 to 100 ms and 0.3 to 1 s; brightness: spectral centroid in Hz, 50 to 500 ms and 0.8 to 1.2 s; `fall`: partial 1's
    slope in dB/s from 0.3 to 2.5 s) and `impl Yard { pub fn to_json(&self) -> Value; pub fn from_json(v: &Value) ->
    Option<Yard> }`

- [ ] **Step 1: Write the failing tests** (in `e2measure.rs`, synthetic signals only, no recordings):
  - `the_pitch_and_stretch_are_found`: a 2 s tone at 261.63 Hz, 20 partials at `k f0 sqrt(1 + 0.0004 k^2)`:
    `pitch` gives f0 within 0.05 Hz and b within 2e-5.
  - `a_partials_fall_is_tracked`: partial 3 falling 10 dB/s: its levels at 0.3 s and 1.3 s differ by 10 +- 0.3 dB; a
    partial 1.5 Hz sharp reads +-0.3 cents of its true offset.
  - `noise_between_the_partials_is_measured_and_the_hiss_removed`: a tone plus white noise burst over the first
    100 ms plus a constant hiss at -70 dB: band levels in the first 50 ms are 30 dB over those at 1 s.
  - `a_release_is_found`: a steady tone that falls with tau 0.08 s at 1.5 s: `release(.., None)` is 0.08 +- 0.01.
  - `the_yardstick_reads_a_known_note`: 16 harmonics at 1/k, steady: levels at 1 s are `-20 log10 k` +- 0.3 dB;
    `start` under 15 ms for a 5 ms attack.
- [ ] **Step 2:** compile with `cargo test --no-run -p bf-harness --lib`.
- [ ] **Step 3:** Implement. Reuse `bf_sound::dsp::fft::Fft` for frames; plain loops for the windowed sums (only at
  grid points, so a note costs about `points x partials x window` operations).
- [ ] **Step 4:** compiles, clippy clean.
- [ ] **Step 5: Commit** `Voice Engine 2: measuring a recording`.

### Task 3: The `recipes` tool

**Files:** Create: `bf-harness/examples/recipes.rs`

**Interfaces:**
- Consumes: Task 1, Task 2, and (for `check`) Task 5's `engine2::player::render`.
- Produces the commands (usage at the top of the file, as `references.rs` does):

  ```text
  cargo run --release -p bf-harness --example recipes -- fetch <set>
  cargo run --release -p bf-harness --example recipes -- study <PIANO|ORGAN>
  cargo run --release -p bf-harness --example recipes -- measure <PIANO|ORGAN>
  cargo run --release -p bf-harness --example recipes -- check <PIANO|ORGAN>
  cargo run --release -p bf-harness --example recipes -- reference <PIANO|ORGAN>
  ```

  - The sets, in one table in the file: `vcsl` (github.com/sgossner/VCSL, CC0), `vsco` (github.com/sgossner/VSCO-2-CE,
    CC0), `salamander` (github.com/sfzinstruments/SalamanderGrandPiano, CC BY 3.0, study only), `headroom`
    (github.com/sfzinstruments/BengtNilsson.HeadroomPiano, CC BY 4.0, study only), `iowa-piano` (University of Iowa
    MIS piano, "for any projects, without restrictions", study only, PC), `palo-burea` and `palo-pitea` (Lars Palo's
    GrandOrgue sets on sourceforge.net/p/ourorgan, CC BY-SA 2.5, study only, PC). Each with its licence, which files,
    and whether it may ship.
  - `fetch`: GitHub sets by `git clone --filter=blob:none --no-checkout` + `git sparse-checkout set --no-cone <paths>`
    + `git checkout` into `studio/reference/.cache/recordings/<set>/` (never `git ls-tree -l`); web sets by `curl -L`
    of each listed file. A set the network refuses prints "not reachable here: fetch it on the PC" and exits 2.
  - `study`: every fetched set of the instrument measured with `yard` per note and strength; writes
    `studio/reference/.cache/study/<NAME>.json` and prints, per pitch region (low, middle, high), each source's
    brightness early and late, partial 1's fall, start, noise early, and level by strength; then the spread (min and
    max over the sources) and each source's distance from the median. A source missing (not fetched) is listed as
    missing, not skipped silently.
  - `measure`: the shipped set only (PIANO: VCSL Steinway B `NoSus` `vl2`..`vl4`, every recorded pitch from A#0 to
    G#7, layers at strengths 0.45, 0.7, 0.95; release time constants from the `Rel` files; ORGAN: VSCO `Loud`
    `Rode_Man3Open_*` and `Rode_Pedal_*` as layer `loud` (strength 1.0), `Quiet` `NT5_Man3Quiet_*` and
    `NT5_PedalQuiet_*` as `quiet` (0.5), MIDI from the measured pitch, a manual note preferred where a pedal note has
    the same pitch); each layer's `gain_db` from the study's consensus level by strength (PIANO) or the recordings'
    own steady levels (ORGAN, quiet against loud, held within -12 to 0 dB); for PIANO, when a layer's brightening
    against the next (centroid early, per register) falls outside the study's spread, that layer's partials are tilted
    by dB per octave of partial number to the study's median and the tilt is printed (the design: the tone's soft to
    loud brightening comes from what the other pianos agree on); writes `bf-content/data/voices/<NAME>.ve2`
    and prints its size. Options per instrument in the file: PIANO `Opts { stretched: true, sub: false,
    max_partials: 64, end: 6.0, floor_db: -70.0 }`, ORGAN `Opts { stretched: false, sub: true, max_partials: 140,
    end: 1.6, floor_db: -70.0 }`. If a file is over 1.5 MB, lower `max_partials` or `end`, rerun, and say so in the
    commit.
  - `check`: each recipe note rebuilt by the engine at its recorded pitch and strength (held 4 s, released, cap 6 s)
    against its recording's `yard`: prints per note the worst partial level difference at each of `YARD_TIMES`, the
    start, stop and noise differences, and the worst overall. This is what sets the test thresholds in Task 11.
  - `reference`: writes `bf-harness/tests/reference/engine2/<NAME>.json`: `{"recordings": [{midi, strength, yard}...],
    "spread": [{midi_lo, midi_hi, bright_early: [min, max], bright_late: [..], fall: [..], noise_early: [..]}]}`
    from the measured set and the study.
  - The tool reads ffmpeg from `BF_FFMPEG`, else `ffmpeg` on the PATH.

- [ ] **Step 1:** Write the example with `fetch` and `study` (the engine is not needed yet).
- [ ] **Step 2:** `cargo build --release -p bf-harness --example recipes`.
- [ ] **Step 3: Commit** `Voice Engine 2: the recipes tool (fetch, study)`. `measure`, `check` and `reference` are
  added in Task 5 and Task 11, once their parts exist.

### Task 4: Study the piano and the organ; the instrument sheets

**Files:** Create: `docs/studio/voice-engine-2-instruments.md`, `docs/studio/voice-engine-2-sources.md`

- [ ] **Step 1 (cloud):** `recipes fetch vcsl`, `fetch vsco`, `fetch salamander`, `fetch headroom`;
  `recipes study PIANO`, `recipes study ORGAN` (VCSL's Renaissance organ 4', 8' and Full also go into ORGAN's study:
  another real organ's speech and wind).
- [ ] **Step 2 (PC):** one Remote Control session in `F:\!Ai Stuff\InstrumentBattleMachine` (env
  `env_01SsVTyeC4g2Wj2T6dkcBj8d`), on this branch, sharing the main build folder: `fetch iowa-piano`, `fetch palo-burea`,
  `fetch palo-pitea`, `study PIANO`, `study ORGAN`; it commits and pushes only `studio/reference/.cache`-free outputs:
  the study tables pasted into the instrument sheets (no recordings). If the PC can't be reached, carry on and run this
  step before the listening page (Task 15 waits for it).
- [ ] **Step 3:** Read the published acoustics for each (web search; for example Fletcher and Rossing, *The Physics of
  Musical Instruments*; the two-stage decay of coupled piano strings; hammer felt and brightness; organ pipe speech,
  chiff and wind). Write each instrument's sheet: working parts; how the tone changes with strength and pitch; start,
  life and stop; noises; how it is played; the spread across sources with each source named; where the literature and
  the measurements disagree (the measurements win, and the sheet says so). Each claim names its source.
- [ ] **Step 4:** Write `voice-engine-2-sources.md`: every set, author, licence, URL, what was measured from it, and
  whether it ships (only VCSL and VSCO do in stage 1).
- [ ] **Step 5:** Note in the PIANO sheet the consensus to carry into the recipes: the level step and the brightness step
  between the three strengths, and the damper's time constant by register (from VCSL's `Rel` files, checked against
  the others' release times).
- [ ] **Step 6:** When the PC part of the study arrives after Task 5, run `recipes measure PIANO` again; if a layer's
  gain or tilt moves by more than 0.5 dB, commit the new recipe with the change in the message.
- [ ] **Step 7: Commit** `Voice Engine 2: the piano and organ sheets and the sources`.

### Task 5: The player

**Files:** Create: `bf-sound/src/voice/engine2/player.rs`, `bf-sound/src/voice/engine2/play.rs`; Modify:
`engine2/mod.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces:
  - In `play.rs` (every field documented for the composition system):
    ```rust
    pub struct Play { pub midi: f64, pub dur: f64, pub vel: f64, pub accent: f64, pub slur_from: Option<f64>,
        pub bend: Option<Bend>, pub swell: Option<Swell>, pub vibrato: Option<Vibrato>, pub pedal: bool,
        pub mute: Mute, pub together: u8 }
    pub struct Bend { pub cents: f64, pub at: f64, pub over: f64 }        // slide, bend, fall or scoop by sign/time
    pub struct Swell { pub from: f64, pub to: f64, pub over: f64 }       // strength from -> to over `over` s
    pub struct Vibrato { pub cents: f64, pub hz: f64, pub wait: f64 }
    pub enum Mute { Open, Muted, Choked, Clipped }
    impl Play { pub fn plain(midi: f64, dur: f64, vel: f64) -> Play; pub fn key(&self) -> Vec<u64> }
    ```
    (`plain`: accent 0, no slur, bend, swell or vibrato, no pedal, `Mute::Open`, `together` 1; `key`: every field's
    bits, `-0.0` as `0.0`, for the note cache.)
  - In `player.rs`:
    - `pub struct Shaping<'a> { pub strength: &'a dyn Fn(f64) -> f64, pub pitch_cents: &'a dyn Fn(f64) -> f64,
      pub level_db: &'a dyn Fn(f64) -> f64, pub stop: Option<(f64, f64)>, pub carry: Carry }` (strength over time;
      pitch and level changes over time; the stop: `(at, time constant)`; how a note past the recipe goes on)
    - `pub enum Carry { Decay, Steady }`
    - `pub fn render(set: &RecipeSet, midi: f64, s: &Shaping, len: usize, variant: u64, seed: u64) -> Vec<f64>`
  - Algorithm (the module notes say it in words):
    - Strength is read every 64-sample block; the four corners (`RecipeSet::nearest`) give each partial's level by
      bilinear interpolation in dB (and cents linearly), at the block's time on the grid (linear in dB between
      points). A corner missing a partial counts as silent for it.
    - Partial k sounds at `midi_hz(midi) * ratio_k`, with `ratio` interpolated between the pitches' notes (so the
      piano's stretch follows the register), times the pitch shaping. A partial at or above 0.45 x 48,000 Hz is not
      made. A partial under the block's strongest by 70 dB is skipped for that block, and one that has been 90 dB
      under for 0.2 s is stopped.
    - Oscillators: a complex phasor per partial, rotated per sample and renormalised per block; the amplitude moves
      linearly across each block.
    - Noise: 24 seeded normal streams, each through `butter_sos(Bandpass, band edges, 2, 48 kHz)`, scaled to unit RMS
      and by the band's level at the block (bilinear as the partials).
    - Past the grid's end: `Carry::Decay` continues each partial and band at its slope over the last 1.5 s of the grid
      (never rising); `Carry::Steady` holds its mean level over the last 0.5 s.
    - The stop: after `at`, every partial and band falls with the time constant.
    - Variant 0 is the note as measured. Variant v > 0 (seeded from `seed ^ (v << 32)`): strength +-0.04, brightness
      +-0.6 dB per octave of partial number, tuning +-1.5 cents, its own starting phases (all variants' phases are
      seeded, Schroeder-spread for v = 0) and its own noise streams.
    - The result is `len` samples, mono, finite.
- [ ] **Step 1: Write the failing tests** (in `player.rs`, on a small synthetic `RecipeSet` built in the test):
  - `a_recorded_note_is_rebuilt_at_its_levels`: a set of one note, 8 partials at known steady levels: the render's
    partial levels at 0.5 s are within 0.3 dB of the recipe's.
  - `between_recordings_the_levels_are_interpolated`: notes at 48 and 54 whose partial 2 is -6 and -18 dB: a note at 51
    has it at -12 +- 0.5 dB.
  - `nothing_is_made_above_0_45_of_the_rate`: a 4 kHz note with partials to ratio 8: no energy over 21.6 kHz
    (FFT, -100 dB).
  - `pitches_past_the_recordings_take_the_nearest_tone` (Review Focus 2): MIDI 12 and 120 on a set of 36 to 96 give
    finite samples with the nearest note's partial balance.
  - `a_note_longer_than_its_recipe_carries_on` (Review Focus 3): grid to 1.6 s, render 4 s: `Steady` keeps its level
    within 1 dB from 1.5 to 4 s; `Decay` keeps falling at the last slope +- 1 dB/s; neither repeats a block exactly.
  - `the_stop_falls_with_its_time_constant`: stop at 0.5 s, tau 0.07: 0.2 s later the level is -24.8 +- 1 dB.
  - `repeats_differ_a_little`: variants 0, 1, 2 all differ (RMS of the difference over 2% of the note's), and each is
    within 1.5 dB of variant 0's loudness.
  - `the_same_note_is_the_same_everywhere` (Review Focus 4): the same render twice, and from two threads, is equal
    sample for sample.
- [ ] **Step 2:** compile (`cargo test --no-run -p bf-sound`).
- [ ] **Step 3:** Implement `play.rs` and `player.rs`.
- [ ] **Step 4:** Add `measure` and `check` to the `recipes` tool; run `recipes measure PIANO` and
  `recipes measure ORGAN` (cloud: VCSL and VSCO are on GitHub), check both files are at most 1.5 MB, run
  `recipes check PIANO` and `check ORGAN` and keep the printout for Task 11.
- [ ] **Step 5: Commit** `Voice Engine 2: the player and the first recipes` (adds `bf-content/data/voices/*.ve2`), with
  `PC-Tests: -p bf-sound engine2` and `PC-Tests: -p bf-harness --lib e2measure`; push and read the PC run.

### Task 6: The piano's working parts

**Files:** Create: `bf-sound/src/voice/engine2/piano.rs`; Modify: `engine2/mod.rs`

**Interfaces:**
- Consumes: Task 5.
- Produces: `pub struct PianoParts { pub undamped_from: i32, pub halo_db: f64 }` and
  `pub fn note(set: &RecipeSet, p: &PianoParts, play: &Play, len: usize, variant: u64) -> Vec<f64>`:
  - strength = `play.vel` + 0.12 x `play.accent`, clamped to 0.2 to 1.1 (above the top layer, the top layer with
    +1 dB per octave of partial number for each 0.1 over); an accent also lifts the noise 3 dB x accent over the first
    50 ms;
  - loudness by strength: the layers' `gain_db` (the study's consensus), interpolated;
  - the damper at `play.dur` with the recipe's `release` (interpolated by pitch) for MIDI under `undamped_from` (91:
    the top 18 keys ring on); with `play.pedal`, no damper and a halo: partials 2 to 6 of the note also ring from the
    open strings, `halo_db` (-34) under the note's partial at 0.3 s, rising over 0.15 s, falling at the note's late
    slope;
  - `Carry::Decay`; the seed is the MIDI note's.
- [ ] **Step 1: Write the failing tests** (in `piano.rs`, on the real recipes,
  `include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../bf-content/data/voices/PIANO.ve2"))`):
  - `the_damper_stops_a_note_at_key_up`: C4, `dur` 0.3: 0.4 s after key-up it is 40 dB under where it was at key-up;
    C7 (MIDI 96, undamped) is under 12 dB lower at the same point.
  - `the_pedal_keeps_it_ringing`: C4 `dur` 0.3 with the pedal: at 1.5 s within 6 dB of a key held 3 s.
  - `harder_is_louder_and_brighter`: vel 0.5 against 1.0 at C4: louder by the layers' step +- 1 dB, centroid higher.
  - `an_accent_starts_harder`: the first 50 ms of an accented note carry more noise than the plain one.
- [ ] **Step 2:** compile. **Step 3:** implement. **Step 4:** compile, clippy.
- [ ] **Step 5: Commit** `Voice Engine 2: the piano's damper, pedal and strength`.

### Task 7: The organ's working parts

**Files:** Create: `bf-sound/src/voice/engine2/organ.rs`; Modify: `engine2/mod.rs`

**Interfaces:**
- Consumes: Task 5.
- Produces: `pub struct OrganParts { pub wind_cents: f64, pub wind_db: f64, pub wind_hz: f64, pub sag_cents: f64,
  pub release: (f64, f64) }` and `pub fn note(set: &RecipeSet, p: &OrganParts, play: &Play, len: usize, variant: u64)
  -> Vec<f64>`:
  - registration as strength: 1.0 (the loud chorus) for an ordinary note; with `play.swell`, strength goes from
    `swell.from` to `swell.to` over `swell.over` (smoothstep), which crossfades the quiet chorus into the loud one tone
    by tone (Aedan's "building up");
  - level by `play.vel`, linearly, as today's organ;
  - the wind: one slow waver shared by every organ note (a fixed seed, not the variant's: notes that start together
    waver together), +-`wind_cents` (1.5) and +-`wind_db` (0.25), band-limited round `wind_hz` (0.7 Hz);
  - the sag: with `play.together` of 3 or more, the pitch sinks `min(sag_cents, 0.8 (together - 2))` cents (4 at most)
    and the level 0.3 dB, easing in over 60 ms;
  - the stop at `play.dur` with the recipe's release clamped to `release` (0.03 to 0.12 s): the pipes stop; the
    recording's church tail is not used (the light home room of Task 10 gives the echo);
  - `Carry::Steady`.
- [ ] **Step 1: Write the failing tests** (in `organ.rs`, on the real recipes, included the same way as Task 6's):
  - `a_swell_builds_from_the_quiet_chorus_to_the_loud`: C4 held 2 s with swell 0.5 to 1.0 over 1.5 s: its first
    200 ms match the quiet layer's partial balance (+-1.5 dB), its last 300 ms the loud layer's.
  - `the_pipes_stop_quickly`: `dur` 0.5: 0.4 s after the release it is 30 dB down or more.
  - `the_wind_wavers_slowly_and_together`: two notes rendered alone share their waver (pitch track correlation over
    0.9 between 0.2 and 2 s); the waver's depth is under 2 cents.
  - `a_big_chord_sags_a_hair`: `together` 6 against 1: the pitch 4 +- 0.5 cents lower after 100 ms.
- [ ] **Step 2:** compile. **Step 3:** implement. **Step 4:** compile, clippy.
- [ ] **Step 5: Commit** `Voice Engine 2: the organ's registration, wind and stop`.

### Task 8: Engine 2 in the score

**Files:** Create: `bf-sound/src/voice/engine2/space.rs` (the two structs only; Task 10 adds what they do). Modify:
`bf-sound/src/voice/mod.rs`, `bf-sound/src/voice/engine2/mod.rs`, `bf-sound/src/music/mod.rs`,
`bf-sound/src/music/tests.rs`

**Interfaces:**
- Consumes: Tasks 5 to 7.
- Produces:
  - In `engine2/space.rs`: `#[derive(Clone, Copy, Debug)] pub struct Space { pub width: f64, pub tilt: f64, pub home:
    Option<Home> }` and `#[derive(Clone, Copy, Debug)] pub struct Home { pub length: f64, pub under: f64, pub hp: f64 }`
    (width: how wide the voice spreads, 0 none; tilt: how far its bass leans left and its treble right; home: its own
    room, `length` s long, `under` LU under the dry sound, high-passed at `hp` Hz).
  - In `engine2/mod.rs`: `pub enum Parts { Piano(PianoParts), Organ(OrganParts) }`;
    `pub struct Instrument { pub name: &'static str, pub recipes: &'static [u8], pub parts: Parts, pub space: Space,
    set: OnceLock<RecipeSet> }` with `pub const fn new(name, recipes, parts, space) -> Instrument` and
    `pub fn set(&self) -> &RecipeSet` (parsed once; an unreadable embedded file panics with its name and the
    `RecipeError`, which a test rules out); `pub fn play(i: &Instrument, p: &Play, cap: f64, variant: u64) -> Vec<f64>`
    (`len` = `int(cap * SR)` when cap > 0, else the natural length to the note's fall 70 dB under its peak, at most
    12 s).
  - In `voice/mod.rs`: `fn engine2(&self) -> Option<&'static engine2::Instrument> { None }` on `Voice`, documented.
  - In `music/mod.rs`:
    - `note_in` under `composed_on()` for a voice with `engine2()`: repeats take turns among `real::TURNS` variants
      (as for a reworked voice), the key also holds `Play::key()`, and the note is `engine2::play(i, &play, cap,
      variant)` times the voice's level match, then `fit` and `inst_gain` as today. Under Classic, or for any other
      voice, `note_in` is unchanged sample for sample.
    - `pub fn note_played(&self, name: &str, play: &Play, cap: f64) -> Result<Arc<[f64]>, MusicError>`: under
      Classic, or for a voice without engine 2, exactly `self.note(name, play.midi, play.dur, cap, play.vel)` (same key,
      same samples); else the engine 2 note of `play`. `note` itself plays `Play::plain` on the engine 2 path.
    - The level match (State gains a `levels2: BTreeMap<String, f64>`): once per voice per build, the RMS of the old
      voice's reference note (`voice.note` on a CLONE of the noise, so the stream is untouched; the middle of its
      register, 0.8, 0.2 s; first 0.3 s, high-passed at 120 Hz) over the same of the engine 2 note
      (`Play::plain(middle, 0.2, 0.8)`, cap 0.6, variant 0).
    - `pub fn engine2_of(&self, name: &str) -> Option<&'static engine2::Instrument>`: the voice's instrument under
      Composed, `None` under Classic.
- [ ] **Step 1: Write the failing tests** (in `music/tests.rs`, with the existing `testkit` and a test voice whose
  `engine2()` is an `Instrument` over a small synthetic recipe, leaked to `'static`):
  - `classic_never_plays_engine2`: under Classic the test voice's notes equal its `note` through `fit`, sample for
    sample, and `note_played` with a swell equals `note`.
  - `composed_plays_engine2_and_takes_turns`: under Composed four asks of one note give variants 0, 1, 2, 0 (the
    fourth the first, the same `Arc`).
  - `a_note_shorter_than_its_attack_fits_its_cap` (Review Focus 1): caps 0.05, 0.11 and 0.33 give exactly
    `int(cap * SR)` finite samples, ending at zero.
  - `an_engine2_voice_keeps_the_old_voices_level` (Review Focus 5): the reference note's high-passed RMS over its
    first 0.3 s through `note` equals the old voice's within 0.1 dB.
  - `the_noise_stream_is_untouched_by_the_level_match`: the shared stream's next normals are the same with and
    without an engine 2 voice's first note.
- [ ] **Step 2:** compile. **Step 3:** implement. **Step 4:** compile, clippy.
- [ ] **Step 5: Commit** `Voice Engine 2: engine 2 notes in the Composed score`.

### Task 9: The piano and the organ move to Voice Engine 2

**Files:** Modify: `bf-content/src/fighters/piano/voice.rs`, `bf-content/src/fighters/organ/voice.rs`,
`bf-content/src/fighters/organ/sound.rs`, `bf-harness/tests/real_voices.rs`, `bf-harness/examples/references.rs`

**Interfaces:**
- Consumes: Task 8.
- Produces:
  - `pub static ENGINE2: Instrument = Instrument::new("PIANO", include_bytes!("../../../data/voices/PIANO.ve2"),
    Parts::Piano(PianoParts { undamped_from: 91, halo_db: -34.0 }), Space { width: 0.35, tilt: 0.12, home: None });`
    in `piano/voice.rs`, and `PianoVoice::engine2()` returning it. The stage 3 `FILL`, `CORRECTION`, `REWORK` and the
    `rework()` override are removed (Classic never used them).
  - `organ/voice.rs` likewise: `Instrument::new("ORGAN", include_bytes!(".../ORGAN.ve2"), Parts::Organ(OrganParts {
    wind_cents: 1.5, wind_db: 0.25, wind_hz: 0.7, sag_cents: 4.0, release: (0.03, 0.12) }), Space { width: 0.45,
    tilt: 0.0, home: Some(Home { length: 1.8, under: 14.0, hp: 200.0 }) })`; its rework tables removed.
  - `organ/sound.rs`: `chord` plays each note through `ctx.note_played` with `together` = the chord's size (so Classic,
    where `note_played` is `note`, is unchanged); a new `swell: Option<Swell>` argument on `chord`: CATHEDRAL passes
    `Swell { from: 0.5, to: 1.0, over: 1.5 * beat }`, PULL ALL STOPS each note `Swell { from: 0.5, to: 1.0, over:
    its dur }`, the title blast `Swell { from: 0.6, to: 1.0, over: 0.45 }`; every other call passes `None`.
  - `real_voices.rs` and `references.rs`: PIANO and ORGAN out of the stage 3 lists (they now have
    `engine2_voices.rs`).
- [ ] **Step 1:** Make the changes; `grep -rn "piano::voice::REWORK\|organ::voice::REWORK\|piano::voice::CORRECTION\|organ::voice::CORRECTION"`
  finds nothing left.
- [ ] **Step 2:** Add to `bf-content` (in `piano/voice.rs` and `organ/voice.rs` tests) `the_recipes_load_and_fit_the_budget`:
  `ENGINE2.set()` parses, has the expected layers (PIANO: 3; ORGAN: `quiet`, `loud`) and covers the fighter's
  register; the embedded file is at most 1,500,000 bytes.
- [ ] **Step 3:** `cargo build --release -p bf-cli`; render `ballfight sound fights/piano_vs_organ.toml --arrangement
  composed --out /tmp/claude-0/ve2/s1_after.wav` and check it is not silent and has no clipping report.
- [ ] **Step 4: Commit** `Voice Engine 2: PIANO and ORGAN play from their recipes (Composed)`.

### Task 10: Width and the organ's light room

**Files:** Modify: `bf-sound/src/voice/engine2/space.rs`, `bf-sound/src/polish/mod.rs`,
`bf-sound/src/polish/room.rs`, `bf-content/src/soundtrack.rs`

**Interfaces:**
- Consumes: Task 8 (`MusicCtx::engine2_of`), Task 9's `Space` values.
- Produces (in `space.rs`, beside Task 8's `Space` and `Home`):
  - `pub fn widen(x: &[[f64; 2]], width: f64, tilt: f64, seed: u64) -> Vec<[f64; 2]>`: adds `+d` to the left and `-d`
    to the right, so left plus right is unchanged (most phones play mono); `d = width x side + tilt x (low - high)`,
    where `side` is the mid through four seeded all-passes (Schroeder, 3 to 11 ms) high-passed at 250 Hz, `low` the mid
    under 250 Hz and `high` the mid over 2 kHz (2nd-order Butterworth): the bass a little left, the treble a little
    right.
  - `pub fn home(x: &[[f64; 2]], h: &Home, seed: u64) -> Vec<[f64; 2]>`: `x` plus its own room: `room::wet_with(x, seed,
    h.length, h.length * 0.55)` high-passed at `h.hp`, `h.under` LU under `x` (`heard::integrated`).
  - `room::wet_with(send, seed, length, fade) -> Vec<[f64; 2]>`; `room::wet` becomes `wet_with(send, seed, LENGTH,
    FADE)`, sample for sample.
  - `polish::Part` gains `pub space: Option<Space>`; `treated` applies `home` then `widen` (seed: the bus name's bytes
    summed) to a part's bus, and to the solo bus when the part is the winner, after its gain and before the M2/M3
    treatments. `soundtrack.rs` `plan()` fills it from `ctx.engine2_of(&cfg[k].pp).map(|i| i.space)`.
- [ ] **Step 1: Write the failing tests** (in `space.rs` and `polish/mod.rs` tests):
  - `widening_keeps_the_mono_sum`: for a random stereo bus, `L + R` after equals before within 1e-12, and the side
    energy is over 1% for width 0.35; width 0 and tilt 0 return the bus unchanged.
  - `the_tilt_puts_the_bass_left_and_the_treble_right`: a 100 Hz tone ends louder left, a 4 kHz tone louder right.
  - `the_home_room_is_light_and_high`: an organ chord's room is `under` +- 1 LU under it, has under 5% of its energy
    below 200 Hz, and is 60 dB down by `length` + 0.3 s.
  - `the_shared_room_is_unchanged`: `room::wet` equals its value before the change on a fixed input (bits).
  - `a_part_without_space_is_untouched`: `treated` with `space: None` equals today's output.
- [ ] **Step 2:** compile. **Step 3:** implement. **Step 4:** compile, clippy.
- [ ] **Step 5: Commit** `Voice Engine 2: width for the new voices and the organ's light room (Composed)`.

### Task 11: The yardstick

**Files:** Create: `bf-harness/tests/engine2_voices.rs`, `bf-harness/tests/reference/engine2/PIANO.json`,
`ORGAN.json`; Modify: `bf-harness/examples/recipes.rs` (`reference`)

**Interfaces:**
- Consumes: Task 2's `yard`, Task 3's `reference` output, Task 9's instruments.
- Produces the test file, with thresholds from Task 5's `check` printout (the design's 2 to 3 dB per overtone;
  write the chosen numbers as named constants with the printout's worst values in their comments):
  - `every_recorded_note_matches_its_recording_over_time`: for each committed recording, the engine 2 note at that
    pitch and strength (held 4 s, then released; cap 6 s) has each of the first 16 partials within `PARTIAL_DB` at each
    of `YARD_TIMES` (partials the recording has within 50 dB of its strongest), `start` within `START_S`, `stop` within
    `STOP_S`, the noise shares within `NOISE_SHARE`.
  - `every_note_sits_inside_the_real_instruments_spread`: across the fighter's register in whole tones, brightness
    early and late, the fall and the early noise lie within the committed spread widened by `SPREAD_MARGIN`.
  - `each_note_is_nearer_its_own_instrument_than_its_rivals`: the `real_voices.rs` rule on the engine 2 notes (FluidR3
    fingerprints: PIANO against harp, marimba and electric piano; ORGAN against accordion, reed organ and trumpet).
  - `repeats_differ_within_bounds`: variants 0 to 2 of C4 and of each register's ends.
- [ ] **Step 1:** Add `reference` to the tool; run `recipes reference PIANO` and `reference ORGAN` (needs Task 4's
  study, PC parts included when available).
- [ ] **Step 2:** Write the test file. **Step 3:** compile (`cargo test --no-run -p bf-harness --test engine2_voices`).
- [ ] **Step 4: Commit** `Voice Engine 2: the yardstick for PIANO and ORGAN`.

### Task 12: Levels and speed

**Files:** `bf-content/src/levels_table.rs` (by the tool)

- [ ] **Step 1:** `cargo run --release -p bf-cli -- levels fights/dev/bold_piano.toml fights/dev/bold_organ.toml --write
  crates/bf-content/src/levels_table.rs`, run twice (the second settles the gain, as the command's notes say); only the
  PIANO and ORGAN rows change.
- [ ] **Step 2:** Speed: `ballfight sound` with `--arrangement composed` for `fights/piano_vs_organ.toml`,
  `fights/dev/bold_piano.toml` and `fights/dev/bold_organ.toml`, three runs each with the before binary (Task 0) and
  the new one, same machine; the median ratio of each is at most 1.25. Write the numbers in the commit message. Over
  1.25: profile (`perf` or timing prints around `engine2::play`), lower the partial floor or block cost, and measure
  again.
- [ ] **Step 3: Commit** `Voice Engine 2: PIANO and ORGAN levels` with the speed table.

### Task 13: The playing doc, the credits, the handoff

**Files:** Create: `docs/studio/voice-engine-2-playing.md`; Modify: `studio/packaging/guide/index.html`,
`docs/studio/HANDOFF.md`, `docs/studio/2026-11-26-voice-engine-2-design.md` (only if something was decided
differently, in a short "Changed in stage 1" note)

- [ ] **Step 1:** `voice-engine-2-playing.md`: every `Play` field in plain words, which voices take it, what today's
  parts pass automatically, and a few lines of how the composition system would ask for a phrase. For Aedan's
  composition design.
- [ ] **Step 2:** The guide's last section, "About the sounds": the piano and organ are rebuilt from free recordings
  (VCSL by Versilian Studios, VSCO 2 CE by Versilian Studios and Simon Dalzell of Ivy Audio), thanked by name; no
  credit is needed. Check any guide test (`grep -rn "guide/index.html" studio/crates`) still passes its rules.
- [ ] **Step 3:** HANDOFF: a short paragraph: what Voice Engine 2 is, where its code, recipes, tool and tests live, the
  rule that shipped recipes come only from CC0 or free-for-any-use sets, and how a later stage adds an instrument
  (study, sheet, measure, check, reference, voice, levels).
- [ ] **Step 4: Commit** `Voice Engine 2: the playing doc, the guide's credits, the handoff`.

### Task 14: The PC run

- [ ] **Step 1:** The last commit's message carries:

  ```text
  PC-Tests: -p bf-sound engine2
  PC-Tests: -p bf-sound music::
  PC-Tests: -p bf-sound polish
  PC-Tests: -p bf-harness --lib e2measure
  PC-Tests: -p bf-content fighters::piano fighters::organ
  PC-Tests: -p bf-harness --test engine2_voices
  PC-Tests: -p bf-harness --test real_voices
  PC-Tests: -p bf-harness --test classic_golden
  PC-Tests: -p bf-harness --test fighter_piano
  PC-Tests: -p bf-harness --test fighter_organ
  PC-Tests: -p bf-harness --test episode_piano_vs_organ
  PC-Tests: -p bf-harness --test composed_build
  PC-Tests: -p bf-harness --test bands
  PC-Tests: -p bf-harness --test theme_mambo
  PC-Tests: -p bf-harness --test composed_horror_to_mambo
  PC-Tests: -p bf-harness --test composed_noir_to_villain
  PC-Tests: -p bf-harness --test theme_quick_phonk_noir
  ```

- [ ] **Step 2:** Push; read the `pc-tests` run. Re-run only what fails while fixing it (the project's rule).
- [ ] **Step 3:** All green: go on. A Classic test that fails means engine 2 leaked into Classic: find where, never
  change the golden.

### Task 15: The listening page, then stop

- [ ] **Step 1:** Render, for PIANO and ORGAN: the riffs of the examples page (today = this branch's parent's
  Composed voice, new = engine 2, real = the recording), a stretch of a real fight (`fights/piano_vs_organ.toml`,
  Composed, before and after, cut `-ss 10 -t 22`), the mambo and noir jazz band pianos (a short stretch of each theme,
  before and after), all at -16 LUFS, each with a phone version (two 350 Hz high-passes, as stage 1's pages) and a
  spectrogram with a playhead; show the organ's CATHEDRAL and PULL ALL STOPS building up.
- [ ] **Step 2:** Publish the page (the earlier pages' Ball Studio look); put its source under
  `/mnt/project-files/ballfight-studio/voice-engine-2/stage1/`.
- [ ] **Step 3:** Post the page in the thread with what to listen for, in plain words, and **stop**: the next family
  starts only after Aedan has reviewed this one. Fix his notes in this stage, re-render, and post again.
- [ ] **Step 4:** After his review and green PC tests: mark PR #29 ready, and merge only on his go (or as he says).
  Then `powershell -ExecutionPolicy Bypass -File studio\scripts\clean-builds.ps1 -Apply` on the PC per HANDOFF.
