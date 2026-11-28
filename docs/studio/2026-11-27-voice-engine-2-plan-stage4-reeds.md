# Voice Engine 2, stage 4: the reeds — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** SOPRANO SAX, ALTO SAX, TENOR SAX, BARI SAX, CLARINET, BASS CLARINET, CONTRABASS CLARINET, OBOE, BASSOON,
CONTRABASSOON, BAGPIPES and ACCORDION play Aedan's picks from the reeds round (Ss2 As2 Ts2 Bs2 Cl1 Bc1 Cc1 Ob1 Bn1
Cf2 Bp1 Ac1, 2026-11-27 18:44Z) on one reed engine under the Composed arrangement, with every technique each has.

**Architecture:** a new family engine `reed.rs`, the sketch `rd.rs` ported number for number: a blown reed (a
mass-spring valve shut by the pressure across it, Bernoulli flow solved exactly each sample, beating on the lay) on a
modal bore (a cone's every resonance with the missing tip's comb, or a cylinder's odd ones, falling above the
tone-hole cutoff, the bore's own impedance over them, the mouthpiece's broad resonance), register vents, the player
(each note's fingering tuned by a trial run and voiced if it will not speak, the player's ear), breath noise, pad
knocks, radiation, formants; and the free reed (the accordion's banks). 96 kHz inside (twice the sample rate), a
windowed-sinc decimator. The sketch plays a whole line through one bore; the app plays a note at a time: each note is
the sketch's line of that note alone, a slurred one with the note it comes from as a short lead-in, a tied one handing
over to the next. `Parts::Reed`, `Instrument::built`, notes at any sample rate.

**Tech Stack:** Rust (bf-sound, bf-content, bf-harness); the sketch `rd.rs`, `reeds.py`, `round1.py`, `rriffs.py` in
`/mnt/project-files/ballfight-studio/voice-engine-2/reed1/sketches/` are the reference for every number.

**Spec:** `docs/studio/2026-11-26-voice-engine-2-design.md` ("Revision: built engines"); the sketch's `spec/sheets.md`.

## Global Constraints

- Composed only: Classic stays byte-identical. A Composed fight within +25% of before.
- Aedan's picks: the saxes' Jazz-rock version (2: brighter mouthpiece, a 2.7 kHz edge, more open reed, more
  breath), Cl1 French Boehm, Bc1 standard, Cc1 standard, Ob1 French, Bn1 Heckel, Cf2 rough buzzy, Bp1 Highland, Ac1
  musette (three reeds, +-14 cents).
- Every technique through `Play` (fa810e99's winds' ways); runs, trills and grace-note movements stay the
  composition system's.
- No tests in the cloud; tests compiled, their numbers checked by probe renders.
- Licences: nothing recorded ships.

## Review Focus

1. A per-note port of a line engine: a slurred and a tied note join without a click or a gap.
2. Breath the round never tried (a full accent at full velocity, a swell from nothing): finite, no runaway squeal.
3. Notes at the edges (under the lowest, far over the top, a forced register that will not speak): finite.
4. Another sample rate (44.1 kHz): in tune.
5. Speed: the tuning run is memoised; a fight within +25%.

## Tasks

### Task 1: the reed engine (`reed.rs`)
- `Reed` (the sketch's `Inst`), `ReedParts` (the reed, the bagpipes' drones, the player's own vibrato), presets for
  the twelve picks; the line kernel ported; `note`, `rings_for`; the Play mapping for every technique.
- Unit tests: in tune; louder harder; techniques change the sound; slur and tie join; finite at the edges; 44.1 kHz;
  same note twice the same, variants differ.

### Task 2: the fighters
- `Parts::Reed`; twelve `ENGINE2`s with their `Space`; the parts' pitched notes that bypass the voice play the engine
  under Composed (growls, honks, subtones as techniques); the bagpipes' drones one long note of their own under the
  chanter; levels measured afresh; `engine2_built.rs` holds each riff (`tests/reference/reed_riffs.txt`) to the
  sketch's octave bands within 2 dB.

### Task 3: checks, docs, commit, review page
- Port vs sketch on the twelve riffs; Classic md5; speed; fmt; clippy.
- Sheets in voice-engine-2-instruments.md, the reeds' table in voice-engine-2-playing.md, HANDOFF.md.
- Review page "Reeds in the App" (fights, a band, every technique).
