# BALLFIGHT Studio, milestones 0 and 1: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundations of BALLFIGHT Studio and take one match-up, FLUTE vs TROMBONE, from a fight file to a
finished, checked 61 s mp4 from the command line, with the Rust picture and sound proven against the Python kit.

**Architecture:** A Cargo workspace in `studio/` with one crate per job (`bf-core`, `bf-sim`, `bf-story`,
`bf-picture`, `bf-sound`, `bf-content`, `bf-export`, `bf-cli`). The fight engine writes a **recording**; picture,
sound and story read only the recording and the fight file. A Python export tool (`tools/studio_export/`) reads the
existing kit and writes a committed reference set that every parity test compares against.

**Tech Stack:** Rust (toolchain pinned in `studio/rust-toolchain.toml`), `rayon`, `serde` + `toml` + `serde_json`
(with `float_roundtrip`), `zip`, `png`, `hound`, `sha2`, `clap`, `libm` (pinned exactly, no arch intrinsics). ffmpeg
with libx264 and AAC from `PATH`. Python 3.13 with numpy 2.5.3, scipy 1.18.1, Pillow 12.3.0 for the export tool only.

**Spec:** `docs/studio/2026-11-10-ballfight-studio-design.md`. Read it first; this plan argues from it. Section
numbers below (§) refer to the spec.

**Scope:** Spec milestones 0 and 1 only. The desktop app (`bf-app`), the other 28 fighters, the other 25 themes, the
new music system, ffmpeg bundling and the Update button are later milestones and get their own plans, written by the
build thread when each starts (§14). By the user's instruction of Nov 10, work continues from milestone to milestone
without stopping for approval; a milestone's plan is written, committed and then executed.

## Global Constraints

Every task's requirements include this section.

- **Nothing in `projects/`, `reelkit/` or `series/` is edited.** The export tool imports the kit read-only (runtime
  patches allowed, as `pp_bridge` already does). `python tools/ballfight_replay_hash.py` output is unchanged.
- **GUITAR and SAX never appear** in any roster, registry, test list or file the Studio writes.
- **Video:** 61.0 s, 1,830 frames, 30 fps, internal 360x640, 3x nearest-neighbour upscale to 1080x1920, h264 + aac.
  Soundtrack: 48,000 Hz stereo, exactly 2,928,640 samples (2,860 AAC frames, 61.013 s).
- **Safe zone:** internal x 20 to 340, y 96 to 500, minus x >= 300 below y 290.
- **Loudness:** opening within 1.5 dB of the ending, every 2 s window within 3 dB of it (one deliberate dip allowed),
  spread <= 5 dB.
- **Determinism (§6):** `f64` and the Python operation order; no fused multiply-add (`mul_add` is forbidden); every
  transcendental (sin, cos, tan, atan2, exp, ln, log10, pow, hypot, sinh/tanh) goes through `bf_core::dmath`; `sqrt`
  may use `f64::sqrt`. `x ** 2` in Python becomes `x * x`. This applies to **every** engine crate (`bf-core`,
  `bf-sim`, `bf-story`, `bf-picture`, `bf-sound`, `bf-content`, the checks in `bf-export`), because frames and sound
  must also be bit-identical on any PC (§3). A source-scan test enforces it (Task 1).
- **No result may depend on hash-map iteration order or on thread count.** Use `Vec`/`BTreeMap` where order matters.
  Parallel work (frames, seeds) must produce the same bytes with 1 worker and with 16.
- **Python number semantics are part of the port:** `round()` is round-half-even and `round(x, n)` is correctly
  rounded; `np.round(x, n)` is multiply, round-half-even, divide; `int()` truncates; `//` floors; `%` takes the sign
  of the divisor; numpy casts truncate toward zero and `uint8` casts wrap unless the code clips first. Use
  `bf_core::py` (Task 2) and keep each numpy intermediate in the dtype numpy would give it (`float32` stays `f32`).
- **Seeds are Studio-owned.** Search tries seeds upward from 1. Balance uses seeds from `1 << 40` upward. A Python
  seed is not expected to give the same fight (§6).
- **Parity bars.** Fight: positions, HP and events agree to 1e-6 over the prefix window. Frames: identical RGB;
  exceptions listed in `studio/reference/frames/exceptions.toml` with a reason and at most 2 levels on at most 0.5 %
  of pixels. Stems: sample differences below -120 dBFS; documented exceptions to -80 dBFS.
- **The vectors are the arbiter.** Where this plan describes a numpy, scipy or Pillow algorithm, port it from that
  library's source at the pinned version; if this plan and the exported vectors disagree, the vectors win.
- **Reference files** live under `studio/reference/`, every one listed in `studio/reference/manifest.json` with the
  repo commit, Python and library versions and a SHA-256. Keep the set under 150 MB (§12).
- **Commits** are small and per task step 5. No model names in commits, code or docs.

## Review Focus

Failure modes the spec implies but states no test for, most likely first. Each has a test in the task named.

1. **A fight with no K.O. before 58 s** (winner and K.O. time are absent). Story marks it "not a fit" with a reason;
   `ballfight render` refuses with "seed N has no K.O."; nothing panics. Tasks 15, 30.
2. **A bad fight file:** unknown fighter, GUITAR or SAX, the same fighter twice, a `format` newer than the app, a
   chord the theme cannot voice, an unknown theme, a chord loop that is not 8 bars. Each gives a one-line error
   naming the field. Tasks 7, 27.
3. **Encode trouble:** ffmpeg missing, ffmpeg exiting non-zero mid-encode, the output folder not writable, the
   target folder already existing. The job fails with the cause, leaves no partial folder, and a second render of the
   same fight lands in `<a>_vs_<b>_seed<N>`. Task 29.
4. **A recording that is truncated, from a newer format, or whose arrays disagree in length.** Reading returns an
   error naming the file and the problem. Task 6.
