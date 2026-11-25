# Sound upgrade, stage 1: measurements

Measured on 2026-11-25 with `cargo run --release -p bf-harness --example measure -- <fight.toml>...` on the ten
review fights of the sound review (the same fight files and seeds as the review page) and on the 17 series fights
under `studio/fights/`. Every figure is of the Composed arrangement. "Today" is the Classic master (one
compressor and the squash) on the same score; "new" is the polish chain of `bf-sound/src/polish`.

Design: `2026-11-25-sound-upgrade-design.md`. Plan: `2026-11-25-sound-upgrade-plan-stage1.md`.

## Done-when lines

| Pick | Done when | Measured | |
|---|---|---|---|
| M1 | every fighter 4 to 6 dB over the band by the heard measure | 4.97 to 5.28 dB (today -6.4 to +14.6) | pass |
| M2 | a low instrument loses at most 9 dB on a phone, rumble share at most 70% | 6.8 to 7.7 dB (today 10.3 to 16.6), 55 to 68% | pass |
| M3 | a piercing instrument's 2 to 6 kHz at most 3 dB over the other fighter | -4.4 to -16.9 dB (today -4.0 to +0.8) | pass |
| F1 | -14 LUFS (within 0.5), true peak at or under -1 dB | -14.0 to -14.1 LUFS, -1.02 to -1.85 dB | pass |
| F2 | a tuba note pulls the piccolo down at most 1 dB; nothing clipped | 0.90 dB (today 3.64); highest sample 0.86 | pass |
| F3 | the room's rumble share under 10% (today 82%) | 0 to 8% | pass |
| C6 | Violin vs Contrabass: the contrabass holds at least 70% of the lows | 77% (today 30% on Classic) | pass |
| C2, C10 | no clash, no note out of register, no repeat in a moving style | 0, 0, 0 (survey of the melody test) | pass |

Three settings moved from the plan while measuring (also noted in the plan):

- **M1 trim, ±9 dB instead of ±6.** In Violin vs Contrabass both fighters stood 13.2 and 13.4 dB over that fight's
  quiet band; at ±6 they stayed 7.2 and 7.4 dB over.
- **F2 middle range, ratio 1.6 instead of 2.0.** At 2.0 a tuba note (its new overtones sit in the middle range)
  still pulled the piccolo down 1.21 dB.
- **C6, the band makes room.** Once the contrabass was trimmed, the band's kick and the hits' thumps held 42% of the
  lows. While a bass fighter holds the bottom, every bus of the band and the hits loses 6 dB under 150 Hz from
  FIGHT! to the K.O. (`lows::room_for_bass`).

## Each fighter

| Fight | Fighter | Over the band, today → new (dB) | Phone loss, new (dB) | Pull by the other, today → new (dB) |
|---|---|---|---|---|
| Bagpipes vs Sousaphone | Bagpipes | +3.9 → +5.0 | 0.1 | 1.15 → 0.68 |
| Bagpipes vs Sousaphone | Sousaphone | +1.6 → +5.0 | 7.2 | 0.24 → 0.46 |
| Cornet vs Mellophone | Cornet | +1.4 → +5.0 | 1.8 | 0.08 → 0.90 |
| Cornet vs Mellophone | Mellophone | -2.1 → +5.0 | 1.1 | -0.11 → 0.83 |
| Electric vs Bass Guitar | Electric | +8.6 → +5.0 | 0.1 | 0.90 → 0.50 |
| Electric vs Bass Guitar | Bass Guitar | +5.8 → +5.0 | 0.5 | 1.55 → 0.80 |
| Flute vs Octobass | Flute | +6.2 → +5.0 | 0.1 | 0.19 → 0.70 |
| Flute vs Octobass | Octobass | -6.4 → +5.0 | 6.8 | 0.43 → 0.71 |
| Snare Drum vs Timpani | Snare Drum | +3.5 → +5.0 | 1.0 | 1.36 → 0.97 |
| Snare Drum vs Timpani | Timpani | +4.1 → +5.0 | 7.7 | 0.29 → 0.29 |
| Soprano Sax vs Bari Sax | Soprano Sax | -3.8 → +5.0 | 0.2 | 0.00 → 0.00 |
| Soprano Sax vs Bari Sax | Bari Sax | +6.3 → +5.3 | 1.5 | 0.14 → 0.77 |
| Trumpet vs Cello | Trumpet | +7.8 → +5.0 | 0.0 | 0.84 → 0.61 |
| Trumpet vs Cello | Cello | +7.6 → +5.0 | 1.6 | 2.30 → 1.08 |
| Trumpet vs Clarinet | Trumpet | +6.6 → +5.0 | 0.1 | 1.27 → 1.08 |
| Trumpet vs Clarinet | Clarinet | +3.0 → +5.0 | 2.0 | 2.03 → 0.79 |
| Piccolo vs Tuba | Piccolo | +8.7 → +5.0 | 0.0 | 3.64 → 0.90 |
| Piccolo vs Tuba | Tuba | +2.6 → +5.0 | 7.2 | 0.70 → 0.34 |
| Violin vs Contrabass | Violin | +14.6 → +5.0 | 0.4 | 1.83 → 0.56 |
| Violin vs Contrabass | Contrabass | +7.6 → +5.0 | 7.5 | 1.94 → 0.54 |

