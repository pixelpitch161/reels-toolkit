# Log: `projects/ballfight_trumpet_vs_clarinet`

Append new entries at the end (newest last).

## #46 BALLFIGHT: Trumpet vs Clarinet (session_01CzTfqa, Oct 28), a kit crossover with a ported fighter
- Made with the episode kit: one `episode.py` (seed, handicap, mood, key, chords, both fighters' lines and solos, title
  hits, a mix override, post text). Who leads the jazz band? TRUMPET (left, yellow) as built in #29 v3, CLARINET (right,
  lavender) as built in #37, both straight from the roster; the only lever is DMG_SIDE. No episode number on screen.
- **TRUMPET ported** into `kit/fighters/trumpet.py` from #29 v3 (styles + render layers + its audio part, unchanged)
  with `examples/trumpet_vs_sax/episode.py` as the check (details in `docs/log/ballfight_studio.md`, the porting log):
  replay hash identical for 191/1/2/3/500; 12 frames pixel-identical in a matched harness; trumpet bus to 1e-16. Port
  details: #29 fought from bar 1, so the kit got an optional `FIGHT_BAR` (default 2, own commit, replay guard
  unchanged); the trumpet turns inside its ball to its aim, so its look draws its own sprite (no yaw sway, as #29) from
  its own atlas `check/sprites_trumpet_aim.npz` (radii 12-19 x 36 rolls, built once in 2 processes, ~25 s) at the roll
  its `ball()` hook sets from the camera; the fingering rule (partial 4 on the chord's root, else its fifth) gives #29's
  table exactly, and the key-bound calls (FANFARE, DOUBLE TONGUE, stinger, fall apart, reassembly, the last bar's climb)
  follow the config's KEY / chords (C minor = #29's notes). New and opt-in: a config line `LINES['TRUMPET']` (8ths)
  makes the punches play a melody (without one the part is #29's).
- Harness lesson: post-K.O. frames first differed everywhere by a few levels: the kit lights its attack flares (scene
  and crowd) on the effects clock held through the hit-stop, #29 on real time. A kit series rule, not the fighter:
  matched in the harness, the shatter frames came out identical.
- **Mood `hot_jazz`** (new, `kit/moods/`): a fast minor two-beat with no horns or clarinet of its own: the Pixel Pitch
  tuba on 1 and 3 (root, fifth; pickups later), a Karplus-Strong rhythm guitar chopping 2 and 4 (dark voicings, the
  dominant's flat ninth), brushed cymbals with a light swung skip (58 %), a felt bass drum on 1 and 3, brush taps that
  turn into snare press rolls on 2 and 4 in the back half, a press roll into every phrase, woodblock off-beats; the VS
  card is three band hits + a tremolo chord; the last bar walks the tuba up chromatically into the tonic under a press
  roll and a riser. D minor, Dm Dm Gm Gm Bb7 A7 Dm A7, loop_bpm(140) = 141.64 (36 bars).
- Music: the trumpet's punches play the melody (combos fall on 8ths 1-2-3 of each half bar, the hardest third punch on
  2 and 4: a hot riff); the clarinet's jumps play the counter-line, one note a beat, long tones above the arpeggios,
  a chromatic fall over A7 (CLARION's 8th-jumps add the passing notes). Both solos original.
- Tuning: balance flips fast (TRUMPET 0.36 / CLARINET 0.37: 92 % / 8 %); lead changes are rare for this pair (~0.5 a
  run: ~180 piston chip hits vs ~68 heavier cuts). Frozen DMG_SIDE (0.34, 0.41): 300 seeds TRUMPET 43 % / CLARINET
  57 %, median K.O. 49.6 s. Fit rate ~0.09 %: search 1..30000 --stop 8 found 8 fits in 9,267 runs (478 s, TRUMPET 46 %).
- **Seed 8410**: the clarinet leads early, lead changes at 36.4 / 39.6 / 43.2 s, RHAPSODY at 41.1 answered by HIGH C at
  41.9 (fires 42.36; the clarinet's jump takes it off the line), CLARINET wins at 49.99 s with 3.4 HP. Captions FANFARE
  12.3, SQUEAK 20.8, DOUBLE TONGUE 25.8, CLARION 30.5, RHAPSODY 41.1 (HIGH C in the HUD list only: 4 s rule).
- Sound lesson (as #44): with the fighters carrying the fight the bed sat 13-16 dB under the trumpet, so the count-in,
  victory and run-up dipped (intro -2.5 LU). The count-in at the full groove + a band hit on each count, the victory at
  full tilt and the bed buses ~+3 dB: intro -1.4 LU, levels PASS (opening +0.3 dB vs the ending, spread 4.0 dB). The
  clarinet's chalumeau shares 140-700 Hz with the tuba and the chop: MIX clarinet x1.9 (+2 dB).
- Checks (`python -m ballfight check`, 164 s): safe zone PASS, focus 0 px, levels PASS, band TRUMPET +10.0..+14.1 dB /
  CLARINET +4.3..+10.0 dB over the bed, flash 0 general (max 16.4 % at 41.3 s), loop PASS.