5. **A tempo whose bar count the classic chords do not fit** (any tempo other than the episode's). The grid always
   gives a whole number of bars and the chord list is built for that bar count, never asserted. Tasks 5, 30.

## Order of work

Milestone 0 is Tasks 1 to 10 and must finish first: every later test reads its reference files. After that there are
four lanes that only meet at Task 30:

- **Fight:** 11, 12, 13, 14, then 15, 16.
- **Picture:** 17, 18, 19, 20, 21, 22, then 23 and 24 (independent of each other).
- **Sound:** 25, 26, 27.
- **Export:** 28 (needs 22 and 27 for its live inputs, but its parity tests run on Python files), 29.
- **Together:** 30, 31.

## File map

```
studio/
  Cargo.toml  Cargo.lock  rust-toolchain.toml  README.md
  crates/bf-core/src/     lib.rs dmath.rs py.rs rng.rs ziggurat.rs pyhash.rs grid.rs harmony.rs event.rs npy.rs
                          recording.rs fightfile.rs show.rs
  crates/bf-sim/src/      lib.rs rules.rs roster.rs geom.rs ball.rs fight.rs schedule.rs record.rs
                          kinds/{mod,striker,blade}.rs  abilities/{mod,haste,dance,wave,shockwave,slam}.rs
  crates/bf-story/src/    lib.rs story.rs score.rs search.rs balance.rs filters.rs stats.rs
  crates/bf-picture/src/  lib.rs img.rs np.rs pil/{mod,polygon,line,ellipse,rect,resize}.rs config.rs font.rs
                          camera.rs geometry.rs raster.rs lighting.rs shadows.rs lens.rs post.rs hull.rs
                          fx/{mod,flares,bursts,shake,shimmer,fringe,particles,marks,moments}.rs
                          ui/{mod,captions,hud,label}.rs  sprite.rs  scene.rs  title/{mod,layout,panels,words,fx}.rs
                          ring/{mod,state,arena,canvas,ball,ropes,shards,post_fx,frame}.rs  look.rs
  crates/bf-sound/src/    lib.rs dsp/{mod,iir,butter,resample,fft,conv,npstat}.rs synth.rs mix.rs wav.rs music.rs
                          voice.rs traits.rs score.rs master.rs
  crates/bf-content/      data/roster.json  data/models/{FLUTE,TROMBONE}.json
                          src/{lib,registry,pair_rules}.rs
                          src/fighters/{mod.rs, flute/{mod,look,sound,voice}.rs, trombone/{mod,look,sound,voice}.rs}
                          src/themes/{mod.rs, big_top/{mod,classic}.rs}
  crates/bf-export/src/   lib.rs checks/{mod,levels,band,loudness,flash,loopcheck,safezone,focus}.rs
                          encode.rs probe.rs output.rs job.rs
  crates/bf-cli/src/      main.rs cmd/{sim,story,search,balance,frame,sound,check,render,hash,bench}.rs
  fights/flute_vs_trombone.toml        the delivered episode as a fight file
  replay/hashes.txt                    self-replay hashes of Studio fights
  reference/                           the committed reference set (Tasks 3 to 10)
tools/studio_export/                   __main__.py common.py manifest.py requirements.txt and one module per export
.github/workflows/studio.yml
docs/studio/m1-measurements.md         written in Task 31
```

Python sources each crate ports, for orientation: `bf-sim` from `projects/ballfight/fight.py` and `fighters.py`;
`bf-story` from `kit/story.py`, `kit/search.py`, `kit/tuning.py`; `bf-picture` from `reelkit/{config,camera,raster,
lighting,shadows,lens,post,fx,font,ui,geometry}.py`, `projects/ballfight/{scene,title}.py`, `kit/picture.py`,
`projects/which_instrument_wins/sprites.py`, `series/instrument-history/piano_lowpoly.py` (`render_geo`, `tri`,
`outline` only); `bf-sound` from `reelkit/audio.py`, `kit/music.py`; `bf-content` from `kit/fighters/{base,flute,
trombone}.py`, `kit/moods/big_top.py`, `series/instrument-history/{flute60,trombone60}_lowpoly.py` (`flute_tone`,
`brass2`, `synth` only); `bf-export` from `kit/checks.py`, `kit/encode.py`, `projects/maze_race/mux.py`,
`reelkit/{levels,flashcheck,loopcheck,safezone}.py`. `kit/` means `projects/ballfight/kit/`.

---

# Milestone 0: Foundations

### Task 1: Workspace, CI and the maths lint

**Files:**
- Create: `studio/Cargo.toml`, `studio/rust-toolchain.toml`, `studio/README.md`, one `Cargo.toml` + `src/lib.rs`
  (or `main.rs`) per crate in the file map, `studio/crates/bf-core/tests/no_std_maths.rs`,
  `.github/workflows/studio.yml`
- Modify: `.gitignore` (add `studio/target/`, and `!studio/reference/**` after the `*.npz` and `*.wav` rules so
  reference files are tracked)

**Interfaces:**
- Produces: the workspace; crate dependency edges exactly as the table in §5 (`bf-picture` and `bf-sound` depend
  only on `bf-core`); the binary is named `ballfight`.

- [ ] **Step 1: Write the failing test** `no_std_maths.rs`: walk `studio/crates/{bf-core,bf-sim,bf-story,bf-picture,
  bf-sound,bf-content,bf-export}/src/**/*.rs`, skip `bf-core/src/dmath.rs`, strip comments and string literals, and
  fail listing `file:line` for any of: `.sin(` `.cos(` `.tan(` `.asin(` `.acos(` `.atan(` `.atan2(` `.sinh(` `.cosh(`
  `.tanh(` `.exp(` `.exp2(` `.exp_m1(` `.ln(` `.ln_1p(` `.log(` `.log2(` `.log10(` `.powf(` `.powi(` `.hypot(`
  `.cbrt(` `.mul_add(` `.sin_cos(`. Include a self-check: the scanner flags a sample string containing `x.sin()`.
- [ ] **Step 2:** `cd studio && cargo test -p bf-core --test no_std_maths`. Expected: FAIL (crates do not exist).
- [ ] **Step 3: Create the workspace.** `rust-toolchain.toml` pins the stable channel installed now (`rustc -V`),
  with `components = ["clippy", "rustfmt"]`. Edition 2024, `resolver = "3"`. Workspace profile: `[profile.release]
  opt-level = 3, lto = "thin", codegen-units = 16, debug = 1`; `[profile.dev] opt-level = 1`; `[profile.test]
  opt-level = 2` (the sim and picture tests are too slow unoptimised). No `-C target-cpu`, no fast-math. Commit
  `Cargo.lock`.
- [ ] **Step 4: CI.** `studio.yml` runs on pushes and PRs touching `studio/**` or `tools/studio_export/**`: Ubuntu
  job = `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace
  --release`. (A small Windows job is added in Task 14.)
- [ ] **Step 5:** `cargo test --workspace` passes; commit `studio: workspace, CI, std-maths scan`.

### Task 2: Deterministic maths and Python number semantics

**Files:**
- Create: `studio/crates/bf-core/src/dmath.rs`, `py.rs`, `studio/crates/bf-core/tests/dmath_bits.rs`,
  `studio/reference/dmath/bits.json`, `tools/studio_export/py_vectors.py`, `studio/reference/py/vectors.json`

**Interfaces:**
- Produces:
  ```rust
  // bf_core::dmath: all pure Rust, same bits on every platform
  pub const PI: f64; pub const TAU: f64;
  pub fn sin(x: f64) -> f64;  pub fn cos(x: f64) -> f64;  pub fn tan(x: f64) -> f64;
  pub fn asin(x: f64) -> f64; pub fn acos(x: f64) -> f64; pub fn atan(x: f64) -> f64;
  pub fn atan2(y: f64, x: f64) -> f64;  pub fn hypot(x: f64, y: f64) -> f64;
  pub fn exp(x: f64) -> f64;  pub fn ln(x: f64) -> f64;  pub fn log10(x: f64) -> f64;  pub fn log2(x: f64) -> f64;
  pub fn pow(x: f64, y: f64) -> f64;  pub fn tanh(x: f64) -> f64;  pub fn sinh(x: f64) -> f64;
  pub fn sinf(x: f32) -> f32; pub fn cosf(x: f32) -> f32; pub fn expf(x: f32) -> f32; pub fn powf(x: f32, y: f32) -> f32;
  // bf_core::py
  pub fn round_half_even(x: f64) -> f64;          // Python round(x) and np.rint
  pub fn round_nd(x: f64, n: i32) -> f64;         // Python round(x, n): correctly rounded decimal
  pub fn np_round_nd(x: f64, n: i32) -> f64;      // np.round(x, n): rint(x * 10^n) / 10^n
  pub fn fmod_py(a: f64, b: f64) -> f64;          // Python a % b (sign of b)
  pub fn floor_div(a: f64, b: f64) -> f64;        // Python a // b
  pub fn imod_py(a: i64, b: i64) -> i64;
  pub fn wrap_pi(a: f64) -> f64;                  // (a + pi) % (2 pi) - pi, as fight.py _wrap
  ```
- [ ] **Step 1: Export the Python vectors.** `py_vectors.py` (run on its own here with `python
  tools/studio_export/py_vectors.py`; Task 3 puts it under the tool's entry point and manifest) writes 2,000 seeded
  inputs and outputs for each of
  `round(x)`, `round(x, 1)`, `round(x, 2)`, `round(x, 3)`, `round(x, 5)`, `np.round(x, 2)`, `a % b`, `a // b`
  (floats of both signs, including exact ties such as 0.5, 2.5, 1.000005) as hex floats.
- [ ] **Step 2: Write the failing tests.** `py::tests::matches_python_vectors` (bit-equal on every row) and
  `dmath_bits.rs::bits_are_stable`: for each `dmath` function, 4,096 fixed inputs generated by a SplitMix64 written
  in the test (edge values 0, -0, tiny, huge, multiples of pi/2 included); compare `to_bits()` with
  `reference/dmath/bits.json`. With `BF_BLESS=1` the test writes the file instead.
- [ ] **Step 3: Implement** `dmath` as thin wrappers over the `libm` crate, pinned with `=x.y.z` and with its
  arch-intrinsics feature off (check the crate's current feature names in its docs; the requirement is "no CPU
  intrinsics, software floats only"). `round_nd` formats with `{:.n$}` and parses back; if the Python vectors show a
  tie that differs, implement the decimal rounding by hand until they match.
- [ ] **Step 4:** Bless `bits.json` once on Linux, commit it, then run without `BF_BLESS`: PASS. (Windows checks the
  same file in Task 14.)
- [ ] **Step 5:** Commit `bf-core: deterministic maths and Python number semantics`.

### Task 3: Export tool skeleton, manifest, random and hash vectors

**Files:**
- Create: `tools/studio_export/{__init__.py,__main__.py,common.py,manifest.py,requirements.txt,README.md,
  rng_vectors.py,pyhash_vectors.py}`, `studio/reference/manifest.json`, `studio/reference/rng/vectors.json`,
  `studio/reference/pyhash/vectors.json`

**Interfaces:**
- Produces: `PYTHONHASHSEED=0 python -m tools.studio_export <name>... | all` run from the repo root. `common.py`
  provides `episode(folder)` (loads a kit episode the way `kit/cli.py` does), `write(path, ...)` helpers and
  `hexf(x)` (float to hex string). `manifest.add(path)` records `{path, sha256, bytes, tool, commit, python, numpy,
  scipy, pillow}`. The entry point refuses to run unless `PYTHONHASHSEED == "0"` and the three library versions equal
  `requirements.txt` (numpy 2.5.3, scipy 1.18.1, pillow 12.3.0).

- [ ] **Step 1:** Write `rng_vectors.py`. For each seed in `[0, 1, 2, 392, 2**40 + 7, [392, 44, 3588], [1, 2, 3]]`
  (lists are numpy list seeds, as the engine's mid-fight `default_rng([seed, 44, round(t * 1000)])`), from a fresh
  `np.random.default_rng(seed)` record, as hex floats or ints: 64 x `random()`; 64 x `uniform(-1, 1)`; 64 x
  `uniform(5, 9)`; 256 x `normal(0, 1)`; 64 x `normal(2, 0.5)`; `normal(0, 1, 1000)` (array call); `uniform(0, 1,
  1000)`; 64 x `integers(7)`; 64 x `integers(1000)`; 64 x `integers(2**40)`; `permutation(n)` for n in 2, 5, 16, 100;
  64 x `choice([-1, 1])`; and one **interleaved** stream (uniform, normal, random, integers(5), permutation(4),
  repeated 32 times) to prove the methods share one stream correctly; and one **resume** case: after 1,000
  `normal(0, 1)` draws from seed 77, the `bit_generator.state` (`state`, `inc`) and the next 64 normals.
- [ ] **Step 2:** Write `pyhash_vectors.py`: `hash(s)` for 500 strings: every ability name and fighter name in
  `fighters.py`, caption lines such as `'*K.O.*'`, `'WINS!'`, `'COMMENT THE'`, `'*NEXT MATCH-UP*'`, the empty string,
  and seeded random ASCII strings of length 1 to 40.
- [ ] **Step 3:** Run `PYTHONHASHSEED=0 python -m tools.studio_export rng pyhash py`. Expected: three files written
  and listed in `manifest.json`.
- [ ] **Step 4:** Run `python tools/ballfight_replay_hash.py | head -5` before and after; the lines are identical.
- [ ] **Step 5:** Commit `tools/studio_export: skeleton, manifest, rng and hash vectors`.

### Task 4: numpy `default_rng` port

**Files:**
- Create: `studio/crates/bf-core/src/rng.rs`, `ziggurat.rs`, `studio/crates/bf-core/tests/rng_vectors.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Generator { /* PCG64 state */ }
  impl Generator {
      pub fn new(seed: u64) -> Self;                 // np.random.default_rng(seed)
      pub fn from_seq(seed: &[u64]) -> Self;         // np.random.default_rng([a, b, c])
      pub fn from_state(state: u128, inc: u128) -> Self;   // resume from bit_generator.state (no buffered 32-bit half)
      pub fn random(&mut self) -> f64;
      pub fn uniform(&mut self, lo: f64, hi: f64) -> f64;
      pub fn standard_normal(&mut self) -> f64;
      pub fn normal(&mut self, loc: f64, scale: f64) -> f64;
      pub fn integers(&mut self, high: u64) -> u64;  // rng.integers(high), low = 0
      pub fn permutation(&mut self, n: usize) -> Vec<usize>;
      pub fn choice_index(&mut self, n: usize) -> usize;   // rng.choice(seq): index into seq
      pub fn fill_normal(&mut self, loc: f64, scale: f64, out: &mut [f64]);
      pub fn fill_uniform(&mut self, lo: f64, hi: f64, out: &mut [f64]);
  }
  ```
- [ ] **Step 1: Write the failing test** `rng_vectors.rs::matches_numpy`: for every seed and every stream in
  `reference/rng/vectors.json`, the Rust values are bit-equal. A second test `clone_is_independent`.
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3: Implement** from numpy's source at 2.5.3: `SeedSequence` (entropy to pool, `generate_state` to four
  `u64`), PCG64 (128-bit LCG, XSL-RR output), `next_double` = `(u64 >> 11) * 2^-53`, the ziggurat normal with the
  tables from `numpy/random/src/distributions/ziggurat_constants.h` copied into `ziggurat.rs` with numpy's BSD
  licence notice, bounded integers as `Generator.integers` does without masking (small ranges draw 32-bit values
  and numpy keeps the unused half of each 64-bit output for the next 32-bit draw: model that buffer), and
  `permutation`/`shuffle`/`choice` as `Generator` implements them. The tail of the ziggurat calls `ln` and `exp`
  through `dmath`.
- [ ] **Step 4:** Run the tests: PASS on every stream, including the interleaved one.
- [ ] **Step 5:** Commit `bf-core: numpy default_rng port`.

### Task 5: Python string hash, the bar grid and harmony

**Files:**
- Create: `studio/crates/bf-core/src/{pyhash.rs,grid.rs,harmony.rs}` with unit tests in each

**Interfaces:**
- Produces:
  ```rust
  pub fn str_hash(s: &str) -> i64;   // CPython 3.13 hash(str) under PYTHONHASHSEED=0 (SipHash-1-3, zero key)
  pub fn loop_bpm(bpm: f64, dur: f64, beats_per_bar: u32) -> f64;
  pub struct Grid { pub fps: u32, pub dur: f64, pub nf: usize, pub bpm_base: f64, pub bpm: f64, pub beat: f64,
      pub bar: f64, pub n_bars: usize, pub s16: f64, pub fight_bar: usize, pub t_fight: f64,
      pub count: [(f64, &'static str); 4], pub t_doors: (f64, f64), pub t_close: f64,
      pub t_reassemble: f64, pub t_glide: f64, pub t_rev: f64 }
  impl Grid { pub fn new(bpm_base: f64) -> Grid;      // fps 30, dur 61.0, fight_bar 2, as kit/episode.py build_timeline
      pub fn bar_t(&self, b: f64) -> f64; pub fn next_bar(&self, t: f64) -> f64;
      pub fn beat_env(&self, t: f64, decay: f64) -> f64; pub fn beat_pulse(&self, t: f64, decay: f64, pre: f64) -> f64; }
  // harmony.rs: pitch_class, parse_chord, SCALES, QUALITY, and a Harmony { key_pc, key_root, scale, chords, loop_,
  // loop_start } with chord_at, chord_tones, scale_notes, root_above, loop_index: ports of kit/episode.py _helpers
  pub fn chords_for(intro: &[String], loop_: &[String], n_bars: usize) -> Vec<String>;  // intro, then the loop repeated
  ```
- [ ] **Step 1: Write the failing tests** with these values:
  ```rust
  assert_eq!(loop_bpm(96.0, 61.0, 4), 24.0 * 4.0 * 60.0 / 61.0);     // 94.43, 24 bars
  assert_eq!(loop_bpm(120.0, 61.0, 4), 30.0 * 4.0 * 60.0 / 61.0);    // 118.03, 30 bars
  let g = Grid::new(132.0);
  assert_eq!(g.n_bars, 34); assert_eq!(g.nf, 1830);
  assert!((g.bpm - 133.770_491_803_278_7).abs() < 1e-12);
  assert!((g.t_fight - 2.0 * g.bar).abs() == 0.0 && (g.t_fight - 3.588_235_294_117_647).abs() < 1e-12);
  assert_eq!(g.t_reassemble, 32.0 * g.bar); assert_eq!(g.t_glide, 33.0 * g.bar);
  assert_eq!(chords_for(&v(["Em","B7"]), &v(["Em","Am","B7","Em","C","Am","B7","B7"]), 34).len(), 34);
  assert_eq!(chords_for(&v(["Em","B7"]), &loop8, 30).len(), 30);     // Review Focus 5: any bar count
  ```
  plus `str_hash` equal to every row of `reference/pyhash/vectors.json`, `pitch_class("E") == 4`, and
  `parse_chord("B7")` giving root 11 and intervals `(0, 4, 7, 10)`.
- [ ] **Step 2:** Run: FAIL. **Step 3:** Implement. `loop_bpm` uses `py::round_half_even`. For a tempo sweep test,
  every integer tempo from 60 to 200 gives `n_bars >= 1` and `(n_bars as f64 * g.bar - 61.0).abs() < 1e-9`.
- [ ] **Step 4:** Run: PASS. **Step 5:** Commit `bf-core: string hash, bar grid, harmony`.

### Task 6: Events and the recording container

**Files:**
- Create: `studio/crates/bf-core/src/{event.rs,npy.rs,recording.rs}`, `studio/crates/bf-core/tests/recording.rs`,
  `tools/studio_export/recording.py`, `tools/studio_export/fights.py`, `studio/reference/fights/flute_vs_trombone/
  {ep392,seed1,seed2,seed3,seed500}.bfrec`, `studio/docs/recording-format.md`

**Interfaces:**
- Produces:
  ```rust
  pub enum Value { Null, Bool(bool), Int(i64), Float(f64), Str(String), List(Vec<Value>) }
  pub struct Event { pub t: f64, pub kind: String, pub fields: Vec<Value> }   // Python e[0], e[1], e[2..]
  impl Event { pub fn at(&self, i: usize) -> &Value;   // Python index: at(2) is e[2]
      pub fn f(&self, i: usize) -> f64; pub fn i(&self, i: usize) -> i64; pub fn s(&self, i: usize) -> &str;
      pub fn who(&self) -> usize; }                    // e[2]
  pub enum Array { F64 { shape: Vec<usize>, data: Vec<f64> }, I64 { shape: Vec<usize>, data: Vec<i64> },
                   Nested(Value) }                     // Nested = ragged per-frame lists
  pub enum Source { Studio { version: String }, Python { commit: String } }
  pub struct Recording { pub format: u32, pub source: Source, pub names: Vec<String>, pub seed: u64,
      pub winner: Option<usize>, pub t_ko: Option<f64>, pub f0: usize, pub fps: u32, pub corner: Vec<[f64; 2]>,
      pub rules: BTreeMap<String, Value>, pub events: Vec<Event>, pub arrays: BTreeMap<String, Array> }
  impl Recording { pub fn read(path: &Path) -> Result<Recording, RecError>; pub fn write(&self, path: &Path) -> Result<(), RecError>;
      pub fn frames(&self) -> usize;                   // len of "HP"
      pub fn hash(&self) -> String; }                  // sha256 over a canonical byte form; first 16 hex digits
  pub const RECORDING_FORMAT: u32 = 1;
  ```
- **The container** (`.bfrec`): a zip archive. `meta.json` holds every scalar field and `events` as a JSON list of
  lists (floats written with shortest round-trip repr, so `serde_json` with `float_roundtrip` reads the same bits).
  Each rectangular numeric per-frame array is `<KEY>.npy` (little-endian `<f8` or `<i8`, C order, version 1.0
  header). Each ragged key (`WAVES`, `SLAMS`, `COB` and any other the kit could not turn into an array) is
  `<KEY>.json`. Python writes it with `zipfile` + `numpy.lib.format`; the exporter converts numpy scalars and tuples
  to plain JSON and asserts every float is finite. `recording-format.md` documents this and the meaning of each key
  used by FLUTE and TROMBONE.
- [ ] **Step 1:** Write `recording.py` (`write_bfrec(run, rules, path)`) and `fights.py`, which loads
  `projects/ballfight/examples/flute_vs_trombone` and writes `story.run_seed(s)` for s in 392, 1, 2, 3, 500. Run
  `python -m tools.studio_export fights`. The five files total a few MB.
- [ ] **Step 2: Write the failing tests:** `reads_python_recording` (ep392: `names == ["FLUTE", "TROMBONE"]`,
  `seed == 392`, `winner.is_some()`, `49.0 <= t_ko <= 53.0`, `fps == 30`, `f0 == 108` which is
  `round(3.588235 * 30)`, the `P` array has shape `[frames, 2, 2]`, events are sorted by time);
  `round_trip_is_bit_exact` (read, write to a temp file, read, `==` and same `hash()`); and Review Focus 4:
  `truncated_file_is_an_error`, `newer_format_is_an_error` (message contains "format 2" and "this app reads 1"),
  `mismatched_array_lengths_is_an_error`. None may panic.
- [ ] **Step 3:** Run: FAIL. **Step 4:** Implement `npy.rs` (read and write of the two dtypes only) and
  `recording.rs`. Run: PASS.
- [ ] **Step 5:** Commit `bf-core: events and the recording container; Python recordings exported`.

### Task 7: Roster export, the fight file and the content registry

**Files:**
- Create: `tools/studio_export/roster.py`, `studio/crates/bf-content/data/roster.json`,
  `studio/crates/bf-core/src/fightfile.rs`, `studio/crates/bf-sim/src/roster.rs`,
  `studio/crates/bf-content/src/{lib.rs,registry.rs,pair_rules.rs}`, `studio/fights/flute_vs_trombone.toml`

**Interfaces:**
- Produces:
  ```rust
  // bf_core::fightfile
  pub const FIGHT_FORMAT: u32 = 1;
  pub struct FightFile { pub format: u32, pub app_version: String, pub fighters: Vec<String>, pub arena: String,
      pub handicap: Vec<f64>, pub dmg_scale: f64, pub pair_rules: BTreeMap<String, Value>, pub tempo_bpm: f64,
      pub seed: Option<u64>, pub music: Music, pub picture: PictureSeeds, pub captions: Captions,
      pub series_tag: Option<String> }
  pub struct Music { pub theme: String, pub arrangement: String /* "classic" in M1 */, pub key: String,
      pub scale: String, pub chords_intro: Vec<String>, pub chord_loop: Vec<String>, pub music_seed: u64,
      pub roles: BTreeMap<String, String>, pub lines: BTreeMap<String, Vec<Option<i32>>>,
      pub solos: BTreeMap<String, Vec<(f64, i32, f64)>>, pub title_hits: BTreeMap<String, Vec<i32>> }
  pub struct PictureSeeds { pub shard_seed: u64 /* default 271 */, pub fx_seed: Option<u64> /* default: sum of the
      two names' char codes % 1000 */ }
  pub struct Captions { pub caption: String, pub instagram: String, pub tiktok: String, pub shorts: Vec<String> }
  impl FightFile { pub fn parse(text: &str) -> Result<FightFile, FightFileError>; pub fn to_toml(&self) -> String;
      pub fn validate(&self, names: &dyn Fn(&str) -> NameStatus) -> Result<(), FightFileError>;
      pub fn grid(&self) -> Grid;  pub fn search_key(&self, feeds_fight: &[bool]) -> String; }
  pub enum NameStatus { Known, Unknown, Retired }
  // bf_sim::roster
  pub struct FighterCfg { pub name: String, pub color: [u8; 3], pub dark: [u8; 3], pub weapon: WeaponCfg,
      pub unlock1: AbilityCfg, pub unlock2: AbilityCfg, pub low: AbilityCfg, pub register: (i32, i32),
      pub idiom: String, pub pp: String, pub raw: serde_json::Value }
  pub enum WeaponCfg { Striker(StrikerCfg), Blade(BladeCfg), Unsupported(String) }
  pub struct AbilityCfg { pub name: String, pub kind: AbilityKind, pub params: BTreeMap<String, f64>, .. }
  // bf_content
  pub fn roster() -> &'static [FighterCfg];           // from data/roster.json via include_str!
  pub const PORTED: [&str; 2] = ["FLUTE", "TROMBONE"];   // grows as fighters are ported
  pub fn available() -> Vec<&'static str>;            // PORTED, in roster order
  pub fn pair_rules(a: &str, b: &str) -> BTreeMap<String, Value>;   // {FLUTE, TROMBONE} -> swat_cobras = true
  pub fn music_feeds_fight(name: &str) -> bool;       // true for a fighter whose notes are its weapon; false for both in M1
  pub const RETIRED: [&str; 2] = ["GUITAR", "SAX"];
  ```
- `roster.py` dumps `fighters.FIGHTERS` for the 20 kit names in §10 to JSON with every number kept exactly, never
  GUITAR or SAX. `StrikerCfg` and `BladeCfg` carry exactly the fields FLUTE and TROMBONE use (the roster values are
  in `fighters.py`; a missing optional field takes the default `fight.py` reads with `.get(name, default)`).
- `flute_vs_trombone.toml` is the example episode config field for field: fighters FLUTE, TROMBONE; handicap
  `[2.0, 0.40]`; `dmg_scale = 1.0`; `pair_rules.swat_cobras = true`; `tempo_bpm = 132`; `seed = 392`; theme
  `big_top`; key E, scale `harmonic minor`; `chords_intro = ["Em", "B7"]`; the 8-bar loop; roles; the FLUTE line
  (64 notes); both solos; both title hits; `shard_seed = 271`, `fx_seed = 41`; `series_tag = "BALLFIGHT #15"`; the
  caption and tags.

- [ ] **Step 1: Write the failing tests.** `parses_the_episode` (values above; `ff.grid().n_bars == 34`);
  `to_toml_round_trips`; `roster_has_20_and_no_retired`; `available_is_flute_and_trombone`; `ported_fighters_have_supported_kinds` (no
  `Unsupported` weapon or ability kind for a name in `PORTED`);
  `pair_rule_is_symmetric` (`pair_rules("TROMBONE", "FLUTE")` also has `swat_cobras`); `search_key_ignores_music`
  (changing theme, key or `music_seed` leaves `search_key(&[false, false])` unchanged; changing handicap, tempo,
  sides or a pair rule changes it; with `feeds_fight = [true, false]` the key changes with the chord loop); and
  Review Focus 2, each asserting the error text: unknown fighter (`fighters[1]: "KAZOO" is not in the roster`),
  retired (`fighters[0]: "SAX" is retired`), duplicate, `format = 2` (`this app reads fight files up to format 1`),
  a 7-chord loop (`music.chord_loop: 8 chords needed, found 7`), an unparseable chord name. (Whether the theme
  exists and can voice each chord is checked in Task 27.)
- [ ] **Step 2:** Run: FAIL. **Step 3:** Implement. **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `fight file, roster export, content registry`.

### Task 8: Fight references: scenarios and statistics

**Files:**
- Create: `tools/studio_export/{scenarios.py,stats.py}`, `studio/reference/fights/flute_vs_trombone/scenarios/
  *.bfrec`, `.../coverage.json`, `.../stats.json`

**Interfaces:**
- Produces: scenario recordings, each a short honest run with rule overrides passed through `story.rules(**over)`:
  `early_levels` (`LV_DMG=(2.0, 5.0)`), `early_low` (`LV_DMG=(2.0, 5.0)`, `LOW_HP=92.0`), `plain` (no overrides,
  seed 7), and `swapped` (TROMBONE left: the exporter builds the config with the fighters and `DMG_SIDE` reversed);
  three seeds each; the overrides are stored in the recording's `rules`. `coverage.json`: for every event kind and
  every `hit`/`ability` sub-kind seen, the scenario and time of its first occurrence. `stats.json`: one row per seed
  for seeds 1 to 4,000 under the episode's rules (`record=False`): `seed, winner, t_ko, lead_changes, winner_hp,
  n_caps, max_gap, lows, fits, score`, plus the pooled figures.

- [ ] **Step 1:** Write both modules; `stats.py` uses a process pool.
- [ ] **Step 2:** Run `python -m tools.studio_export scenarios stats`.
- [ ] **Step 3: Check coverage by assertion in the exporter.** The required list is built from the data: every
  event kind and every `hit` sub-kind that occurs in at least 20 of the 4,000 statistics fights. Each must appear in
  `coverage.json` within 20 s of FIGHT!. It must include at least: `basket`, `strike`, `thrust`, `swat`, `rope`,
  `clash`, hits of sub-kind `fang` and `blade`, and all six abilities (FLUTTER TONGUE, SNAKE DANCE, PICCOLO, SAD
  TROMBONE, POWER SLIDE, PEDAL TONE) with the hits their waves, shockwaves and slams cause. If one is missing, add
  seeds or lower the thresholds until it appears. The exporter prints the table.
- [ ] **Step 4:** `stats.json` pooled figures printed: win share, median K.O., mean lead changes, fit rate.
- [ ] **Step 5:** Commit `reference: scenario fights, coverage table, 4,000-seed statistics`.

### Task 9: Picture references

**Files:**
- Create: `tools/studio_export/{pillow_vectors.py,models.py,sprites.py,frames.py,stages.py}`,
  `studio/reference/pillow/*.npz`, `studio/crates/bf-content/data/models/{FLUTE,TROMBONE}.json`,
  `studio/reference/sprites/{FLUTE,TROMBONE}.npz`, `studio/reference/frames/flute_vs_trombone/{f*.png,frames.json,
  title/*.png}`, `studio/reference/frames/exceptions.toml` (empty list)

**Interfaces:**
- Produces:
  - **Pillow vectors:** for each primitive the renderer calls (`ImageDraw.polygon`, `.line` with widths 1, 2, 3,
    `.ellipse` filled and outlined, `.rectangle`, `Image.resize` with NEAREST, BILINEAR and BICUBIC, on modes `L`,
    `RGB` and `RGBA` as used), 200 seeded cases each on canvases up to 96x96: float and integer vertices, vertices
    outside the canvas, degenerate and self-intersecting polygons, horizontal and vertical edges, sub-pixel slivers.
    Stored as the argument list plus the resulting array.
  - **Models:** `pp_bridge.model_faces(name)` as JSON: per face `P` (k x 3), `n`, `rgb`, `two`, `emit`, `gid`,
    `spec`, with round-trip floats.
  - **Sprites:** `kit.picture.sprite` output (RGBA) for radius 12, 16, 22 and 30, yaw every 6 degrees, roll 0, plus
    every extra roll the two looks request.
  - **Frames:** internal 360x640 RGB PNGs of the episode (seed 392) from `kit.picture.render(f)`. `frames.json` lists
    each frame with its reason. Chosen by the exporter from the recording: frames 0, 13, 27, the last closed-door
    frame, a mid-door frame, the "3", "2", "1" and "FIGHT!" frames; for each fighter the frame 0.2 s and 0.6 s after
    the first use of each of its three abilities, a frame on one of its weapon hits, and a frame where its HP is
    below 25; a parry frame and a cobra-swat frame; the K.O. frame, K.O. + 0.2 s, + 0.6 s, + 1.5 s (WINS caption),
    mid-solo, the COMMENT caption, mid-reassembly, mid-glide, and frames 1,828 and 1,829. At least 40 frames.
  - **Title card vectors:** `title.draw(img, t)` over a fixed seeded-noise 360x640 `uint8` image at 16 times: each
    word's slam (the frame of the slam, +2 and +5 frames), the hold, four frames through the doors opening, and four
    through the doors closing at the end. The card is 94 % opaque over a dimmed arena, so it cannot be tested from
    the full frames alone.
  - **Stage dumps (not committed):** `python -m tools.studio_export stages <frame>...` writes the float buffers after
    each stage of `kit.picture.render` (floor, arena, skirt, canvas, painter pass, over-ring and particles, lens,
    glows and beam, bloom and grade, post fx, HUD and captions, title card) to `studio/reference/.cache/stages/`
    (git-ignored), by wrapping the functions `render` calls. Used to find where a frame first differs.

- [ ] **Step 1:** Write the five modules. **Step 2:** Run `python -m tools.studio_export pillow models sprites
  frames`.
- [ ] **Step 3:** Open six of the PNGs and confirm they show the episode (title card, fight, K.O., solo).
- [ ] **Step 4:** Confirm `du -sh studio/reference` is under 60 MB so far.
- [ ] **Step 5:** Commit `reference: Pillow vectors, models, sprites, golden frames`.

### Task 10: Sound references and check references

**Files:**
- Create: `tools/studio_export/{dsp_vectors.py,voices.py,stems.py,checks.py}`, `studio/reference/dsp/*.npz`,
  `studio/reference/sound/voices/*.npz`, `studio/reference/sound/flute_vs_trombone/{stems.npz,rms.json,audio.wav}`,
  `studio/reference/checks/flute_vs_trombone.json`

**Interfaces:**
- Produces:
  - **DSP vectors:** `butter` SOS coefficients for every `(kind, cutoff, order)` the M1 modules request (collected
    by wrapping `reelkit.audio.sos` and `scipy.signal.butter` while the episode's soundtrack is built); `sosfilt` and
    `lfilter` outputs on a fixed 4,800-sample noise input for 12 of those filters; `resample_poly(x, 160, 147)` on
    three inputs; `fftconvolve` on two input pairs; `np.interp`, `np.percentile(x, 99.97)`, `np.convolve(..., 'same')`
    cases; every `reelkit.audio` generator (`kick`, `snare`, `clap`, `hat`, `crash`, `whoosh`, `thud`, `pluck`, `pad`,
    `bass`, `chime`, `reverb`, `tape_stop`, `saw`, `square`, `tri`, `adsr`, `expenv`) at its default arguments and at
    each argument set the episode uses. Stored as `float64`.
  - **Voices:** `pp_bridge.voice` for FLUTE at MIDI 72, 76, 84, 93 and TROMBONE at 40, 47, 52, 64, at velocities 0.6
    and 0.9 and durations 0.12, 0.25 and 1.0; and each `kit.music` helper (`note`, `gliss`, `bend`, `vibrato`, `wah`,
    `colour`, `clank`, `hiss`, `roar`, `thunder`, `riser`) on those notes. **The flute's breath noise comes from a
    generator shared by every Pixel Pitch voice** (`drums60_lowpoly._rng`, seeded 77 and advanced by every call), so
    each voice vector also stores that generator's `state` and `inc` just before the call.
  - **Noise start and call log:** in a fresh interpreter that builds the episode's soundtrack, the state of
    `drums60_lowpoly._rng` after all imports and before the first note (`pp_noise_start.json`), and the ordered list
    of every first-time `kit.music.note` call during `score()` with its arguments (`note_calls.json`).
  - **Stems:** from `kit.music.score()` and `master()` for the episode: for every bus, `float32` windows of 2.0 s
    starting at 0.0, 3.4, 12.0, 30.0, T_KO - 1.0, T_KO + 3.0 and 59.0 s (`stems.npz`), and the RMS of every 100 ms
    block over the whole length (`rms.json`, in dB); the pre-master buses the same way; and the mastered `audio.wav`
    (16-bit, as the kit writes it).
  - **Checks:** the numbers each kit check prints for the episode: the `reelkit.levels` profile and verdict, `band`
    per fighter and span, `loudness` (intro minus fight, LU), `flash` (worst count, largest area and its time),
    `loop` (each line's measured value), `focus` (worst CoC), `safezone` (pass and the tracked boxes).

- [ ] **Step 1:** Write the four modules. **Step 2:** Run `python -m tools.studio_export dsp voices stems checks`.
- [ ] **Step 3:** Play `audio.wav` through `ffprobe`: 61.013 s, 48,000 Hz, stereo, 2,928,640 samples.
- [ ] **Step 4:** `du -sh studio/reference` under 150 MB; `python -m tools.studio_export verify` re-hashes every
  file in the manifest and reports none missing or changed.
- [ ] **Step 5:** Commit `reference: DSP vectors, voices, stems, check numbers`. Milestone 0 is done: open
  `studio/README.md` and write how to regenerate the reference set and how to run the tests.

---

# Milestone 1: One match-up end to end

### Task 11: Fight engine core

**Files:**
- Create: `studio/crates/bf-sim/src/{rules.rs,geom.rs,ball.rs,fight.rs,schedule.rs,record.rs}`,
  `studio/crates/bf-sim/tests/core_prefix.rs`

**Interfaces:**
- Consumes: `Generator`, `dmath`, `py`, `Recording`, `Event`, `Value`, `FighterCfg`.
- Produces:
  ```rust
  pub struct Rules { pub a: f64 /*3.0*/, pub r: f64 /*0.6*/, pub v: f64 /*2.6*/, pub dt: f64 /*1/120*/, pub fps: u32,
      pub rope_give: f64 /*5.0*/, pub hp: f64 /*100*/, pub low_hp: f64 /*25*/, pub lv_dmg: (f64, f64) /*14, 36*/,
      pub knock_t: f64 /*0.38*/, pub launch_spread: f64 /*38*/, pub key_r: f64 /*0.11*/, pub bpm: f64, pub beat: f64,
      pub t_fight: f64, pub t_max: f64 /*58.0*/, pub dmg_scale: f64, pub dmg_side: Option<[f64; 2]>,
      pub swat_cobras: bool }
  impl Rules { pub fn new(bpm: f64, t_fight: f64) -> Rules;
      pub fn apply(&mut self, overrides: &BTreeMap<String, Value>) -> Result<(), SimError>; }  // pair rules, scenario overrides
  pub struct Setup<'a> { pub fighters: [&'a FighterCfg; 2], pub rules: Rules, pub seed: u64 }
  pub fn run(setup: &Setup, record: bool) -> Recording;   // record = false: events, winner, t_ko only
  pub fn setup_from(ff: &FightFile, roster: &[FighterCfg], seed: u64) -> Result<Setup, SimError>;
  ```
- **Structure.** `Fight` owns the two `Ball`s, the generator, the event list and the per-kind state as
  `Option<...>` fields. `schedule.rs` is the one place that holds the order of a step, transcribed from
  `Fight.run()` (`fight.py` from `def run` to the end of the class): beat-grid triggers, each kind's 16th-note
  counter in the Python order, timers, `move`, each kind's `*_step`, abilities, effects, K.O. test, `snap`. Kinds not
  ported yet are simply absent; the order of those present never changes. Every kind module exposes free functions
  taking `&mut Fight` and is called only from `schedule.rs`.
- This task ports: `Rules`, the helpers above `class Ball` (`_wrap`, `_swept`, `_seg_pt`, `_seg_seg`),
  `Ball.__init__`, `Fight.__init__`, `ev`, `damage`, `knock`, `move` (walls, rope give, ball-to-ball clash), level
  and low-HP bookkeeping, `snap` for the base keys (`P V TH BL FR NV HP LV SH HA SPIN OM K WAVES SLAMS`),
  `lead_changes`, and `run` with no weapon kinds.

- [ ] **Step 1: Write the failing tests.** Unit: `launch_matches_python` (seed 392: after `Fight::new`, both balls'
  `p`, `v` and `th` equal frame `f0` of `ep392.bfrec` to 1e-12); `event_times_are_rounded_to_5_places`.
  Integration `core_prefix.rs::motion_agrees_until_first_hit`: for each of the five reference recordings, run the
  Rust fight (no weapon kinds exist yet at this task; the test stays valid once they do) and compare `P` and `V`
  frame by frame up to the frame before the first `hit`, `block` or `ability` event in the Python recording;
  require agreement to 1e-9 and at least 20 frames compared; `rope` and `clash` events in that span match in order,
  time and fields.
- [ ] **Step 2:** Run: FAIL. **Step 3:** Implement. **Step 4:** Run: PASS.
- [ ] **Step 5:** Commit `bf-sim: engine core`.

### Task 12: Striker weapon (the FLUTE's cobras) and the swat rule

**Files:**
- Create: `studio/crates/bf-sim/src/kinds/{mod.rs,striker.rs}`; modify `schedule.rs`, `record.rs` (the `COB` key)

**Interfaces:**
- Consumes: `Fight`, `StrikerCfg`. Produces: `striker::{setup, beat, fire, step}` and `swat_cobras(fight, k)`,
  ports of `striker_setup`, `striker_beat`, `striker_fire`, `striker_step` and `swat_cobras` in `fight.py`.

- [ ] **Step 1: Write the failing test** `striker_prefix`: for the five reference recordings, `P`, `V`, `HP`, the
  `COB` rows and the events of kinds `basket`, `strike`, `rope`, `clash` and `hit` (sub-kind `fang`) agree with
  Python to 1e-9 from `f0` until the first event the trombone's lance causes (a `hit` of sub-kind `blade`, a `swat`
  or a `block`) or the first `ability`; those need Tasks 13 and 14. Unit tests: a basket drops every `drop_every =
  4` beats and the oldest is taken back beyond `max_baskets = 2` (the `basket` event carries the removed id, else
  -1); a cobra cannot strike before `rise = 1.0` beat after its drop; each landed bite grows the reach by `grow =
  0.07` up to `max_reach = 2.4` and shortens the strike by `speed_grow = 0.003` down to `t_min = 0.07`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-sim: striker kind and swat rule`.

