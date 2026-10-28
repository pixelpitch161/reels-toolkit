# Log: `projects/ballfight_euphonium_vs_trombone`

Append new entries at the end (newest last).

## #48 BALLFIGHT: Euphonium vs Trombone (session_01CzTfqaiKkq, Oct 28), a kit crossover with a ported fighter
- Made with the episode kit: one `episode.py` (seed, handicap, the opt-in swat rule, mood, key, chords, both fighters'
  lines and solos, title hits, a mix override, post text). Valves vs slide: EUPHONIUM (left, amber) as built in #38,
  TROMBONE (right, mint) as built in #34 / #41, both straight from the roster; the only lever is DMG_SIDE. No episode
  number on screen.
- **EUPHONIUM ported** into `kit/fighters/euphonium.py` from #38 with `examples/bassclarinet_vs_euphonium` as the check
  (details in `docs/log/ballfight_studio.md`, the porting log): replay hash identical for 1269/1/2/3/500; 10 frames
  pixel-identical in a matched harness; the euphonium bus bit-identical. Kit hooks added for it (additive): HUD holds for
  `barrage` / `copy`, `Look.ball_sprite` (IDENTITY CRISIS's amber stranger) and `Look.extra_sprites`.
- **IDENTITY CRISIS copies** (new with the port): the copied move is drawn by the enemy's own look module instantiated
  for the euphonium's side, so in amber (the trombone's SAD TROMBONE ring and the slowed enemy's droopy wobble, PEDAL
  TONE's bell), and played in the euphonium's voice: a copied SAD TROMBONE is the plunger wah-wah down to the tonic.
- **Engine, opt-in rule `swat_shells`** (own commit `5abbafd`): whether the lance swats mortar shells was undefined (its
  swat covered volley projectiles and #41's opt-in cobras; shells live in their own list). A live thrust lance knocks a
  shell out of the air when it is coming down through the lance's height band (the renderer's arc) within 0.2 of the
  lance; it never lands, no damage, no growth; event 'swat_shell', recorded with landed = 3 + its swat time. On for this
  episode. In the run the swatted shell spins away as a brass slug with a burst and a brass ping (the euphonium's look
  and sound own it). Replay guard identical.
- **Mood `second_line`** (new, `kit/moods/`): a dark minor New Orleans street groove with no horns of its own: the
  parade bass drum with a cymbal on top (1, the and of 2, 4 | 1, the and of 2, 3, the and of 4), the cymbal's chick on
  every beat (accents 2 + 4), snare stick shots on 2 + 4 and swung-16th ghost notes, press rolls into each phrase, a low
  simple sousaphone tresillo (the Pixel Pitch tuba) with chromatic approaches later, tambourine off-beats and claps in
  the back half; the VS card three band hits over a held sousaphone root and a press-roll swell; the last bar walks the
  sousaphone up into the tonic under a press-roll crescendo and a riser. E harmonic minor, Em Am Em C7 B7 Em Am B7,
  loop_bpm(122) = 121.97 (31 bars, FIGHT! at 3.94 s).
- **Music**: same register, split jobs. The euphonium sings the melody (LINES, a note a beat: its 2-beat salvos play
  beats 1 + 3, MARCHING ORDERS all four); the trombone riffs and smears around it via its new opt-in role `'riff'`
  (own commit `7502248`): every thrust is a stab of LINES['TROMBONE'] (8ths; POWER SLIDE every 8th) smeared up into it,
  the leader doubled an octave down; without the role it stays #41's circus bass. Solos written for the run's bars 27-28
  (Am, Em): a tailgate trombone (the euphonium's hymn is in the config too).
- **Tuning**: the first 100-seed tune said 0.44 / 0.37 = 48 %, but the 5,187 decided runs of the first search said
  EUPHONIUM 39 % (outside 40-60), so re-tuned on 400 seeds: 0.44 / 0.37 = 38 %, **0.44 / 0.35 = 52 %** (median K.O.
  51.2 s), 0.46 / 0.35 = 64 %. Lesson: tune on 300-400 seeds; 100 is too few for a pair this even. Fit rate ~0.15 % (the
  captions 4 s apart and two lead changes rarely coincide): search 1..40000 --stop 7 found 7 fits in 4,606 runs (739 s;
  EUPHONIUM 56 % of 4,470 decided). All fits had IDENTITY CRISIS copying SAD TROMBONE at some point.
- **Seed 4604** (ranked 2nd, picked for the story): its captioned IDENTITY CRISIS (32.5 s) is the euphonium copying SAD
  TROMBONE, the amber ring lands on the trombone at 32.85; three swats (38.4, 41.3, 46.7); PEDAL TONE 43.3 / NOT A TUBA
  44.8; lead changes 42.8 / 44.9; TROMBONE wins at 51.19 s on 7.2 HP. Captions SAD TROMBONE 12.8, MARCHING ORDERS 19.7,
  POWER SLIDE 25.6, IDENTITY CRISIS 32.5, PEDAL TONE 43.3 (NOT A TUBA in the HUD list: 4 s rule).
- **Sound**: band check first failed for the trombone (-0.4..-3.8 dB in #41's bass band 70-700 Hz, against the
  sousaphone and bass drum). The riff role measures 120-1500 Hz (its stabs sit an octave above the bass line) and MIX
  lifts both fighters and lowers the bed (trombone 2.4, euphonium 1.6, sousa 1.45, drums 1.25): EUPHONIUM +5.6..+8.5 dB,
  TROMBONE +5.0..+7.0 dB over the bed; levels PASS (opening +0.5 dB vs the ending, spread 3.8 dB), intro vs fight -0.1 LU.
- Checks: safe zone PASS, focus 0 px, flash 0 general (max 16.9 % at 43.4 s), loop PASS (seam 0.5x, audio 3.8 dB).
  The #47 session pushed engine / kit changes during production: my cached run and 16 sheet tiles came out identical
  on the merged code. Render 1830 frames in 663 s on 2 cores + the loop-clean encode 306 s (one full render, no
  patches). Verify: 1080x1920, h264 + aac, 61.013 s; loopcheck + flashcheck PASS on the mp4; levels PASS on the mp4;
  mp4 sheet matches the stills. Replay guard: every earlier line unchanged; #48 added to KIT_EPISODES.
- Feedback: (pending)
