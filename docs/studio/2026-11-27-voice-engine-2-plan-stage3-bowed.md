# Voice Engine 2, stage 3: the bowed strings — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** VIOLIN, VIOLA, CELLO, CONTRABASS and OCTOBASS play Aedan's picks from the bowed round (Vn3, Va3, Vc1, Cb1,
Oc2, 2026-11-27 04:48Z) on one bowed-string engine under the Composed arrangement, with every technique each has.

**Architecture:** a new family engine `bowed.rs`: a waveguide string (two delay lines meeting at the bow, each end a
lossy reflection) held by the bow's friction (a hyperbolic friction curve, the slip kept until it can stick again),
with the string's twist (torsional waves at 3.7 times the speed, which keep the Helmholtz motion clean), a "player's
ear" that keeps the note in tune, and a free ring once the bow leaves. The twist's wave speed is set per instrument
(violin 6.2, viola 6.4, cello 8.0, contrabass 7.0, octobass 8.5 times the string's) so that its resonances fall
between the note's overtones and every note bows in tune (at 9.0 the cello's upper notes, and at 6.8 the violin's top,
bowed about 40 cents flat and the player's ear scooped them up; the twist's bridge side may be as short as one sample,
a floor of two moved its echo at the top notes): at the round's 3.7 they sat on exact overtones of every note (the 7th, 15th and 22nd on
the violin, the 11th on the cello), a pattern that followed the melody like a synth playing along (Aedan's remark,
2026-11-27; measured with a note-locked overtone score against the CC0 solo violin). The
string runs at its own rate (192 kHz for the violin and viola, 96 kHz for the cello, the heard rate for the basses)
and is brought down with a windowed-sinc decimator. Plucks, snaps, the slap and col legno battuto play on the plucked
engine (`plucked.rs`) with the instrument's strings and body. The bodies join `body.rs`. Same shape as stage 2:
`Parts::Bowed`, `Instrument::built`, notes at any sample rate.

**Tech Stack:** Rust (bf-sound, bf-content, bf-harness); the sketch `bw.rs` and `bowed.py`/`round1.py` in
`/mnt/project-files/ballfight-studio/voice-engine-2/bow1/sketches/` are the reference for every number.

**Spec:** `docs/studio/2026-11-26-voice-engine-2-design.md` ("Revision: built engines"), the sheets in
`docs/studio/voice-engine-2-instruments.md`.

## Global Constraints

- Composed only: Classic stays byte-identical (`classic_golden`).
- Aedan's picks: Vn3 (warm violin), Va3 (big viola), Vc1 (concert cello), Cb1 (orchestral bass), Oc2 (octobass,
  growl); his note: no synth-like layer following the notes (the twist speeds above, and the free ring; no wobble of
  the bowing point, which sounded broken).
- His second note, no squeak in short notes (fix 4, sketch `round1.py` STOP and ACC): a stopped stroke lets its weight
  off with its speed (`stop_k` 0: weight left at zero speed pins the string like a fret and the pitch jumps two
  semitones); an accent comes with bow speed and weight together (`bite_v`), its bite lasts 20 ms, and a strong accent
  (0.6 and up) starts the bow quicker (`att_acc` 0.7; a light one keeps 0.4); accent bite 0.8 on the violin and viola,
  0.4 on the cello, the basses as before. Accents settle in about 25 to 35 ms.
- His third note (fix 5, the last for these): an open string is bowed with 15% more weight (`open_press` 1.15; with no
  finger it is less damped and slips twice a period at a weight a stopped string bows cleanly), on the violin, viola,
  cello and contrabass, not the octobass (pressed hard already; more creaks); the viola's accent bite 0.4, lasting
  12 ms.
- Every technique (Aedan, 2026-11-26 21:36Z) through `Play`; runs of notes stay the composition system's.
- The contrabass and the octobass keep today's rule for a plain note: plucked under half a second, bowed from there.
- VST later: no hard-coded 48 kHz; the string's rate follows the sample rate.
- No tests in the cloud; `PC-Tests:` lines only at the very end. Full checks run on GitHub's machines.
- Licences: the VSCO 2 CE recordings (CC0) were yardsticks only; nothing recorded ships.

## Review Focus

1. A bow force or speed the round never tried (a full accent at full velocity, a swell from nothing, an expression
   pedal shut): the string must not run away, go silent, or squeal for long.
2. Notes at the edges: under the lowest open string, far over the top, a harmonic on a short string: finite.
3. A slurred note (`slur_from`) and a tied one (`tie`): no click at the join, no gap, no doubled attack.
4. Another sample rate (44.1 kHz): the string's own rate and the decimator follow, the note stays in tune.
5. Speed: a fight's bowed parts within the +25% budget.

## Tasks

### Task 1: the playing interface (`play.rs`)
- `Technique` gains `Arco`, `Pizzicato`, `SnapPizz`, `LeftHandPizz`, `Spiccato`, `Martele`, `Tremolo`, `ColLegno`,
  `ColLegnoTratto`; the plucked engine plays them as its own normal pluck.
- `Play.tie: bool`: the next note is slurred on from this one (the bow doesn't stop; the note hands over).
- `position` on a bowed string is the bowing point (sul ponticello near 0.03, sul tasto near 0.25); `Mute::Muted` the
  mute (con sordino); a slur with a bend of 0 cents is a portamento over the bend's `over`.
- Both in `Play::key`; tests: keys differ with each.

### Task 2: the bodies (`body.rs`)
- `BodyKind::{ViolinWarm, ViolaBig, Cello, Contrabass, OctobassGrowl}` from `bowed.py` with round 1's variant
  offsets (Vn3 and Va3 WARM, Va3's modes at 0.92, Oc2 LOWMID).

### Task 3: the bowed-string engine (`bowed.rs`)
- `BowedParts` with the sketch's fields; presets `VN3`, `VA3`, `VC1`, `CB1`, `OC2` (with their twist speeds).
- A test that pins the fix: on held notes across each instrument's range, no overtone number stands out or drops out on
  every note (the note-locked score under 2 dB, as `nl2.py` measures it).
- Tests that pin fix 4: a held note's bowed pitch within 10 cents of the note across the range (ear off); an accent's
  pitch within 20 cents of the note from 50 ms on and its overtone balance that of one slip a period (the 2nd overtone
  under the fundamental); a martelé's stop never sounds above the note.
- `note(p, play, len, variant, fs)`, `rings_for`; unit tests: in tune; clean Helmholtz motion (one slip a period);
  harder louder and brighter; spiccato short; martelé stops; tremolo strokes; a harmonic sounds its overtone; pizz
  plucked; the mute darker; the free ring dies with its overtones; slur and tie joins without a click; the same note
  twice the same and variants differ; finite at the edges; 44.1 kHz.

### Task 4: the fighters
- `Parts::Bowed`; the five fighters get `ENGINE2 = Instrument::built(..)` with their picks and a `Space`; the levels
  table measured afresh; `engine2_built.rs` holds each one's round 1 riff (`tests/reference/bowed_riffs.txt`) to the
  sketch's octave bands within 2 dB.

### Task 5: docs and the review page
- Sheets for the five instruments, the playing guide's new techniques and `tie`, HANDOFF.
- Review page: fights (Violin vs Cello, Contrabass vs Octobass, Viola in a fight), a band with strings, and every
  technique per instrument.