### Task 13: Blade weapon in thrust mode (the TROMBONE's slide)

**Files:**
- Create: `studio/crates/bf-sim/src/kinds/blade.rs`; modify `schedule.rs`

**Interfaces:**
- Produces: `blade::{start_thrust, thrust_step, step}`, ports of `start_thrust`, `thrust_step` and the thrust
  branch of `blade_step`, including `parry`, `rest = 0.30`, the fret growth (`frets0 = 1`, `grow = 0.08`, `max_len =
  2.5`) and `aim_rate = 7.0`.

- [ ] **Step 1: Write the failing test** `prefix_before_abilities`: for the five reference recordings, positions,
  HP, `BL`, `COB` and the full event list agree with Python (1e-6 on floats, exact on kinds and integers) from `f0`
  until the first `ability` event. Unit: an un-hasted thrust takes `t_out + t_hold + t_back = 0.32 s` and hits at
  most once; a thrust that meets a shield emits `block` and pulls straight back; a live lance through a risen cobra
  emits `swat` and that cobra cannot strike for 2 beats.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-sim: blade thrust kind`.

### Task 14: Abilities, full parity, self-replay and the Windows job

**Files:**
- Create: `studio/crates/bf-sim/src/abilities/{mod,haste,dance,wave,shockwave,slam}.rs`,
  `studio/crates/bf-sim/tests/{prefix.rs,coverage.rs,replay.rs}`, `studio/replay/hashes.txt`; modify
  `.github/workflows/studio.yml`

**Interfaces:**
- Produces: ports of `trigger`, `abilities_step`, `effects_step` and `ab_mult` for the kinds FLUTE and TROMBONE use:
  `haste`, `dance`, `wave`, `shockwave` (with its slow), `slam`. `pub const PREFIX_FLOOR_S: f64 = 10.0;`
  `pub fn prefix_window(rust: &Recording, python: &Recording) -> PrefixReport { agree_until_s: f64, first_break:
  Option<String> }`.

- [ ] **Step 1: Write the failing tests.**
  - `prefix.rs::agrees_with_python`: for the five reference fights and every scenario, `agree_until_s -
    t_fight >= PREFIX_FLOOR_S` (or the whole fight if shorter). Prints one line per fight with the window.
  - `coverage.rs::every_kind_fires_inside_the_window`: every entry of `coverage.json` occurs in the Rust run of its
    scenario at the same time (1e-6) with the same fields.
  - `replay.rs::hashes_are_stable`: `run` for FLUTE vs TROMBONE at the episode's rules for seeds 1, 2, 3, 500 and
    `1 << 40`, both side orders, prints `seed winner t_ko n_events hash`; equal to `studio/replay/hashes.txt`
    (written with `BF_BLESS=1`).
  - `replay.rs::record_flag_does_not_change_the_fight`: events, winner and K.O. time are equal with `record` true and
    false.
- [ ] **Step 2:** FAIL. **Step 3:** Implement the five ability modules and wire them into `schedule.rs`.
- [ ] **Step 4:** PASS. If a fight breaks before the floor, the break is a bug until shown otherwise: compare the
  events around the break. The floor may only be lowered with the cause written in `m1-measurements.md`.
- [ ] **Step 5:** Add the Windows CI job: build `bf-cli`, then `cargo test --release -p bf-core -p bf-sim` (this
  runs the `dmath` bits, the random vectors and the replay hashes on Windows). Push and confirm both jobs pass.
  Commit `bf-sim: abilities, parity suites, self-replay; Windows job`.

### Task 15: Story and score

**Files:**
- Create: `studio/crates/bf-core/src/show.rs`, `studio/crates/bf-story/src/{story.rs,score.rs}`,
  `studio/crates/bf-story/tests/story_parity.rs`

**Interfaces:**
- Produces:
  ```rust
  // bf_story
  pub struct BeatSheet { pub t_ko: (f64, f64) /*49, 53*/, pub min_lead_changes: u32 /*2*/, pub max_winner_hp: f64 /*30*/,
      pub cap_gap: f64 /*4.0*/, pub max_gap: f64 /*4.0*/, pub min_caps: usize /*5*/,
      pub cap_targets: BTreeMap<String, f64> }   // unlock1 12 s, unlock2 24 s, low 40 s, by ability name
  pub struct Story { pub seed: u64, pub winner: Option<usize>, pub t_ko: Option<f64>, pub lead_changes: u32,
      pub lc_t: Vec<f64>, pub winner_hp: f64, pub firsts: Vec<(f64, usize, String)>,
      pub caps: Vec<(f64, usize, String)>, pub max_gap: f64, pub lows: Vec<String>,
      pub lead: Vec<(f64, f64)> }                // (time, hp[0] - hp[1]) after each hit: the lead ribbon's data
  pub struct Verdict { pub fits: bool, pub score: f64, pub misses: Vec<String> }   // "no K.O.", "K.O. at 47.2 s", ...
  pub fn story(rec: &Recording, grid: &Grid, sheet: &BeatSheet, low_names: [&str; 2]) -> Story;
  pub fn score(st: &Story, sheet: &BeatSheet) -> Verdict;
  // bf_core::show: what picture and sound both need, derived once
  pub struct Show { pub grid: Grid, pub names: [String; 2], pub win: usize, pub lose: usize, pub t_ko: f64,
      pub t_hs: f64 /* t_ko + 0.35 */, pub t_solo: f64 /* next_bar(t_ko + 1.6) */, pub caps_ability: [Vec<Cap>; 2],
      pub cap_ko: Vec<Cap>, pub cap_wins: Vec<Cap>, pub cap_comment: Vec<Cap>, pub captions: Vec<Cap> }
  pub struct Cap { pub t0: f64, pub t1: f64, pub lines: Vec<String> }
  pub fn captions(events: &[Event], gap: f64) -> Vec<(f64, usize, String)>;   // kit/story.py captions(); bf-story reuses it
  impl Show { pub fn new(rec: &Recording, ff: &FightFile) -> Result<Show, ShowError>; }   // caption gap 4.0 s
  ```
  `score` is the formula in `kit/story.py score()` unchanged, rounded with `py::round_nd(s, 2)`. `Show` holds the
  caption windows built by `kit/picture.py _caps()` (ability name for 2.4 s; `*K.O.*` for 1.25 s; winner name and
  `WINS!` from +1.3 to +3.55 s; `COMMENT THE` / `*NEXT MATCH-UP*` from +3.65 s to the reassembly).

- [ ] **Step 1: Write the failing tests.** `story_parity.rs`: for the five Python recordings, `story` + `score` give
  the same winner, K.O. time, lead changes, `lc_t`, winner HP, captions, `max_gap`, lows, `fits` and `score` as that
  seed's row in `stats.json` (and ep392 scores as its line in the kit's `seeds.txt` format). Review Focus 1:
  `no_ko_is_not_a_fit` (a recording with `winner = None`, `t_ko = None` gives `fits == false`, `misses ==
  ["no K.O."]`, and `Show::new` returns `Err(ShowError::NoKo)`; nothing panics).
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-story: story and score; bf-core: show timings`.

