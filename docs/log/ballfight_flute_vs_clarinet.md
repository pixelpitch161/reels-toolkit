# Log: `projects/ballfight_flute_vs_clarinet`

Append new entries at the end (newest last).

## #49 BALLFIGHT: Flute vs Clarinet (session_01CzTfqd, Oct 29), a kit crossover with a new mood
- Made with the episode kit: one `episode.py`. FLUTE (left, silver) as built in #32 v2 / drawn in #41, CLARINET (right,
  lavender) as built in #37, both straight from the roster; the only lever is DMG_SIDE. No episode number on screen.
  RULES empty (no lance, so no swat_cobras).
- **Mood `bossa_noir`** (new, `kit/moods/`): a minor bossa with no woodwinds: nylon guitar (Karplus-Strong, soft
  finger attack) on the 2-bar 3-2 clave figure with the thumb on 1 and 3, a soft upright (sine body + KS string + thump,
  dotted-quarter + eighth: root 1, fifth 2&, fifth 3, pickup 4&), brushes on straight 8ths, felt kick on 1 / 2& / 3 / 4&,
  cross-stick rims on the clave, a dark FM Rhodes pad with tremolo; back half adds shaker, ghost strokes, the Rhodes an
  octave up, bass pickups. Card = three band hits + a nylon tremolo; last bar = chromatic bass walk-up under a 16th rim
  roll + riser. D minor, Dm7 Dm6 Gm7 C7 Fmaj7 Bbmaj7 Em7b5 A7, loop_bpm(132) = 133.77 (34 bars).
- Music: the flute's cobras sing the high melody (8ths, C#5-E6), the clarinet's jumps the counter-line below (one note a
  beat, D4-E5, a chromatic fall over A7). Both solos original.
- Tuning: this pair is lopsided in hit counts: the jumps dodge most strikes (~11 bites vs ~68 cuts a run), so the flute's
  bites carry a big multiplier. Frozen DMG_SIDE (5.3, 0.36): 300 seeds 50 % / 50 %, median K.O. 49.4 s. Search 1..40000
  --stop 8: 8 fits in 6,454 runs (679 s, FLUTE 53 % of 6,348 decided); every fit was a flute comeback.
- **Seed 3864**: the clarinet leads from 16.6 s, the flute retakes it at 48.9 s and wins at 49.16 s with 8.1 HP.
  Captions SQUEAK 14.4, CLARION 28.7, SNAKE DANCE 34.1, PICCOLO 42.2, RHAPSODY 48.4 (FLUTTER TONGUE 31.4 in the HUD
  list only: 4 s rule).
- Colours: silver vs lavender read apart on the early sheet (steel-blue glass + silver flute vs milky lilac + black
  clarinet), no edge glow needed.
- Sound lesson: the first mix failed `band` (flute +3-4 dB, clarinet -3..+3 dB over the bed): the upright's octave string
  and harmonics and the Rhodes' G3+ voicing sat in the clarinet's 140-700 Hz, the nylon's top in the flute's band. Fix:
  warmer upright (fundamental string, double LP 520 Hz), darker nylon (LP 2200/2600), bus gains guitar 1.85 -> 1.35,
  keys 1.35 -> 0.95, drums 1.55 -> 1.70; MIX clarinet (2.4, 0.35), flute (2.1, 0.45). Then flute +8.2..+9.8 dB,
  clarinet +4.4..+9.9 dB; levels PASS (opening +0.4 dB vs the ending, spread 2.0 dB); intro -1.5 LU vs the fight.
- Mid-session the #51 session pushed additive engine kinds: seed 3864 hashed identically on both engines before render.
- Checks: safe zone PASS, focus 0 px, levels PASS, band PASS, flash 0 general (max 17.7 % at 42.4 s), loop PASS.
- Render: 1830 frames in 496 s + loop-clean encode 229 s (one full render, no patches). Verify: 1080x1920 h264 + aac,
  61.013 s; loopcheck + flashcheck PASS; mp4 sheet matches the stills. Replay guard: every earlier line unchanged; #49
  added to KIT_EPISODES.
- Feedback: (pending)
