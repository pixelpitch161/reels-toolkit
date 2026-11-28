# Voice Engine 2, stage 5: the brass and flutes — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** TRUMPET, CORNET, FLUGELHORN, HORN, MELLOPHONE, TROMBONE, EUPHONIUM, TUBA, SOUSAPHONE, FLUTE and PICCOLO
play Aedan's picks from the brass and flutes round (Tp1, Co1, Fh1, Hn1, Me1, Tb1, Eu1, Tu1, Su1, Fl1, Pc1,
2026-11-27 18:44Z) on two built engines under the Composed arrangement, with every technique each has.

**Architecture:** two family engines. `brass.rs`: the player's lips a mass on a spring (inward striking, as the
sketch rules), the flow through them worked out every sample against the horn's input impedance (one resonance per
natural note, strongest round the mouthpiece's ring, gone above the bell's cutoff, the tube's own resistance over it),
the valves or slide scaling every resonance; the outgoing wave steepened over the cylindrical length (the blare), the
wall loss, the bell's two high-passes and its soft top, at 4x the sample rate (2x tuba and sousaphone) and brought
down with a windowed-sinc decimator; then the pulse limiter, the mutes and the horn's hand as filters (biquads at any
sample rate, `winds.py`'s corners, gains and levels), the soft limiter. The player model (each note's lip tension and
horn trim from trial runs, the standard fingerings, slurred notes re-tuned as played, the stop ease) comes along: the
trial runs are the port's own code, run once offline into a compiled-in table per pick (MIDI x the sketch's own
strength steps of 0.05), with the same code at run time for a note the table lacks. `flute.rs`: the jet-drive flute,
its tube's resonances, the jet's delay and split, the embouchure's level control, breath in the loop, every flute
technique; its ear (three trial passes a note, cheap) at run time, cached. One note at a time, at any sample rate:
the sketch plays a line in one breath; a note here is the sketch's line of that one note, a slurred note with the
note it slurs from as its lead-in and joined as the bowed strings join.

**Tech Stack:** Rust (bf-sound, bf-content, bf-harness); the sketch `br.rs`, `winds.py`, `round1.py`, `wriffs.py`
in `/mnt/project-files/ballfight-studio/voice-engine-2/brass1/sketches/` are the reference for every number.

**Spec:** `docs/studio/2026-11-26-voice-engine-2-design.md` ("Revision: built engines"); the sheets in
`brass1/spec/sheets.md`; the winds' ways in `docs/studio/voice-engine-2-playing.md`.

## Global Constraints

- Composed only: Classic stays byte-identical (`classic_golden`, md5 of three fights before and after).
- Aedan's picks (version 1 of every one): Tp1 orchestra C trumpet, Co1 brass band cornet, Fh1 flugelhorn, Hn1 double
  horn with the hand in the bell, Me1 marching mellophone, Tb1 orchestra tenor trombone with the F valve, Eu1
  euphonium, Tu1 orchestra C tuba, Su1 brass sousaphone, Fl1 silver flute, Pc1 silver piccolo. The approved sound is
  the round 1 page as heard (Tu1 and Su1 were heard before the sketch's last change, short low notes tuned over
  their heard part: the port follows what was heard).
- Every technique through `Play` (Aedan's rule); runs, trills, rips in a row stay the composition system's.
- Speed: a Composed fight within +25% of before, for every one of the eleven.
- VST later: no hard-coded 48 kHz. No tests run in the cloud: written, compiled, numbers checked by probe renders.
- Licences: nothing recorded ships.

## Review Focus

1. A note the round never tried (full strength with a full accent, a pedal note, the top of the range, a fall of two
   octaves): finite, no runaway squeal, no long silence.
2. A slurred note and a tied one: no click at the join, no gap, no doubled start.
3. The player model's table: a note between its steps or outside it plays in tune (the run-time tuner).
4. Another sample rate (44.1 kHz): the engines' own rates and the filters follow, notes stay in tune.
5. Speed: the trial runs never run for a fight's plain notes; a fight's brass within +25%.

## Tasks

### Task 1: the brass engine (`brass.rs`)
- `BrassParts` with the sketch's fields; presets `TP1` .. `SU1` (round1.py's settings over the sketch's presets).
- The line, the steepening, the bell, the decimator; the player's ear (trial runs), the fingerings, the slur re-tune;
  the pulse limiter, mutes (each mute's level against the open horn as `winds.py` sets it on the pick's riff), the
  hand, the soft limiter; the table (`brass_tune.rs`, written by the probe).
- Unit tests: in tune across the range; louder is brighter; every technique finite and changing the note; slur and
  tie joins without a click; mutes quieter and shaped; the same note twice the same, variants differ; 44.1 kHz.

### Task 2: the flute engine (`flute.rs`)
- `FluteParts`, presets `FL1`, `PC1`; the jet, the tube, the ear (cached); every flute technique; unit tests as above.

### Task 3: the fighters
- `Parts::Brass`, `Parts::Flute`; the eleven fighters get `ENGINE2 = Instrument::built(..)` with their picks and a
  `Space` (the sketch's light room: 2.6 s, 14 LU under; the dry in the middle); pitched notes the kits make outside
  the voice go through the engine; levels measured afresh; `engine2_built.rs` holds each pick's round 1 riff
  (`tests/reference/wind_riffs.txt`) to the sketch's octave bands within 2 dB.

### Task 4: checks, docs, the review page
- Port vs sketch on the eleven riffs; Classic md5; speed table; fmt; clippy.
- Sheets for the eleven, the playing guide's brass and flutes table, HANDOFF.
- Review page `stage5/`: Trumpet v Trombone, Horn v Tuba, Flute v Piccolo, a band with brass, every technique per
  instrument, the speed table and checks.