### Task 16: Search, balance, filters and the statistics proof

**Files:**
- Create: `studio/crates/bf-story/src/{search.rs,balance.rs,filters.rs,stats.rs}`,
  `studio/crates/bf-story/tests/{stats_parity.rs,search.rs,balance.rs}`

**Interfaces:**
- Produces:
  ```rust
  pub struct Candidate { pub seed: u64, pub story: Story, pub verdict: Verdict }
  pub struct SearchOpts { pub from: u64 /*1*/, pub stop_fits: usize /*200*/, pub max_seeds: Option<u64>, pub workers: usize }
  pub fn search(setup_for: &(dyn Fn(u64) -> Setup + Sync), sheet: &BeatSheet, opts: &SearchOpts,
      progress: &(dyn Fn(SearchProgress) + Sync), cancel: &AtomicBool) -> Vec<Candidate>;   // fits first, by score desc, then seed asc
  pub struct Filters { pub winner: Option<usize>, pub ko: Option<(f64, f64)>, pub min_lead_changes: Option<u32>,
      pub close_finish: bool /* winner_hp <= 12 */, pub comeback: bool, pub ability: Option<String>, pub show_misfits: bool }
  impl Filters { pub fn keep(&self, c: &Candidate) -> bool; }
  pub struct Stats { pub n: usize, pub decided: usize, pub win_share: f64, pub ko_median: f64, pub ko_p10: f64,
      pub ko_p90: f64, pub no_ko_share: f64, pub lead_changes_mean: f64, pub fit_rate: f64 }
  pub fn stats(setup_for: ..., sheet: &BeatSheet, seeds: Range<u64>, workers: usize) -> Stats;
  pub const BALANCE_SEED0: u64 = 1 << 40;
  pub struct BalanceResult { pub handicap: [f64; 2], pub confirm: Stats, pub in_band: bool, pub steps: Vec<BalanceStep> }
  pub fn auto_balance(setup_for: &(dyn Fn([f64; 2], u64) -> Setup + Sync), sheet: &BeatSheet, start: [f64; 2],
      workers: usize, progress: ..., cancel: &AtomicBool) -> BalanceResult;
  ```
