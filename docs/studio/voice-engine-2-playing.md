# Voice Engine 2: how a note can be played

For Aedan's composition system. A Voice Engine 2 instrument is asked for a note with a **`Play`**
(`bf-sound/src/voice/engine2/play.rs`): the pitch and length, plus everything a player does with one note. A part asks
with `MusicCtx::note_played(voice, &play, cap)`. Under Classic, or for a voice that is not on Voice Engine 2 yet, the
same call plays the voice's ordinary note and leaves the rest of the `Play` aside, so asking never breaks anything.

An instrument takes what it can do and ignores the rest: a piano has no vibrato, an organ has no accent by touch. Stage
1 has the piano and the organ; later stages add each family's own (a wind's slur and vibrato, a string's bow, a
brass's mute and fall).

## The fields

| Field | In plain words | Piano | Organ |
|---|---|---|---|
| `midi` | Which note (60 is middle C; a fraction is between two keys). | yes | yes |
| `dur` | How long the key is held, in seconds. The note goes on sounding after it as the instrument does (a piano's damper, an organ's quick stop). | yes | yes |
| `vel` | How hard, 0 to 1. | strength: harder is brighter, as a real piano; how much louder follows the old voice (below) | how loud (a pipe sounds the same however the key is pressed, so this is only level) |
| `accent` | An accent on top of `vel`, 0 to 1: a harder, noisier start. | strikes harder (up to 0.12 more strength) with more of the hammer's knock | no |
| `slur_from` | Join this note to the one before (at that pitch) without a new start. | no (a piano always strikes) | not yet |
| `bend` | Slide the pitch by some cents, starting at some point, over some time (a bend, a fall, a scoop). | no | no |
| `swell` | Grow the strength from one value to another over the note's start: stops coming in, a crescendo. | no | yes: from the quiet setting towards the full chorus |
| `vibrato` | Wobble the pitch by some cents, so many times a second, after a wait. | no | no (the tremulant is not used) |
| `pedal` | The sustain pedal down: the note rings on past the key, and the other strings ring along softly. | yes | no |
| `mute` | `Open`; `Muted`; `Choked` (stopped dead at release); `Clipped` (as short as it goes). | `Muted` is the soft pedal (softer, darker); `Choked` and `Clipped` stop it twice as fast | `Choked` and `Clipped` use the quickest stop |
| `together` | How many notes sound at once, this one included. | no | a chord of 3 or more sags a hair in pitch, as the wind does under a big chord |

`Play::plain(midi, dur, vel)` is a note with none of the extras: open, on its own, no pedal.

## What today's parts pass by themselves

- Every fighter note that goes through `MusicCtx::note` (the attacks' riffs, the music's lines) is played as
  `Play::plain` with its pitch, length and strength, and its repeats take turns among three slightly different takes,
  as a real player never plays a note twice the same.
- The organ fighter's chords pass `together` (the chord's size) and, for CATHEDRAL, PULL ALL STOPS and the title blast,
  a `swell` from the quiet setting to the full chorus.
- Every note's loudness at its `vel` is the old voice's at that velocity (matched once per voice from 0.1 to 1, in
  the middle of its register), so the balance today's parts were written for stands; only the tone changes with
  `vel`. A real piano's soft notes are much quieter than that (velocity 0.6 is about 7 dB under 0.8, the old voice
  under 2 dB): the composition system can be given that range when it is built.

## How the composition system would ask for a phrase

A sketch, not code that exists yet:

```text
piano, bars 1-2, pedal down for the bar:
  C4 0.5 s  vel 0.55  pedal
  E4 0.5 s  vel 0.50  pedal
  G4 1.0 s  vel 0.65  pedal  accent 0.5      (the top of the phrase leans in)
organ, under it:
  chord C3 E3 G3  2.0 s  vel 0.8  together 3  swell 0.5 -> 1.0 over 1.5 s   (stops coming in)
  chord F3 A3 C4  1.0 s  vel 0.8  together 3  mute Clipped                  (a short, dry stab)
```

Each line is one `Play`; a chord is several `Play`s that share `together`. The composition system decides the
musical choices (where the pedal goes down, which note leans in); the instrument decides how that sounds.
