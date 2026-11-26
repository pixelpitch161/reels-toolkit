# Voice Engine 2, stage 1b: the built piano and organ, implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** PIANO and ORGAN play, under the Composed arrangement, the instruments Aedan picked by ear on 2026-11-26:
the piano P16 (simulated strings struck by a felt hammer) and the organ O8 (pipes built stop by stop), in place of the
stage 1 recipe voices.

**Architecture:** One engine per instrument family (Aedan's idea, 21:01Z): a struck-string engine (`strings`, with
its soundboard in `board`) and a pipe engine (`pipes`), beside the stage 1 recipe engine, which stays as it is. Both
are ports of the listening sketches that made P16 and O8 (`/mnt/project-files/ballfight-studio/voice-engine-2/
built4/sketches/wgpiano_main.rs` and `wg.py`; `built2/sketches/organ2.py`), to the app's deterministic maths
(`bf_core::dmath`, `bf_core::rng::Generator`). `engine2::play` keeps its signature: one mono note at 48 kHz.

**Tech stack:** Rust (bf-sound, bf-content, bf-harness), no new dependencies.

**Spec:** `docs/studio/2026-11-26-voice-engine-2-design.md`, with its "Revision: built engines" section (this
plan's Task 0).

## Global constraints

- Composed only: Classic stays byte-identical (`classic_golden`).
- Tests never run in the cloud: `cargo build`, `cargo test --no-run`, `cargo fmt --all` and per-crate clippy only;
  every test runs on Aedan's PC through `PC-Tests:` lines (at most 10 a push).
- Licences: nothing recorded is shipped by the built voices (they are models); the stage 1 recipes stay CC0.
- The sketches are the reference: the port must measure like them (octave bands of the riff within 1 dB, the high
  notes' overtone balance within 2 dB), checked by rendering through the throwaway `ve2_riff` example and measuring
  in the sketches' own tools.
- Speed: a Composed fight with PIANO or ORGAN within the design's budget (+25% over the stage 0 render).

## Review focus

1. A note asked for with a zero or tiny length, or a cap shorter than the hammer's contact: a short, finite note.
2. MIDI notes at the ends of the 88 keys (21, 108) and between keys (a bend's fraction): finite, in tune, no panic.
3. A velocity of 0 or over 1, an accent of 1: finite, loudness capped, the hammer still leaves the strings.
4. The same note asked for twice gives the same samples (the cache depends on it); its repeats (variants) differ.
5. A big organ chord (`together` 6) and a swell with `over` 0: no step, no NaN.

---

### Task 0: Design revision and this plan

**Files:** Modify `docs/studio/2026-11-26-voice-engine-2-design.md` (a "Revision: built engines (2026-11-26)" section:
Aedan's rule of 19:50Z, the picks O8 and P16, one engine per family with a listening tool each, the recipe engine
kept but no fighter on it). Create this plan.

- [ ] Write both, commit.

### Task 1: The struck-string engine

**Files:** Create `studio/crates/bf-sound/src/voice/engine2/strings.rs`, `.../engine2/board.rs`.

**Interfaces:**
- Produces: `pub struct StringsParts { pub hard: f64, pub detune: f64, pub treble_detune: f64, pub treble_fade: f64,
  pub felt: f64, pub strike: f64, pub undamped_from: i32, pub damper: f64, pub glide: f64 }` (P16: 1.0, 0.6, 1.2,
  25.0, 0.8, 0.118, 89, 1.0, 1.0); `pub fn note(p: &StringsParts, play: &Play, len: usize, variant: u64) -> Vec<f64>`;
  `pub fn contact(midi: f64) -> f64` (the register's hammer contact, seconds); `board::apply(x: &[f64]) -> Vec<f64>`.
- The model (as the sketch): per string, modes `k` with `f_k = k f0 sqrt(1 + B k^2)` up to 14 kHz; a mass-on-felt
  hammer `F = K d^p (1 + felt d')`, `K` found per key so the contact at strength 0.6 lasts `contact(midi)`; the key's
  strings joined at the bridge (in-step motion lost to it at the prompt rate, the rest at the aftersound rate);
  tension rising with the swing; felt dampers over 20 ms from the key's release (none from `undamped_from`).
- Board: 1400 seeded resonances 45 Hz to 10 kHz, each as damped as a real soundboard's (Q about 35 to 70), direct 1.0
  plus 0.45 of them, 0.5 s; then low shelf 200 Hz -2.5 dB, peak 1500 Hz -3 dB, high shelf 3 kHz +4 dB, high-pass 30 Hz.
- Playing: strength `vel + 0.12 accent`; `Muted` (soft pedal) strikes one string fewer; `pedal` or an undamped key
  rings on; `Choked`/`Clipped` damps twice as fast at the release; repeats (`variant`) vary the strike (strength
  ±3%, strike point) and never the key's make-up.

- [ ] Unit tests first (in `strings.rs`): `the_hammer_leaves_within_its_registers_contact` (strength 0.6 at 36, 60,
  84: within 15% of `contact`), `a_harder_strike_is_louder_and_brighter`, `notes_are_in_tune` (40 to 96: the
  fundamental within 3 cents of the stretched tuning), `a_damped_key_dies_and_a_top_key_rings_on`,
  `repeats_differ_but_the_same_note_is_the_same`, `edge_notes_are_finite` (21, 108, 60.5, vel 0 and 1.2, len 1).
- [ ] Port, `cargo test --no-run`, clippy, render through `ve2_riff`, measure against the sketch.

### Task 2: The pipe engine

**Files:** Create `studio/crates/bf-sound/src/voice/engine2/pipes.rs`.

**Interfaces:**
- Produces: `pub struct PipeParts { pub wind_cents: f64, pub wind_db: f64, pub wind_hz: f64, pub sag_cents: f64,
  pub pedal_top: i32 }`; `pub fn note(p: &PipeParts, play: &Play, len: usize, variant: u64) -> Vec<f64>`.
- O8's registration (grand): manual Praestant 16', Bourdon 16', Principal 8', Octave 4', Quint 2 2/3', Superoctave
  2', Mixture IV; pedal (under `pedal_top` 43) Subbass 16', Octavebass 8', Choralbass 4'. Each pipe additive (its
  kind's spectrum, speech, the octave flick, chiff, breath, a fixed detune by pipe), the shared wind as
  `organ::waver` (notes that start together waver together) and a sag for `together >= 3`; a swell brings the stops
  in in that order; O8's studio polish EQ per note (low shelf 110 +2, peak 250 +1.5, peak 2800 +1, high shelf 9000
  +1.5). Mono: the stops' stereo places are left to the voice's `Space` width.

- [ ] Unit tests first: `each_stop_sounds_its_pitch` (a 16' gives an octave under), `pipes_speak_quickly_and_stop_
  quickly`, `a_swell_brings_the_stops_in`, `a_big_chord_sags_a_hair`, `the_same_note_is_the_same`, `edge_notes_are_
  finite`.
- [ ] Port, check, render, measure against the sketch.

### Task 3: Plug in, fighters, levels

**Files:** Modify `engine2/mod.rs` (`Parts::Strings`, `Parts::Pipes`; `Instrument::built(name, parts, space)` with no
recipe; `natural` per family), `bf-content/src/fighters/{piano,organ}/voice.rs` (ENGINE2 on the built engines;
spaces: piano width 0.35, tilt 0.12, a home of 2.0 s 15 LU under; organ width 0.45, home 3.2 s 14 LU under),
`bf-content/src/levels_table.rs` (by `ballfight levels --write`).

- [ ] Wire, build, rewrite the two level rows, render `piano_vs_organ` Composed, time it.

### Task 4: Tests

**Files:** Delete `studio/crates/bf-harness/tests/engine2_voices.rs` (it holds the recipe voices to their
recordings; no fighter plays them now). Create `studio/crates/bf-harness/tests/engine2_built.rs`: the riff's octave
bands inside the four real pianos' (±1.5 dB), the high notes' overtone balance against the real range (±4 dB), the
organ's held chord within the sketch's bands (±1.5 dB), both voices repeat-differing and in tune.

- [ ] Write, `--no-run`, PC-Tests lines.

### Task 5: Docs, page, push

- [ ] Instrument sheets ("Piano", "Organ") and `voice-engine-2-playing.md` for the built engines; HANDOFF.
- [ ] Fights and bands page (piano_vs_organ, the mambo and noir jazz bands, each today against stage 1b).
- [ ] Commit with PC-Tests lines, push, PR comment.