- **Search** hands out seeds in order to the workers, collects results by seed, and stops once `stop_fits` fits are
  found **among a contiguous prefix of seeds**, so the result is the same for any worker count.
- **Auto-balance.** Write the handicap as `(s * sqrt(r), s / sqrt(r))`. Round A: bisect `ln r` over `[ln 1/8, ln 8]`
  for a 50 % win share, 7 steps. Round B: bisect `ln s` over `[ln 1/4, ln 4]` for a median K.O. of 50.5 s (a fight
  with no K.O. counts as 99 s), 7 steps. Do A, B, A, B. Every evaluation uses the same 400 seeds
  `BALANCE_SEED0..BALANCE_SEED0 + 400`. Round the result to 2 decimals. Confirm on 3,000 seeds from
  `BALANCE_SEED0 + 1_000_000`. `in_band` = win share in 40 to 60 % and median K.O. in 49 to 52 s. The comeback filter
  reads `Story.lead`: the winner trailed by 20 HP or more at some time after 15 s, or was first below 25 HP.

- [ ] **Step 1: Write the failing tests.**
  - `stats_parity.rs` (release only, `#[ignore]` in debug): Rust statistics over seeds 1 to 40,000 against Python's
    4,000 in `stats.json`: win share within 3 points, median K.O. within 0.7 s, mean lead changes within 0.25, fit
    rate within 1.5 points. (Seeds are independent samples in the two engines; 40,000 Rust fights keep the sampling
    noise of the comparison under 1 point.)
  - `search.rs::same_shortlist_for_any_worker_count` (1 worker and 8 workers, `stop_fits = 5`: identical seeds and
    order); `search_can_be_cancelled`; each filter on hand-built candidates.
  - `balance.rs::lands_in_band` (start `[1.0, 1.0]`: `in_band`, and the ratio `handicap[0] / handicap[1]` is between
    2.5 and 10, the neighbourhood of the kit's hand-tuned 2.0 / 0.40); `same_result_for_any_worker_count`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement with `rayon`. **Step 4:** PASS. Record fights per second.
- [ ] **Step 5:** Commit `bf-story: search, balance, filters, statistics parity`.

### Task 17: Picture foundations: images, numpy semantics, Pillow primitives, font

**Files:**
- Create: `studio/crates/bf-picture/src/{img.rs,np.rs,config.rs,font.rs}`, `pil/{mod,polygon,line,ellipse,rect,
  resize}.rs`, `studio/crates/bf-picture/tests/pillow_vectors.rs`

**Interfaces:**
- Produces:
  ```rust
  pub const W: usize = 360; pub const H: usize = 640; pub const UP: usize = 3; pub const FPS: u32 = 30;
  pub const SAFE: (i32, i32, i32, i32) = (20, 96, 340, 500); pub const SAFE_BUTTONS: (i32, i32, i32, i32) = (300, 290, 360, 640);
  pub struct Img<T> { pub w: usize, pub h: usize, pub c: usize, pub data: Vec<T> }   // row-major, interleaved
  pub type Rgb8 = Img<u8>; pub type RgbF = Img<f64>; pub type Gray32 = Img<f32>;
  pub fn bayer_full() -> &'static Img<f64>;    // reelkit.config.BAYER_FULL
  pub fn ss(a: f64, b: f64, x: f64) -> f64; pub fn ease(x: f64) -> f64; pub fn safe_x1(y0: i32, y1: i32) -> i32;
  // np.rs: clip, the casts (f64 -> u8 truncating, wrapping as numpy does), floor-dither helpers, roll, nonzero order
  // pil: the same results as Pillow 12.3.0
  pub fn polygon(img: &mut Img<u8>, pts: &[(f64, f64)], fill: &[u8]);
  pub fn line(img: &mut Img<u8>, pts: &[(f64, f64)], fill: &[u8], width: u32);
  pub fn ellipse(img: &mut Img<u8>, bbox: (f64, f64, f64, f64), fill: Option<&[u8]>, outline: Option<&[u8]>, width: u32);
  pub fn rectangle(img: &mut Img<u8>, bbox: (f64, f64, f64, f64), fill: Option<&[u8]>, outline: Option<&[u8]>);
  pub enum Resample { Nearest, Bilinear, Bicubic }
  pub fn resize(src: &Img<u8>, w: usize, h: usize, how: Resample) -> Img<u8>;
  // font.rs: reelkit/font.py
  pub const ADV: i32; pub fn text_mask(s: &str, scale: u32) -> Img<u8>; pub fn dilate(m: &Img<u8>, r: u32) -> Img<u8>;
  pub fn stamp(frame: &mut Rgb8, mask: &Img<u8>, x: i32, y: i32, color: [u8; 3], outline: [u8; 3], shadow: bool, alpha: f64);
  ```
- [ ] **Step 1: Write the failing test** `pillow_vectors.rs`: every case in `reference/pillow/*.npz` is reproduced
  byte for byte, reported per primitive (`polygon: 200/200`).
- [ ] **Step 2:** FAIL. **Step 3:** Implement from Pillow's `Draw.c` and `Resample.c` (the edge-list polygon
  filler with its rounding, the wide-line-as-polygon rule, the ellipse quadrant algorithm, the fixed-point resample
  filters). Font glyphs come from the table in `reelkit/font.py`.
