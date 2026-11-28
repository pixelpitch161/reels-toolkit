# Voice Engine 2, stage 2: the plucked strings — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ACOUSTIC, ELECTRIC, BASS GUITAR, UKULELE and HARP play Aedan's picks from the plucked rounds on one
plucked-string engine under the Composed arrangement, with every technique each instrument has.

**Architecture:** a new family engine `plucked.rs` (a stiff string's modes, two polarisations, plucked at a point by a
finger or a pick, damped when let go), a body (`body.rs`: named low modes and many small ones, fitted to a target
average response) for the acoustic, ukulele and harp, pickups and a coil for the electric and the bass, and an amp
(`amp.rs`) that plays on the fighter's whole bus (as on a real amp, so a chord's notes distort together). Same shape as
stage 1b: `Parts::Plucked`, `Instrument::built`, notes at any sample rate in 32-sample blocks.

**Tech Stack:** Rust (bf-sound, bf-content, bf-harness); the sketch in Python (`/mnt/project-files/ballfight-studio/
voice-engine-2/pluck1/sketches/pl.py`, round 2 in `pluck2/sketches/`) is the reference for every number.

**Spec:** `docs/studio/2026-11-26-voice-engine-2-design.md` ("Revision: built engines"), the sheets in
`docs/studio/voice-engine-2-instruments.md`.

## Global Constraints

- Composed only: Classic stays byte-identical (`classic_golden`).
- Aedan's picks (2026-11-27 00:16Z): A1 acoustic, E2 electric, B3 bass, U1 ukulele, H3 harp; the acoustic and the
  ukulele brighter: round 2's A6 and U6 (00:28Z).
- Every technique (Aedan, 2026-11-26 21:36Z): what is one note's business goes through `Play`; strums, trills,
  tremolo picking, glissandos are the composition system's.
- VST later: no hard-coded 48 kHz in the engine; notes run in small blocks.
- No tests in the cloud; `PC-Tests:` lines only at the very end (Aedan, 2026-11-26 23:43Z: no merge or tests per
  family). Full checks run on GitHub's machines.
- Licences: nothing recorded ships; the harp and bass recordings (VSCO 2 CE, Karoryfer growlybass, both CC0) are
  yardsticks only.

## Review Focus

1. A bus amp at a level the sketch never saw: a fighter's notes are level-matched to the old voice, so the amp's
   drive must be set against that level, or E2 turns clean or fizzy.
2. A bend, slide or vibrato on a note whose modes change pitch per block: no zipper noise, the same note twice the same.
3. Notes outside an instrument's strings (a harp note under its lowest, a bass note over its top): finite and sensible.
4. A note cut short (`cap`) or held longer than it rings: ends without a click.
5. The amp on the winner's solo bus and in a fight where both fighters have amps: each bus its own.

## Tasks

### Task 1: the playing interface for plucked strings (`play.rs`)
- Techniques: `Fingers` (the flesh), `Picked` (a pick), `Nails`, `Thumb`, `Slap`, `Pop`, `Tapped`, `Hammer` (a
  hammer-on or pull-off: no new pluck), `PinchHarmonic`, plus the existing `Harmonic(n)`; `Normal` is the
  instrument's own way (the electric and bass picked, the rest fingers).
- New fields: `position: Option<f64>` (where along the string it is plucked, 0 at the bridge, 0.5 the middle: sul
  ponticello, près de la table), `string: Option<u8>` (which string a fretted note is played on, 0 the lowest).
- `slur_from` on a plucked string: the note slides from that pitch without a new pluck (`bend.over` sets the slide's
  time, 60 ms otherwise); with `Hammer` the pitch changes at once with a small tap.
- `mute`: `Muted` palm mute (harp: étouffé), `Choked` a dead note (ghost note, chunk), `Clipped` let go at once.
- Both new fields in `Play::key`; tests: keys differ with each.

### Task 2: the body (`body.rs`)
- `BodyKind` presets from the sketch (`dread_b` or `dread` as picked, `soprano_b`/`soprano`, `pedalharp`), the
  response made once per (kind, rate): named modes, the dense modes, then the minimum-phase fit to the target curve,
  then the high-pass. Test: the response's third-octave levels within 1.5 dB of the target, made once.

### Task 3: the plucked-string engine (`plucked.rs`)
- `PluckedParts` with the sketch's fields (string losses, stiffness, body drain, open strings, pluck point, width,
  softness, damping, ramp, knock, top frequency, pickups and coil, gain).
- `note(p, play, len, variant, fs)`: modes by complex phasors updated per 32-sample block (pitch and damping steps),
  the vertical and horizontal polarisations, the knock, the body or the pickups, an end fade.
- Unit tests: in tune; a harder pluck louder and brighter; a palm mute short and dull; a dead note shortest; a
  harmonic an octave up; a bend reaches its pitch; a slide from `slur_from`; the same note twice the same and
  variants differ; finite at the edges; another sample rate.

### Task 4: the amp (`amp.rs`) on the bus
- `Amp` presets `HIGH_GAIN` (E2) and `BASS_GRIT` (B3); `apply(x: &[[f64; 2]], amp, fs)`: 4x oversampled gain
  stages, tone stack, power stage, cabinet; its drive set against the voice's reference level; `Space.amp`.
- `polish::treated` runs a part's amp before its gain; the solo bus too.
- Tests: a quiet note stays near clean and a loud one distorts; two notes together make difference tones; a
  bus without an amp is untouched.

### Task 5: the fighters
- `Parts::Plucked`; ACOUSTIC, ELECTRIC, BASS GUITAR, UKULELE, HARP get `ENGINE2 = Instrument::built(..)` with their
  pick's parts and a `Space` (width, home room, amp for E2 and B3); level2 matching as stage 1b.
- `engine2_built.rs` grows a riff per instrument held to its sketch's octave bands (within 2 dB).

### Task 6: docs and the review page
- Sheets for the five instruments, the playing guide's new fields and techniques, HANDOFF.
- Review page: fights (Electric vs Bass Guitar, Acoustic vs Ukulele, Harp in a fight), the stadium rock band's
  acoustic strums, and every technique per instrument.