"Pull" is how much lower a fighter sits, after the compressor, while the other fighter is loud (its top quarter)
than while it is quiet (under its median). Today it is close to 0 where the old compressor barely worked (the
quieter fights); the new one works the same in every fight, so a few small pulls grew while the big ones fell.

Low and piercing fighters (the ones M2 and M3 treat):

| Fighter | Phone loss (dB) | Energy under 200 Hz | Piercing share | 2 to 6 kHz against the other (dB) |
|---|---|---|---|---|
| Tuba | 16.6 → 7.2 | 93% → 61% | | |
| Contrabass | 13.6 → 7.5 | 90% → 68% | | |
| Sousaphone | 12.8 → 7.2 | 93% → 66% | | |
| Octobass | 11.6 → 6.8 | 86% → 55% | | |
| Timpani | 10.3 → 7.7 | 79% → 66% | | |
| Bagpipes | | | 42% → 16% | +0.6 → -5.1 |
| Snare Drum | | | 42% → 21% | -3.8 → -4.4 |
| Violin | | | 12% → 2% | +0.8 → -11.8 |
| Piccolo | | | 10% → 2% | -4.0 → -16.9 |

## The whole soundtrack

| Fight | Loudness, today → new (LUFS) | True peak, new (dB) | Room rumble | Lows held by the bass fighter |
|---|---|---|---|---|
| Bagpipes vs Sousaphone | -11.3 → -14.0 | -1.29 | 0.04 | 88% |
| Cornet vs Mellophone | -14.9 → -14.1 | -1.13 | 0.02 | |
| Electric vs Bass Guitar | -14.4 → -14.1 | -1.02 | 0.02 | |
| Flute vs Octobass | -14.1 → -14.1 | -1.06 | 0.00 | 92% |
| Snare Drum vs Timpani | -11.9 → -14.1 | -1.02 | 0.08 | 88% |
| Soprano Sax vs Bari Sax | -15.4 → -14.1 | -1.12 | 0.03 | |
| Trumpet vs Cello | -9.5 → -14.0 | -1.26 | 0.01 | |
| Trumpet vs Clarinet | -9.4 → -14.0 | -1.85 | 0.01 | |
| Piccolo vs Tuba | -10.5 → -14.1 | -1.18 | 0.02 | 94% |
| Violin vs Contrabass | -10.0 → -14.0 | -1.20 | 0.02 | 77% |

The levels check (the loudness even from the opening to the ending) on the 17 series fights under Composed: 11
pass on the master as it comes out of the polish chain; 6 miss it narrowly, since the new compressor is gentler
than the squash: 5.3 to 5.8 dB between the loudest and the quietest 2 s where 5 are allowed (Bass Clarinet vs
Bassoon, Cello vs Contrabass, Flute vs Sousaphone, Trumpet vs Clarinet), an opening 1.9 dB under the ending where
1.5 are allowed (Bass Clarinet vs Euphonium), two dips where one is allowed (Electric vs Acoustic). The render's levels fix
evens those out as it always has, and the loudness target runs again after it. Trombone vs Horn under Composed has
no K.O. at its file's seed (the horn's notes are part of its fight), so it was left out.

## The melodies (C2, C10, C11)

From `bf-harness/tests/melodies.rs` and its survey (`examples/melodies.rs`), over every theme and both corners:
no note clashes with the other fighter on a strong 8th, no note outside a fighter's register, no note written
twice in a row in a moving style (Climbing, Runs, Broken chords). Signature melodies change from bar to bar 96 to
99.7% as often as today's and repeat whole bars less. The six styles differ from each other in at least half of
their notes, the bass lines in at least a quarter.