- [ ] **Step 4:** PASS at 100 % for every primitive the renderer uses. A primitive that cannot reach 100 % is
  listed in `m1-measurements.md` with its failing cases before any frame work depends on it.
- [ ] **Step 5:** Commit `bf-picture: images, numpy casts, Pillow-exact primitives, pixel font`.

### Task 18: The renderer kit: camera, raster, lighting, shadows, lens, post

**Files:**
- Create: `studio/crates/bf-picture/src/{camera.rs,geometry.rs,raster.rs,lighting.rs,shadows.rs,lens.rs,post.rs,
  hull.rs}`, `tools/studio_export/kit_vectors.py`, `studio/reference/picture_kit/*.npz`,
  `studio/crates/bf-picture/tests/kit_vectors.rs`

**Interfaces:**
- Produces ports with the Python names and argument orders: `Cam` (`new`, `to_cam`, `proj`, `wp`, `depth`,
  `pixel_rays`, `ground_hit`), `clip_near`, `interp_keys`; `Frame` (`new(bg)`, `poly`, `screen_poly`, `line`, `disc`,
  `glow`, `array`, `depth`, `set_array`), `apply_glows`; `Lights` (`key`, `fill`, `point`), `shade`; `MaskGrid`
  (`raster`, `sample`), `proj_point`; `coc_map`, `dof` (octagonal kernels); `bloom`, `grade`, `vignette`,
  `top_scrim`; the `geometry.rs` builders the ring uses (`face`, `box_`, `prism`, `lathe`, `catenary`, `thin_quad`);
  `convex_hull(pts: &[(f64, f64)]) -> Vec<usize>` returning the vertices in the order scipy's `ConvexHull(...)
  .vertices` gives for 2D input (counter-clockwise).
- [ ] **Step 1:** Write `kit_vectors.py`: for each function above, 5 to 20 seeded calls with small inputs (for
  example `dof` on a 64x96 noise image with a random CoC map; `shade` on 50 faces; `MaskGrid.raster` on 10 polygon
  sets; `bloom`, `grade`, `vignette` on a 64x96 image; `Frame.poly` on 30 random quads with depth) storing inputs and
  outputs. Run it; commit the vectors (a few MB).
- [ ] **Step 2: Write the failing test** `kit_vectors.rs`: outputs equal Python's: `uint8` results exactly; float
  results to 1e-9 relative.
- [ ] **Step 3:** FAIL. **Step 4:** Implement each numpy expression as a loop that keeps numpy's dtype and
  operation order; no SIMD crates. PASS.
- [ ] **Step 5:** Commit `bf-picture: camera, raster, lighting, shadows, lens, post`.

### Task 19: Effects kit and the HUD

**Files:**
- Create: `studio/crates/bf-picture/src/fx/*.rs`, `ui/*.rs`; extend `tools/studio_export/kit_vectors.py` and
  `tests/kit_vectors.rs`

**Interfaces:**
- Produces ports of `reelkit/fx.py` (`env_ad`, `FlashBudget`, `Flares`, `starburst`, `Bursts`, `Shake`, `shimmer`,
  `shimmer_world`, `ellipse_of`, `fringe`, `Particles` with `emit`, `alive`, `draw`, `Marks` with `add`, `apply`,
  `Moments` with `add`, `env`, `apply_dim`, `speed_lines`) and of the parts of `reelkit/ui.py` the fight uses
  (`draw_captions` in `impact` style with `_sand_puff` and `_hash01`, `impact_landings`, `label`, `fight_hud` with
  `_ability_rows`, `panel`). Every draw call records its box in a `DrawLog` (`pub struct DrawLog { pub boxes:
  Vec<Noted> }`, the port of `ui._note`) for the safe-zone check. `_sand_puff`'s key uses `bf_core::pyhash::
  str_hash(line) % 997` with Python's modulo.
- [ ] **Step 1:** Add vectors: each effect class driven by a seeded script of 20 `add`/`emit` calls and drawn at 6
  times on a 360x640 mid-grey image; `fight_hud` in 8 states (full HP, damaged with trail, low HP, flash, K.O., win,
  levels 1 to 3, with and without ability rows); `draw_captions` at 6 times across a caption's life.
- [ ] **Step 2:** Failing test as in Task 18. **Step 3:** FAIL. **Step 4:** Implement; PASS (`uint8` images exact).
- [ ] **Step 5:** Commit `bf-picture: effects kit, captions, HUD`.

### Task 20: Models and the sprite renderer

**Files:**
- Create: `studio/crates/bf-picture/src/sprite.rs`, `studio/crates/bf-picture/tests/sprites.rs`

**Interfaces:**
- Produces: `pub struct Mesh { pub faces: Vec<Face> }` (deserialised from the model JSON); `pub fn render_sprite(
  mesh: &Mesh, radius_px: u32, yaw_deg: f64, roll_deg: f64) -> Img<u8>` (RGBA, side `2 r + 7`), the port of
  `sprites.render_sprite` with `piano_lowpoly.render_geo`, `tri` and `outline` (light direction, the 7-band shading,
  the specular steps and the ink outline as in that file); `pub struct SpriteCache` keyed by `(fighter index,
  radius, yaw step of 6 degrees, roll)` and safe to share across render threads.
- [ ] **Step 1: Write the failing test** `sprites.rs`: every sprite in `reference/sprites/{FLUTE,TROMBONE}.npz` is
  reproduced exactly (all four radii, 60 yaws, each roll).
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-picture: meshes and the sprite renderer`.

### Task 21: The VS title card

**Files:**
- Create: `studio/crates/bf-picture/src/title/{mod,layout,panels,words,fx}.rs`,
  `studio/crates/bf-picture/tests/title_frames.rs`

**Interfaces:**
- Consumes: `Show`, `Grid`, fighter colours, `SpriteCache`, `Look::title_pattern` and `Look::title_closeup`
  (declared here as a minimal trait and completed in Task 22).
- Produces: `pub struct TitleCard`; `TitleCard::new(show: &Show, colors: [[f64; 3]; 2], darks: [[f64; 3]; 2],
  looks: ..., series_tag: Option<&str>) -> TitleCard`; `pub fn door_offset(&self, t: f64) -> f64`;
  `pub fn draw(&self, img: &mut Rgb8, t: f64)`; `pub fn cover_frame(&self) -> usize` = the last frame with
  `door_offset == 0.0` and zero card shake (`ceil(t_doors.0 * 30) - 1`, stepped back while `shake(t) != 0`). Port of
  `projects/ballfight/title.py`, including the TROMBONE's `_slide_tubes` panel pattern and the plain panel.
- [ ] **Step 1: Write the failing test** `title_frames.rs`: `draw` over the same seeded-noise image reproduces
  each of the 16 PNGs in `reference/frames/flute_vs_trombone/title/` exactly; `door_offset` is 0 at the hold and
  `cover_frame()` is a frame inside the hold.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-picture: VS title card`.

### Task 22: The ring renderer and the `Look` trait

**Files:**
- Create: `studio/crates/bf-picture/src/{scene.rs,look.rs}`, `ring/{mod,state,arena,canvas,ball,ropes,shards,
  post_fx,frame}.rs`, `studio/crates/bf-picture/tests/ring_stages.rs`

**Interfaces:**
- Produces:
  ```rust
  pub trait Look: Send + Sync { /* one method per hook of kit/fighters/base.py Look, same names, same defaults,
      with &PictureCtx in place of the Python module P, plus the class attributes as methods:
      glass, flare_scale, shards, spark_big, spark_flash, flash_hits, fringe_hits, quiet_hits, teleports,
      speed_lift, rope_kinds, sprite_k, block_burst, block_spark */ }
  pub struct DefaultLook;            // every default, as the Python base class
  pub struct Picture { /* recording, show, effects built once, looks, sprite cache, title card */ }
  impl Picture {
      pub fn new(rec: Arc<Recording>, ff: &FightFile, show: Show, cfg: [&FighterCfg; 2], meshes: [Mesh; 2],
                 looks: [Box<dyn Look>; 2]) -> Result<Picture, PictureError>;
      pub fn frame(&self, f: usize) -> Rgb8;                    // 360x640, a pure function of f
      pub fn frame_logged(&self, f: usize) -> (Rgb8, DrawLog);  // for the safe-zone check
      pub fn stage(&self, f: usize, stage: Stage) -> RgbF;      // the buffer after a pipeline stage
      pub fn focus_points(&self, f: usize) -> Vec<(f64, String)>;   // CoC per subject point, for the focus check
      pub fn nf(&self) -> usize; }
  pub fn upscale3(img: &Rgb8) -> Rgb8;                           // 1080x1920 nearest
  ```
- Ports `projects/ballfight/scene.py` and `kit/picture.py`: the module-level derivations, `fighter` (the per-ball
  picture state in the `rev`, `fight` and `tail` phases), `camera`, `lights`, `rope_points`, `_shards` and
  `parts_at` (seeded by `shard_seed`), `sprite`, `draw_ball`, `arena` (stands, crowd, phones, flashbulbs),
  `canvas_shadows`, `sparks`, `_fx_build` (seeded by `fx_seed`), `shimmers`, `post_fx`, `countdown`, `status`,
  `ability_rows`, and `render(f)` with every hook call in the order the Python file makes it.
- [ ] **Step 1: Write the failing tests.** `ring_stages.rs` (needs the local stage dumps; `#[ignore]` when
  `studio/reference/.cache/stages` is absent): with `DefaultLook` for both fighters, the `floor`, `arena` and `skirt`
  stages of frames 150, 600 and 1,500 equal Python's exactly after the dither (they do not depend on the fighters'
  layers). `frame_is_pure`: frames 300 and 301 rendered in either order and on 1 or 8 threads are byte-identical.
  `no_panic_sweep`: every 15th frame renders.
