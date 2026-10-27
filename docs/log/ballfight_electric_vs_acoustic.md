# Log: `projects/ballfight_electric_vs_acoustic`

Append new entries at the end (newest last).

## #44 BALLFIGHT: Electric vs Acoustic (session_01CzTfqcUYqS, Oct 27), the first episode-kit episode with a new fighter
- Made with the episode kit: one `episode.py` (seed, handicap, mood, key, chords, the ELECTRIC's riff line, both solos,
  title hits, post text). The guitar family feud, plugged vs unplugged; no episode number on screen.
- **ACOUSTIC ported** into `kit/fighters/acoustic.py` from #33 v3 (styles + render layers + its audio part, unchanged)
  with `examples/ukulele_vs_acoustic/episode.py` as the check: seeds 1130/1/2/3/500 hash the same as #33's own sim; the
  acoustic's layers match #33's renderer frame for frame (0.86 sprite, capo, rake strings, claw marks, BIG STRUM,
  shards; the rest of each frame differs only where the unported UKULELE and the kit's HUD list are); its audio bus
  matches #33's to 4e-16 (summation order). Two port details: the sprite size became a Look option (`sprite_k`, default
  0.80, so #41 / #37 don't move), and #33's own up-stroke hash is kept (the kit's newer splitmix would change which
  up-strokes drop out). Its voicings are standard open shapes, so any episode's chords are voiced the same way.
- **New ELECTRIC** (neon green): engine kinds `lightning` (LIGHTNING RIFF + `fork` DISTORTION + `tapping` TAPPING) and
  `smash` (GUITAR SMASH), additive, own commits, replay guard identical (105 lines) after each, including after merging
  #45's PIANO ROLL engine work. A riff every bar; each note an instant bolt from the headstock (reach 1.1) or from the
  corner post nearest the enemy (1.7: the lightning rods); +1 note per 2 landed riffs (3-3-2 first). TAPPING became a
  second hand (a tap bolt on every 8th on top of the riff): as first written a full riff took every 8th and taps never
  fired. GUITAR SMASH: shockwave + 10 splinters, a beat in pieces (still, intangible, starts nothing), then it fights on.
- Numbers: the first grid (riff from 2 notes, reach 1.5) was a flat damage race (ELECTRIC 60+ bolts to ~20 strokes, ~0
  lead changes): its knockback kept the acoustic out of rake range. Knock 0.35 -> 0.15, a slower riff ramp and the
  short headstock / long post reach gave even hit counts and real lead changes. Handicap (crossover rule) ELECTRIC x0.92,
  ACOUSTIC x1.33: 300 seeds ELECTRIC 51 %, median K.O. 51.0 s, both desperation moves in 90 % of fights. Fits are rare
  (~0.5 %; captions average 4 because six unlocks crowd 15-45 s): search 1..15000 --stop 6 found 6 in 1,251 runs (88 s).
- **Seed 418**: the ACOUSTIC leads early, the ELECTRIC takes over at 20 s, BIG STRUM at 38.6, GUITAR SMASH at 42.9
  (the finish the idea hoped for), level at 15-15 by 45 s, ACOUSTIC wins at 51.53 s with 7 HP. Captions CAPO, TAPPING,
  FINGERPICK, BIG STRUM, GUITAR SMASH (DISTORTION 0.5 s after CAPO goes to the HUD list only).
- **Mood `stadium_rock`** (new, `kit/moods/`): one original minor arena-rock song (E minor, Em C G D Em C Am B7,
  loop_bpm(126) = 125.90, 32 bars) in two arrangements. Plugged: fuzz rhythm guitar kept dark (LP 2.1 kHz, under the
  electric's riff), pick bass, stadium kit; unplugged: a quiet high strum shimmer, upright-ish bass, cajon / shaker /
  tambourine; the stadium kick is in both. Config `PLUGGED = 'ELECTRIC'`. Both arrangements are matched to the same
  K-weighted loudness over the fight (here the unplugged one x0.72), then an equal-power crossfade over one beat from the
  next 8th at every lead change (series hysteresis 5 HP); both at once on the card, before anyone leads and into the
  seam. Result: unplugged 8.1 s, plugged 20.3 s, unplugged 42.9 s and for the victory; the bed measured -18.7 / -17.8 /
  -18.8 / -18.9 LUFS across those stretches and the crossfades.
- Sound lesson: with the fighters carrying the fight, the first bed was 10-20 dB under them, so the card, count-in and
  the run-up into the seam (no fighters) read 2.2 LU quieter than the fight and the last bar was the quietest window.
  Bed +4 dB, the count-in keeping the groove (not three hits) and the card at full density: opening +0.2 dB vs the
  ending, spread 3.2 dB, intro -0.9 LU vs the fight, fighters still +6..+18 dB over the bed in their bands.
- Kit fixes on the way (own commits): `preview` could pick a run without a K.O. (crashed the renderer); the HUD list
  now holds DISTORTION / TAPPING / GUITAR SMASH lit while they last.
- Preview (75 s to make) sent before tuning; the first bolts were 1-px threads that vanished at phone size: a 2-px
  white-hot core, a wider glow, a soft halo and a glow where each bolt leaves fixed it.
- Checks before the render: safe zone PASS (0.25 s), focus CoC 0 on 1,830 frames, levels PASS, band PASS, flash 0
  general (busiest 37.6-53 s, max 14.9 % of the frame), loop PASS (seam 0.5x, audio 2.4 dB, no click).
- Render 353 s + loop-clean encode 189 s (2 cores). Verify: 1080x1920 h264 + aac, 61.013 s (2860 AAC frames),
  reelkit.loopcheck PASS and reelkit.flashcheck PASS on the mp4; the mp4 sheet matches the stills. Claim to delivery about
  1 h (13:05-14:06, the port, two engine kinds, a new fighter's look and sound and a new mood included).
- Replay guard: `tools/ballfight_replay_hash.py` now runs this episode too (KIT_EPISODES).
- Feedback: (pending)
