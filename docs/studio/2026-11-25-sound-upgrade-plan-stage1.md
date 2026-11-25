# Sound upgrade, stage 1 (mix, polish, melodies, bari sax) Implementation Plan

> **For agentic workers:** implement task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Under the Composed arrangement, the fighters sit 5 dB over the band, the master is polished to -14 LUFS, every pitched fighter plays a written melody in one of six styles, and the bari sax no longer slides. Classic does not change by one sample.

**Architecture:** Everything new hangs off one value, `MusicCtx::polish: Option<Polish>`, which is `Some` only when `ff.music.arrangement == themes::COMPOSED`. Fighter parts ask the context for their note (`ctx.play_line`) and place ordinary notes through `Mix::note`; both fall back to today's behaviour when `polish` is `None`. The Composed master is a new function beside the classic one (`master::master_composed`), built from small pure filters in a new `bf_sound::polish` module and measured by a new `bf_sound::heard` module.

**Tech stack:** Rust 1.99.1, the `studio/` workspace, libm soft floats (no new crates).

**Spec:** `docs/studio/2026-11-25-sound-upgrade-design.md`. Stages 2 (bands) and 3 (instruments) get their own plans when stage 1 is merged.

## Global constraints

- Composed only. A fight on Classic must render the same bytes as on the build branch (Task 1 pins this before any other change).
- No check stops a Composed render. Checks fix or note.
- No `cargo test` in the cloud. Tests run on Aedan's PC from `PC-Tests:` lines in the last commit message, only for what changed. Rendering with the release binary in the cloud is allowed (that is how hashes, the level table and listening pages are made).
- Gates before every commit, run in `studio/`: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace --all-targets`; `git status --short projects reelkit series` must print nothing.
- Never commit anything from `sound-review/tools/` (the spikes are reference only). No environment-variable switches in shipped code.
- Results must be the same on every machine: only `libm`/`bf_core` maths, no `HashMap` iteration, no statics, no threads inside the new audio code.
- Do not install.

## Review focus

1. A fight file written before this change (no `melody_style`) on Composed: it must load and play the Signature style.
2. A Composed fight where one fighter is unpitched (snare, target dummy): harmony pass, no-repeat walk and M1 trim must skip it without panicking.
3. A fight with no K.O. inside the loop, or a silent fighter stem: `over_band` returns `None` and the trim is 0 dB.
4. Two notes placed at the same instant on a one-voice part (a chord): one event, neither fades the other.
5. A fight file that names a melody style the theme does not list: it plays that style anyway (the file wins; the menu only suggests).

Each has a unit test in the task that owns the code (Tasks 3, 4, 6).

---

### Task 1: Lock Classic with golden hashes

**Files:**
- Create: `studio/crates/bf-testdata/src/golden.rs` (the fight list and the hash function), `studio/crates/bf-harness/tests/classic_golden.rs`, `studio/crates/bf-harness/tests/golden/classic.txt`
- Modify: `studio/crates/bf-cli/src/cmd/` (new dev command `golden`)

**Interfaces (produces):**
- `golden::fights() -> Vec<GoldenFight { theme: &'static str, a: &'static str, b: &'static str }>`: one fight per theme (62), fighters dealt round-robin from the roster so all 42 appear, each pair taken from `themes_for` so the theme is legal for it.
- `golden::hash(g: &GoldenFight) -> Result<String>`: `starter(a, b)`, theme set, arrangement `classic`, first seed in `1..=12` that reaches a K.O., `build_soundtrack`, SHA-256 over the master's `f64` little-endian bytes.
- `ballfight golden [--out FILE]`: prints `theme a b seed hash` lines.

- [ ] `golden.rs` and the CLI command; run it in the cloud on the untouched build branch code; commit the output as `classic.txt`.
- [ ] `classic_golden.rs`: for every line of `classic.txt`, `golden::hash` equals the stored hash; fails with the theme name.
- [ ] Commit alone, with `PC-Tests: -p bf-harness --test classic_golden`. Every later commit that touches sound keeps this line.

### Task 2: The heard measure

**Files:** Create `studio/crates/bf-sound/src/heard.rs`; modify `bf-sound/src/lib.rs`, `bf-export/src/checks/loudness.rs` (takes its K-weighting from `heard`).

**Interfaces (produces):**
- `heard::k_weight(x: &[f64]) -> Vec<f64>`; `heard::blocks(x: &[[f64; 2]]) -> Vec<f64>` (400 ms blocks every 100 ms, LUFS per block); `heard::integrated(x) -> f64` (gates -70 absolute, -10 relative).
- `heard::heard(x) -> Vec<f64>`: `blocks` after a 2nd-order 250 Hz high-pass.
- `heard::over_band(fighter, band, from: f64, to: f64) -> Option<f64>`: mean heard level of the fighter minus the band's over the blocks where the fighter is within 20 dB of its loudest block and above -65.
- `heard::phone_loss(x) -> f64` (integrated minus integrated after a 3rd-order 350 Hz high-pass); `heard::share(x, lo_hz, hi_hz) -> f64` (energy share by FFT of the mono sum); `heard::RUMBLE = (0.0, 200.0)`, `heard::PIERCING = (2000.0, 6000.0)`.
- `heard::true_peak(x) -> f64` (4x oversampled).

- [ ] Unit tests in the module: a 1 kHz sine at -20 dBFS reads -20 ±0.2 integrated; a 60 Hz sine has rumble share > 0.99 and phone loss > 20 dB; `over_band` of a signal against itself at -6 dB is 6 ±0.1; a silent fighter gives `None`.

### Task 3: The Composed hooks on the context and the mix

**Files:** Modify `bf-sound/src/music/mod.rs`, `bf-sound/src/mix.rs`, `bf-sound/src/traits.rs`; create `bf-sound/src/music/played.rs`.

**Interfaces (produces):**
- `pub struct Polish { pub style: Style, pub parts: BTreeMap<String, PartNote>, pub hand_bass: bool }` and `pub struct PartNote { pub one_voice: bool, pub piercing: bool, pub low: bool, pub gain: f64 }`; `MusicCtx::polish: Option<Polish>`.
- `pub enum Style { Signature, Climbing, Riff, Runs, BrokenChords, Anchor }` in `bf-sound/src/music/style.rs` with `Style::moving(self) -> bool` (Climbing, Runs, BrokenChords), `Style::key(self) -> &'static str`, `Style::from_key(&str) -> Option<Style>`, `Style::ALL`.
- `Sound::one_voice(&self) -> bool` (default `false`; `true` for wind, brass and bowed fighters).
- `ctx.play_line(name: &str, t: f64) -> i32`: `line_at` when `polish` is `None` or the style is not moving. In a moving style it returns a note that differs from the notes the same fighter sounded just before and just after `t` (looked up by time in a per-fighter `BTreeMap`, so the order parts are built in does not matter): if `line_at` repeats a neighbour, step to the next scale note in the line's direction, choosing the one that does not clash with the other fighter's line at `t` (clash = semitone, tritone or major seventh). Records the note it returns.
- `ctx.hold_line(name, t) -> i32`: the last note `play_line` returned at or before `t`, without recording (for rapid-fire effects that keep their pitch).
- `ctx.played(name) -> Vec<(f64, i32)>` for tests.
- `Mix::note(&mut self, bus: &str, sig: &Sig, t: f64, gain: f64, pan: f64, one_voice: bool)`: `place` when `one_voice` is false. Otherwise queued per bus; `Mix::settle(&mut self)` (called once at the end of `score`) sorts each queue by time, treats notes within 1 ms as one event, and fades every earlier event to silence over 30 ms from the start of the next.

- [ ] Unit tests: two queued notes 0.5 s apart, each 2 s long: the first is silent 40 ms after the second starts; two at the same instant both ring; `one_voice = false` equals `place` sample for sample. `play_line` over a line of one repeated note in a moving style never returns the same note twice running, and stays within the fighter's register; in Signature it equals `line_at`.

### Task 4: Melodies (C10, C2, C11)

**Files:** Modify `bf-content/src/composer.rs` (split styles into `bf-content/src/composer/styles.rs` if it passes ~900 lines), `bf-content/src/soundtrack.rs`, `bf-content/src/themes/mod.rs` (new `themes/family.rs`), `bf-core/src/fightfile/{mod,read,write}.rs`.

**Interfaces (produces):**
- `Music::melody_style: Option<String>` in the fight file (`music.melody_style = "climbing"`); absent means Signature. Not written when `None`, so older files round-trip unchanged.
- `family::Family { Latin, Jazz, Rock, Electronic, Orchestral, Folk, March, Hiphop, World, Game }`, `family::of(theme: &str) -> Family`, `family::styles(Family) -> &'static [Style]` (3 to 6 each, Signature always first). Stage 2 reuses `Family`.
- `composer::pick_style(theme: &str, music_seed: u64) -> Style`; `soundtrack::style_of(ff) -> Style` (the file's field, else Signature).
- `composer::compose(p, role, side, h, signal, music_seed, style)`:
  - **C10, all styles:** in each 4-bar phrase, bar 1 is the motif, bar 3 is the answer (the motif's shape turned upside down from the chord tone nearest its first note; mirrored left to right on every other pass of the loop), bar 4 runs by step into the chord tone nearest the motif's first note, bar 2 walks by step with at most one favourite leap, which is turned back by step.
  - **Styles:** Signature = the above. Climbing = each bar starts a scale step above the last and climbs, falling back at the phrase end. Riff = one bar-long figure repeated and moved to each chord. Runs = scale runs in 8ths that turn at the register edges. Broken chords = chord tones in up, down, up-down order by bar. Anchor = returns to one home note on every strong 8th with moving notes between.
  - Moving styles write no two equal notes in a row inside the line (the play-time walk covers repeats caused by rests and effects).
- `composer::harmonise(giver: &mut [Option<i32>], other: &[Option<i32>], per_bar: usize, scale: &[i32], register: (i32, i32))` (C2): per 8th slot where both sound, a semitone, tritone or major seventh (or a whole tone on 8ths 1 and 5) moves the giver to the nearest scale note within 4 semitones, in its register, that makes a unison, third, fifth, sixth or octave; ties go to the note closer to the giver's previous note. The giver is the melody when the other is the bass, else the lower register.
- `compose_into` fills `ctx.polish` (style, each fighter's `PartNote`, `hand_bass`).

- [ ] Unit tests: for every fighter and every moving style, 3 seeds: no equal neighbours, all notes in register and scale. Climbing's bar starts rise within a phrase. Riff's bars share one interval shape. Anchor has its home note on at least half the strong 8ths. After `harmonise`, no slot holds a clash. `melody_style` round-trips; a file without it reads `None`; an unknown word reads as Signature with no error.

### Task 5: Fighter parts

**Files:** `bf-content/src/fighters/<slug>/sound.rs` for the 16 fighters below, the one-voice fighters, `piccolo`, `bagpipes`, `violin`, `bari_sax/{sound,voice}.rs`.

- [ ] **Follow the line:** ACCORDION, ACOUSTIC, BARI SAX, BASS GUITAR, CONTRABASSOON, CORNET, HARP, MELLOPHONE, ORGAN, PIANO, SOPRANO SAX, TIMPANI, TUBA, UKULELE, SYNTH, and TRUMPET whenever a line exists: when `ctx.polish` is `Some`, ordinary notes (bounces, hits, steps) take `ctx.play_line(name, t)`; chords are built on it; signature specials keep their own notes. Classic branch untouched.
- [ ] **All other pitched fighters:** `line_at` becomes `play_line` for ordinary notes, `hold_line` for rapid-fire repeats; flute's grace note takes the scale step below in moving styles.
- [ ] **M5:** `one_voice()` true for the wind, brass and bowed fighters; their ordinary notes go through `mix.note(.., true)`.
- [ ] **M3 notes:** `polish::soften(midi) -> f64` (1.0 up to C6 = 84, then -1.5 dB per semitone, floor -9 dB) multiplies ordinary notes of piercing fighters; the composed line of a piercing fighter folds notes in the top octave of its register down an octave; piccolo bounce ticks drop an octave under Composed.
- [ ] **Bari sax:** under Composed, `doppler` keeps its swell with up = down = 1.0; scoop is `0.012` at most; the crater honk and GROWL are not bent; `fall_apart` keeps its fall.
- [ ] **ALTO SAX vs SYNTH** "sosfilt: x is empty": reproduce with the CLI, fix the empty-note case at its source.
- [ ] Test `bf-harness/tests/line_follow.rs`: for each of the 16, two different composed lines give different fighter stems, and `ctx.played(name)` is not empty; for bari sax, pitch-track the stem outside `fall_apart` and assert no note moves more than 0.5 semitone.

### Task 6: The polish chain (M1, M2, M3, F3, F2, F1, C6)

**Files:** Create `bf-sound/src/polish/{mod,lows,highs,room,bands,limit,loud}.rs`; modify `bf-sound/src/master.rs`, `bf-sound/src/music/layers.rs`, `bf-content/src/soundtrack.rs`; create `bf-content/src/levels_table.rs` (generated) and CLI dev command `ballfight levels`.

**Interfaces (produces):**
- `lows::treat(x: &mut [[f64; 2]])` (M2): overtones one and two octaves up made from the band under 300 Hz, stronger the lower the note; 38 Hz high-pass; shelf down under 110 Hz; loudness restored. Used when the fighter's saved rumble share > 0.5.
- `highs::treat(x)` (M3): 1250–6500 Hz band pulled down by `(thr/env)^0.7` once its envelope passes its 25th percentile, at most 13 dB; 7.5 kHz low-pass. Used when saved piercing share > 0.25.
- `levels_table::LEVELS: &[(name, gain, rumble, piercing)]`, written by `ballfight levels --write`: each fighter against the same reference fight (hot_jazz, Composed, tempo 140, versus TARGET DUMMY), gain solved so `over_band` = 5.0. `levels_table::of(name) -> Option<Saved>`.
- `master::master_composed(mix, levels, bed, polish, window: (f64, f64)) -> Soundtrack`, in this order: bus gains from the table; M2/M3 on flagged fighter buses; M1 trim per fighter, clamped ±6 dB, to bring `over_band` to 5.0 over FIGHT!..K.O. (the solo bus takes the winner's trim); F3 room; fold onto the loop; F2; F1.
- `room::ir(seed) -> Vec<[f64; 2]>` (F3): 20 ms predelay, taps at 17, 23, 31, 41, 53, 67, 79 ms, 1.25 s tail whose highs decay 0.45x as long, band-limited 220 Hz–7 kHz; sends 0.04 (low fighters and bass layers), 0.2 (piercing), 0.3 (others); room level set 18 dB under the dry mix.
- `bands::compress(x, stems)` (F2): split at 200 Hz and 3 kHz; ratios 3.0 / 2.0 / 2.2, attack 20 / 15 / 5 ms, release 180 / 200 / 120 ms; the same gains applied to the stems. `limit::limit(x, ceiling)`: 5 ms look-ahead, 80 ms release.
- `loud::to_target(x, stems, lufs: -14.0, peak_db: -1.0)` (F1): measure, gain, limit, measure again, one more gain if off by more than 0.2.
- **C6:** `Polish::hand_bass` is set when the bass-role fighter's saved rumble ≥ 0.5; `LayeredBed::play_bar` skips layers of kind `Bass` between FIGHT! and K.O. when it is set.

- [ ] Unit tests per module: `lows::treat` on a 55 Hz tone cuts phone loss to ≤ 9 dB; `highs::treat` on a 3 kHz tone lowers it by ≥ 3 dB and leaves a 300 Hz tone within 0.5 dB; `room::ir` has rumble share < 0.10 and nothing before 17 ms; `limit` never exceeds the ceiling and is the identity on a quiet signal; `to_target` lands within ±0.5 LUFS of -14 with true peak ≤ -1 dB; a missing fighter in the table trims 0 dB.

### Task 7: Export and checks

**Files:** `bf-export/src/{job.rs, fix.rs, checks/band.rs}`, `bf-app/src/engine/takes.rs`, `bf-cli/src/cmd/check.rs`, `docs/studio/HANDOFF.md`.

- [ ] `BandPlan` gains `composed: bool`; under Composed the band check measures with `heard::over_band` (target 4–6 dB) and `fix_sound` re-limits with `polish::limit` then runs `loud::to_target` last.
- [ ] `report()`: for a Composed render every failing check becomes a note in the report and the render continues. Correct the HANDOFF.md line that says a failing check stops the render.
- [ ] Test `bf-export` unit: a Composed plan with a deliberately failing check returns `Ok` with the note; a Classic plan still fails as today.

### Task 8: Melody style in the app

**Files:** `bf-app/src/{model/draft.rs, engine/convert.rs, state/fight.rs, state/events.rs, screens/music.rs, seam/fake.rs, fixtures/music.rs}`.

- [ ] Draft field `melody_style: Option<Style>` (`None` = Auto). On convert: explicit choice, else `composer::pick_style(theme, music_seed)`; the resolved word is written to `ff.music.melody_style`. Loading a file without the field shows Signature.
- [ ] Music panel: a "Melody style" menu under the arrangement control, shown only for Composed: Auto plus the theme's styles from `family::styles`.
- [ ] App screen test for the menu (existing screen-test pattern), run on the PC.

### Task 9: Measure, listen, ship

- [ ] `ballfight levels --write` in the cloud; commit the table.
- [ ] Render the ten review fights (`sound-review/tools/round2/f/`) on the build branch and on this branch; measure every Done-when line of the spec with the review scripts; write the numbers to `docs/studio/sound-upgrade-measurements.md`.
- [ ] Listening page (Today next to New, level-matched, per fight, plus one clip per melody style and the bari sax before/after), in the style of the review page.
- [ ] Draft PR into `claude/project-thread-h102zt`; last commit carries the `PC-Tests:` lines for the tests named above.