- [ ] **Step 2:** Generate the dumps (`python -m tools.studio_export stages 150 600 1500`). Run: FAIL.
- [ ] **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-picture: ring renderer and Look trait`.

### Task 23: FLUTE look

**Files:**
- Create: `studio/crates/bf-content/src/fighters/{mod.rs,flute/{mod,look}.rs}`

**Interfaces:**
- Produces: `pub struct FluteLook` implementing `Look`, the port of the `Look` half of `kit/fighters/flute.py`
  (baskets, cobras, the PICCOLO ring, SNAKE DANCE, FLUTTER TONGUE, its shards, sparks and title close-up);
  `bf_content::look_for(name: &str) -> Box<dyn Look>`.
- [ ] **Step 1: Write the failing test** `flute_golden` in `studio/crates/bf-content/tests/golden_frames.rs`:
  every frame in `frames.json` tagged FLUTE (and not tagged TROMBONE) matches its PNG exactly, or is within the
  exceptions bar and listed. The test prints, per failing frame, the count of differing pixels, the largest
  difference and the bounding box.
- [ ] **Step 2:** FAIL. **Step 3:** Implement; use the stage dumps to find the first stage that differs.
- [ ] **Step 4:** PASS. **Step 5:** Commit `bf-content: FLUTE look`.

### Task 24: TROMBONE look and the full golden-frame gate

**Files:**
- Create: `studio/crates/bf-content/src/fighters/trombone/{mod,look}.rs`; extend `tests/golden_frames.rs`

**Interfaces:**
- Produces: `pub struct TromboneLook`, the port of the `Look` half of `kit/fighters/trombone.py` (the slide lance,
  SAD TROMBONE's shockwave, POWER SLIDE, PEDAL TONE's slam, its shards and title pattern);
  `pub fn build_picture(rec: Arc<Recording>, ff: &FightFile) -> Result<Picture, ContentError>` in `bf-content`.
- [ ] **Step 1: Write the failing test** `all_golden_frames`: every frame in `frames.json` for the imported
  episode recording matches (exactly, or within the bar and listed in `exceptions.toml` with a reason).
  `exceptions_are_within_the_bar` fails if a listed frame differs by more than 2 levels or on more than 0.5 % of
  pixels, or if a listed frame now matches exactly (stale entry).
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS. Save a contact sheet of every excepted frame
  beside Python's for the measurements report.
- [ ] **Step 5:** Commit `bf-content: TROMBONE look; golden frames pass`.

### Task 25: Sound foundations: DSP, generators, mixer

**Files:**
- Create: `studio/crates/bf-sound/src/dsp/{mod,iir,butter,resample,fft,conv,npstat}.rs`, `synth.rs`, `mix.rs`,
  `wav.rs`, `studio/crates/bf-sound/tests/dsp_vectors.rs`

**Interfaces:**
- Produces:
  ```rust
  pub const SR: usize = 48_000; pub const NA: usize = 2_928_640;
  pub type Sos = Vec<[f64; 6]>;
  pub fn butter_sos(kind: FilterKind /* Lp, Hp, Bp */, freqs: &[f64], order: usize, fs: f64) -> Sos;   // scipy butter(..., output='sos')
  pub fn sosfilt(sos: &Sos, x: &[f64]) -> Vec<f64>;  pub fn lfilter(b: &[f64], a: &[f64], x: &[f64]) -> Vec<f64>;
  pub fn resample_poly(x: &[f64], up: usize, down: usize) -> Vec<f64>;   // scipy defaults (Kaiser 5.0)
  pub fn fftconvolve(a: &[f64], b: &[f64]) -> Vec<f64>;                  // own radix-2 FFT, scalar, fixed order
  pub fn interp(x: &[f64], xp: &[f64], fp: &[f64]) -> Vec<f64>;  pub fn percentile(x: &[f64], q: f64) -> f64;
  pub fn convolve_same(a: &[f64], b: &[f64]) -> Vec<f64>;
  // synth.rs: reelkit/audio.py: tt, mtof, hp, lp, bp, adsr, expenv, saw, square, tri, kick, snare, clap, hat, crash,
  // whoosh, thud, pluck, pad, bass, chime, reverb, tape_stop (same names, same default arguments)
  pub struct Mix { /* named stereo buses, BTreeMap */ }
  impl Mix { pub fn new(dur: f64) -> Mix; pub fn place(&mut self, bus: &str, sig: &Sig, t: f64, gain: f64, pan: f64);
      pub fn bus(&self, name: &str) -> Option<&[[f64; 2]]>; pub fn bus_names(&self) -> Vec<&str>; }
  pub enum Sig { Mono(Vec<f64>), Stereo(Vec<[f64; 2]>) }
  pub fn write_wav16(path: &Path, x: &[[f64; 2]]) -> io::Result<()>;     // clip, * 32767, truncate, as reelkit.audio.write
  ```
  Seeded noise uses `Generator::new(seed).fill_normal`. The FFT must not use a SIMD or runtime-dispatched library.
- [ ] **Step 1: Write the failing test** `dsp_vectors.rs`: SOS coefficients within 1e-12 relative; `sosfilt`,
  `lfilter`, `resample_poly`, `interp`, `percentile`, `convolve_same` and every generator below -120 dBFS of
  difference (`20 log10(max |a - b|) < -120`); `fftconvolve` and `reverb` below -100 dBFS. `Mix::place` unit tests:
  equal-power pan `cos`/`sin` of `(pan + 1) pi / 4`, negative start times trimmed, `round(t * SR)` half-even.
- [ ] **Step 2:** FAIL. **Step 3:** Implement from scipy's source (`butter` through `zpk` and `zpk2sos` with its
  pairing order; `sosfilt` direct form II transposed). **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-sound: DSP, generators, mixer`.

### Task 26: Voices, music helpers and the master

**Files:**
- Create: `studio/crates/bf-sound/src/{voice.rs,music.rs,traits.rs,master.rs}`,
  `studio/crates/bf-content/src/fighters/{flute,trombone}/voice.rs`, `studio/crates/bf-sound/tests/voices.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct PpNoise(pub Generator);   // the Pixel Pitch voices' shared noise stream; PpNoise::start() resumes from
                                       // the exported pp_noise_start.json constants, once per soundtrack build
  pub trait Voice: Send + Sync { fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64>; }   // 48 kHz, as pp_bridge.voice
  pub struct MusicCtx<'a> { pub show: &'a Show, pub rec: &'a Recording, pub ff: &'a FightFile, pub harmony: Harmony,
      pub cfg: [&'a FighterCfg; 2], pub voices: [Arc<dyn Voice>; 2], /* note cache, the PpNoise stream */ }
  // music.rs: kit/music.py helpers with the same names: q16, hp_at, h01, Thinner, h01_37, thin_37, fit, inst_gain,
  // note, gliss, bend, vibrato, wah, colour, clank, hiss, roar, thunder, riser, line_override, shared_events
  pub trait Sound: Send + Sync { /* kit/fighters/base.py Sound: bus_gain, send, band, colour_args, voice, detune,
      colour, prepare, title_blast, count_in, play, fall_apart, stinger, solo, reassemble, last_bar */ }
  pub trait Bed: Send + Sync { /* a theme's classic arrangement: buses, reverb, comp, roar_gain, pop, doors_gain,
      landing, and title, vs, title_rest, count_in, fight_start, fight, clash, ko, stinger, after_ko, victory, last_bars */ }
  pub struct Soundtrack { pub master: Vec<[f64; 2]>, pub stems: BTreeMap<String, Vec<[f64; 2]>> }   // each NA long
  pub fn master(mix: &Mix, levels: &BTreeMap<String, (f64, f64)>, bed: &dyn Bed, ceiling: f64 /*0.89*/) -> Soundtrack;
  ```
  FLUTE's voice is `flute60_lowpoly.synth` for the scene `pp_bridge.scene("FLUTE")` selects (its first scene, key
  `modern`: `flute_tone` with that tone, `seed = 0`, plus the two echoes at `0.75 * BEAT_S` of that module, 112 BPM);
  the breath noise is `lp(noise.normal(0, 1, n), 2500)` with the series' own `lp` (`butter(2, ...)` + `lfilter`, in
  `drums60_lowpoly.py`). TROMBONE's is `trombone60_lowpoly.synth` for its scene (`brass2`, glide 0 for a lone note).
  Both are made at 44,100 Hz and passed through `resample_poly(x, 160, 147)`. Because the noise stream is shared and
  stateful, notes are synthesised one at a time in the order the score first asks for them, never in parallel, and
  `music::note` caches by `(name, midi, round(dur, 3), cap, round(vel, 2))` exactly as the kit does. `master` is `kit/music.py master()`: bus gains and sends, reverb,
  the fold of the overflow past `NA` onto the start, the compressor envelope primed with the last second, the
  99.97th-percentile scale, `tanh` limiter, final peak scale; stems carry the same gain.
- [ ] **Step 1: Write the failing test** `voices.rs`: every voice note (each started from the generator state
  stored with it) and every helper output in `reference/sound/voices/` below -120 dBFS of difference; `master` on a synthetic three-bus mix exported by
  `tools/studio_export/stems.py` (add that case) below -110 dBFS.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-sound: voices, music helpers, master`.

### Task 27: Big top bed, the two fighters' parts and the full soundtrack

**Files:**
- Create: `studio/crates/bf-sound/src/score.rs`, `studio/crates/bf-content/src/themes/{mod.rs,big_top/{mod,
  classic}.rs}`, `studio/crates/bf-content/src/fighters/{flute,trombone}/sound.rs`,
  `studio/crates/bf-content/tests/stems.rs`, `studio/reference/sound/exceptions.toml` (empty list)

**Interfaces:**
- Produces: `pub fn score(ctx: &MusicCtx, bed: &dyn Bed, sounds: [&dyn Sound; 2]) -> Mix` (the port of
  `kit/music.py score()`, every call in its order, including the door noise from `Generator::new(93)` and the caption
  landing thuds from `impact_landings`); `BigTopClassic` (`kit/moods/big_top.py`); `FluteSound`, `TromboneSound`;
  `bf_content::build_soundtrack(rec: &Recording, ff: &FightFile) -> Result<Soundtrack, ContentError>`;
  `bf_content::theme_names() -> Vec<&'static str>` (`["big_top"]` in M1); `bf_content::validate(ff: &FightFile) ->
  Result<(), ContentError>` (calls `FightFile::validate`, then checks the theme exists and voices every chord).
- [ ] **Step 1: Write the failing tests** on the imported episode recording: `stem_windows` (for every bus and
  every window in `stems.npz`, the difference is below -120 dBFS, or below -80 dBFS for buses listed in
  `reference/sound/exceptions.toml` with a reason); `stem_rms_profile` (every 100 ms block of every bus within
  0.01 dB of `rms.json`, blocks below -90 dBFS skipped); `master_matches` (against `audio.wav`: every sample within
  3 LSB of 16 bit, RMS of the difference below -90 dBFS); `length_is_whole_aac_frames` (`master.len() ==
  2_928_640`); `loops_cleanly` (the first and last 4 ms follow the kit's fold: no fade at the seam beyond what the
  Python file has); `note_order_matches` (the sequence of first-time note syntheses equals `note_calls.json`);
  Review Focus 2: `unknown_theme_is_an_error` (`music.theme: "polka" is not a theme`) and
  `unvoiced_chord_is_an_error` (`music.chord_loop[4]: big_top has no voicing for "F#m"`).
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS. Record the soundtrack build time.
- [ ] **Step 5:** Commit `big top bed, FLUTE and TROMBONE parts, full soundtrack parity`.

### Task 28: The checks

**Files:**
- Create: `studio/crates/bf-export/src/checks/{mod,levels,band,loudness,flash,loopcheck,safezone,focus}.rs`,
  `studio/crates/bf-export/tests/checks_parity.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct CheckResult { pub name: &'static str, pub pass: bool, pub summary: String, pub numbers: BTreeMap<String, f64> }
  pub fn levels(master: &[[f64; 2]]) -> CheckResult;                       // reelkit/levels.py, WIN = 2.0 s
  pub fn band(st: &Soundtrack, show: &Show, sounds: [(&str, (f64, f64)); 2], bed_buses: &[&str]) -> CheckResult;
  pub fn loudness(master: &[[f64; 2]], show: &Show) -> CheckResult;        // BS.1770 K-weighting, intro minus fight
  pub fn flash(lum: &[Img<f64>]) -> CheckResult;                           // reelkit/flashcheck.py analyse; pass = worst <= 3
  pub fn flash_span(show: &Show, story: &Story, low_names: [&str; 2]) -> (usize, usize);   // the busiest stretch, kit/checks.py flash()
  pub fn loop_picture(tail: &[Rgb8], head: &[Rgb8]) -> CheckResult;  pub fn loop_audio(master: &[[f64; 2]]) -> CheckResult;
  pub fn safezone(logs: &[(f64, DrawLog)]) -> CheckResult;                 // every 0.25 s
  pub fn focus(pic: &Picture) -> CheckResult;                              // pass = worst CoC == 0
  pub fn pre_render(pic: &Picture, st: &Soundtrack, show: &Show, story: &Story, workers: usize) -> Vec<CheckResult>;
  ```
  A failed check's `summary` names the check and the measured numbers in plain words (§9).
- [ ] **Step 1: Write the failing test** `checks_parity.rs`: run each check on the Python inputs (`audio.wav`, the
  Rust stems already proven equal, the Rust frames already proven equal) and compare with
  `reference/checks/flute_vs_trombone.json`: every verdict equal; levels and loudness numbers within 0.01 dB; band
  within 0.1 dB; flash counts equal and largest area within 0.1 point; loop numbers within 1 %; focus worst CoC
  equal; safe zone pass with the same number of tracked boxes. Negative tests: a soundtrack with its first 2 s
  scaled by 0.5 fails `levels` and the summary contains "opening"; 4 full-white frames in one second fail `flash`;
  a caption box moved to x = 345 fails `safezone`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-export: series checks with parity tests`.

### Task 29: Encode, verify and the render job

**Files:**
- Create: `studio/crates/bf-export/src/{encode.rs,probe.rs,output.rs,job.rs}`,
  `studio/crates/bf-export/tests/job.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn find_ffmpeg() -> Result<Ffmpeg, ExportError>;      // env BALLFIGHT_FFMPEG, else PATH; needs libx264 and aac
  pub struct Encoder { /* child process, stdin pipe */ }
  impl Encoder { pub fn start(ff: &Ffmpeg, wav: &Path, out: &Path, nf: usize) -> Result<Encoder, ExportError>;
      pub fn push(&mut self, frame: &Rgb8) -> Result<(), ExportError>; pub fn finish(self) -> Result<(), ExportError>; }
  pub fn verify_mp4(ff: &Ffmpeg, mp4: &Path) -> Vec<CheckResult>;   // size, length, streams, loop and flash on decoded frames
  pub fn folder_name(names: [&str; 2], seed: u64, root: &Path) -> PathBuf;   // flute_vs_trombone, else flute_vs_trombone_seed<N>
  pub struct JobOpts { pub out_root: PathBuf, pub workers: usize, pub frames: Option<Range<usize>> /* clips for tests */ }
  pub enum JobEvent { Stage(&'static str), Progress { done: usize, total: usize }, Check(CheckResult), Done(PathBuf) }
  pub fn run_job(ff: &FightFile, rec: Arc<Recording>, opts: &JobOpts, on: &(dyn Fn(JobEvent) + Sync),
                 cancel: &AtomicBool) -> Result<PathBuf, ExportError>;
  ```
- **Encode arguments, exactly:** `-y -loglevel error -f rawvideo -pix_fmt rgb24 -s 360x640 -r 30 -i pipe:0 -i
  <audio.wav> -vf scale=1080:1920:flags=neighbor -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -profile:v high
  -tune animation -x264-params mbtree=0 -force_key_frames expr:eq(n,0)+eq(n,<nf-1>)`, then the audio and container
  arguments exactly as `projects/maze_race/mux.py main()` continues after that line (AAC 256k at 48 kHz, soundtrack
  untrimmed). Frames are drawn in parallel in order-preserving batches and written in order; raw frames are never
  written to disk.
- **The job** (§9): build the soundtrack, pre-render checks, draw and encode, verify the mp4, write `cover.png`
  (`upscale3` of `TitleCard::cover_frame`), `caption.txt` (caption, then Instagram, TikTok and Shorts tags) and the
  fight file, all inside `<out_root>/.tmp-<name>-<pid>/`, then rename to `folder_name(...)`. A failed check or a
  cancel removes the temp folder.
- [ ] **Step 1: Write the failing tests** (they skip with a printed notice if ffmpeg is absent, except the first):
  Review Focus 3: `missing_ffmpeg_is_a_clear_error` (`BALLFIGHT_FFMPEG=/nonexistent`: the error text contains
  "ffmpeg" and the path); `failing_ffmpeg_leaves_nothing` (a stub script that exits 1: `run_job` errs and `out_root`
  is empty); `unwritable_root_is_a_clear_error`; `second_render_gets_seed_suffix` (pre-create
  `flute_vs_trombone/`: the result folder is `flute_vs_trombone_seed392`); `cancel_leaves_nothing`. And
  `clip_is_valid`: frames 0 to 90 of the episode encode to an mp4 that `ffprobe` reports as h264 1080x1920 at 30 fps
  with an aac 48 kHz stereo stream, whose first and last video frames are keyframes.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `bf-export: encode, verify, render job`.

### Task 30: The `ballfight` command line and the end-to-end proofs

**Files:**
- Create: `studio/crates/bf-cli/src/main.rs`, `cmd/{sim,story,search,balance,frame,sound,check,render,hash}.rs`,
  `studio/crates/bf-cli/tests/e2e.rs`; modify `.github/workflows/studio.yml`, `studio/README.md`

**Interfaces:**
- Produces the commands (all take a fight file; `--rec <file.bfrec>` uses an existing recording instead of
  simulating; `--workers N` defaults to cores minus one; `--json` prints machine-readable output):
  ```
  ballfight sim <fight.toml> [--seed N] --out <file.bfrec>
  ballfight story <fight.toml> [--seed N | --rec F]
  ballfight search <fight.toml> [--from 1] [--stop 200] [--max N] [--out shortlist.json]
  ballfight balance <fight.toml> [--write]          # --write stores the handicap in the fight file
  ballfight frame <fight.toml> --frame N --out f.png [--up] [--rec F]
  ballfight sound <fight.toml> --out audio.wav [--stems DIR] [--rec F]
  ballfight check <fight.toml> [--rec F]
  ballfight render <fight.toml> --out <folder> [--seed N] [--rec F] [--frames A:B]
  ballfight hash <fight.toml> --seeds 1,2,3
  ```
  Exit code 0 on success, 1 on a failed check or bad input, with the reason on stderr in one line.
- [ ] **Step 1: Write the failing tests** `e2e.rs` (release; the long ones `#[ignore]` by default and run by name):
  - `imported_episode_renders` : `render fights/flute_vs_trombone.toml --rec reference/.../ep392.bfrec` produces the
    folder with the mp4, `cover.png` (1080x1920), `caption.txt` and the fight file; every check passes; `ffprobe`
    reports 61.01 s.
  - `native_fight_renders`: `search --stop 3` on the same fight file with `seed` removed, then `render --seed
    <best>`: every check passes. This is the milestone's proof that a Studio-native fight makes a passing video.
  - `render_is_reproducible`: two `sound` runs and two `frame` runs (frames 0, 500, 1,700) with 1 and 8 workers give
    identical bytes, and their SHA-256 values for the imported episode equal `studio/replay/render_hashes.txt`
    (blessed once on Linux; the Windows job runs this test too, which is the cross-platform proof for picture and
    sound).
  - Review Focus 1: `no_ko_seed_is_refused` (find a seed with no K.O. by scanning with handicap `[0.05, 0.05]`;
    `render` exits 1 with "seed N has no K.O. before 58 s"; `story` still prints its summary).
  - `bad_fight_file_exit_code` (a fight file naming SAX exits 1 with the Task 7 message).
  - Review Focus 5: `other_tempos_work` (the same fight file at `tempo_bpm = 120` and at 150, 30 and 38 bars:
    `search --stop 1`, then `check` passes and `render --frames 0:60` succeeds).
- [ ] **Step 2:** FAIL. **Step 3:** Implement with `clap`. **Step 4:** PASS, including the two long renders.
- [ ] **Step 5:** Extend the Windows CI job: the `render_is_reproducible` test, then `ballfight render
  studio/fights/flute_vs_trombone.toml --rec ... ep392.bfrec --frames 0:90 --out out` (ffmpeg from the runner image
  or a setup action). Commit
  `bf-cli: the ballfight command line; end-to-end renders`.

### Task 31: Measure, report and close the milestone

**Files:**
- Create: `studio/crates/bf-cli/src/cmd/bench.rs`, `docs/studio/m1-measurements.md`
- Modify: `docs/studio/HANDOFF.md` (status section: the Studio exists, where it lives, how to run it),
  `studio/README.md`

**Interfaces:**
- Produces: `ballfight bench <fight.toml> [--rec F] [--json]` printing: fights per second (record off, all workers,
  2,000 seeds); milliseconds per frame single-threaded (mean and 95th percentile over every 10th frame) with a
  per-stage breakdown; full 1,830-frame draw time with all workers, with and without the encode; soundtrack build
  time; CPU model and worker count.

- [ ] **Step 1:** Implement `bench`. Run it in the build environment and, through the access the user granted, on
  the user's PC (Ryzen 7 9800X3D) from a checkout in `F:\!Ai Stuff\InstrumentBattleMachine`: `cargo build --release`
  then `ballfight bench` and one full `ballfight render` there.
- [ ] **Step 2:** Write `m1-measurements.md`: both machines' numbers against the targets in §7 (live playback at
  30 fps means under 33 ms per frame per core with headroom; full render under 60 s); the prefix-agreement window per
  reference fight; the statistics comparison table; the list of frame exceptions with images; the list of stem
  exceptions; each Pillow or scipy primitive that needed special handling; the reference set's size.
- [ ] **Step 3:** If a target is missed, add a "Speed plan" section naming the three most expensive stages from the
  breakdown and the change proposed for each (for example caching per-frame-constant buffers, tiling the canvas
  pass). Do the changes that keep frames bit-identical now; list the rest for the milestone 2 plan.
- [ ] **Step 4:** Run the whole suite once: `cargo test --workspace --release -- --include-ignored` on Linux; CI
  green on both jobs; `python tools/ballfight_replay_hash.py` unchanged from before the milestone.
- [ ] **Step 5:** Commit `milestone 1: measurements and handoff notes`. Then write the milestone 2 plan (the app)
  from §4, §11 and the mock-ups, using the measured numbers.

---

## What milestone 1 proves, mapped to the spec

| Spec requirement | Where it is proven |
|---|---|
| §6 determinism rules | Tasks 1, 2, 4 (vectors), 14 (replay hashes on Linux and Windows), 30 (`render_is_reproducible`) |
| §6 proofs 1 to 4 | Task 14 (prefix, coverage, self-replay), Task 16 (statistics) |
| §6 search, balance, filters | Task 16 |
| §7 identical frames, exceptions file | Tasks 17 to 24 |
| §8 stems at -120 dB, classic arrangement | Tasks 25 to 27 |
| §9 checks, encode, output folder, cover, captions | Tasks 28, 29 |
| §12 reference tools and manifest | Tasks 3, 6 to 10 |
| §13 CI on Linux and Windows | Tasks 1, 14, 30 |
| §14 milestone 1 measurements | Task 31 |

Not in this plan, by design: the queue's persistence and reordering, the Library, the per-search-key shortlist
cache, caption drafting from emoji and tags, live preview and audio playback (milestone 2); the music system (milestone 5); ffmpeg download and the Update
button (milestone 6).
