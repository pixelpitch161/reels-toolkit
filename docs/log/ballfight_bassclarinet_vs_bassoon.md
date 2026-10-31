# Log: #55 BALLFIGHT: Bass Clarinet vs Bassoon (`projects/ballfight_bassclarinet_vs_bassoon/`)

## Oct 31: made (session_01CzTfqb)
- Episode-kit crossover and viewer request: the #38 / #40 BASS CLARINET (left, slate blue) vs the #39 / #52 BASSOON
  (right, forest green), both exactly as in `fighters.py`. No episode number on screen. No approval waits.
- BASS CLARINET wasn't in the kit: ported from #40 v2 into `kit/fighters/bass_clarinet.py` and checked against #40 (see
  `docs/log/ballfight_studio.md`): replay hash identical, picture state identical on every fight frame, its layers
  pixel-identical, its bus bit-identical. Additive kit changes (ball `sink`, HUD holds, ANC / LO), replay guard identical.
  Its octave follows its role: here 'bass' (the growl at the bottom, keeps its fundamental, pumps into the slam).
- Interactions checked before tuning (#41's lesson): the whirl and the twin's arm hit anything in the circle, a hopping
  bassoon included; hook and stomps work on any ball. No opt-in rule needed.
- New mood `kit/moods/horror_trap.py`: half-time trap (pure sub 808 sliding at phrase ends, kick + pickup, clap on 3,
  skittering hats), a detuned music-box lullaby, creeping low strings, reversed swells into the big hits; no woodwinds.
  E minor, loop_bpm(140) = 141.64, 36 bars. LIGHTS OUT drops the bed to music box + 808 (mix held within 0.2 dB);
  MOUNTAIN KING: the trap beat keeps going, its hats on every hop, under the bassoon's Grieg quote (E4).
- Tuning (the only lever, frozen): DMG_SIDE = (0.58, 0.70): 300 seeds 51 / 49 %, median K.O. 49.5 s, both lows 74 %.
- Search 1.. --stop 6: 6 fits in 692 runs. SEED 4 (score 12.06): BASSOON wins at 50.2 s on 8.8 HP, 5 lead changes;
  captions SLAP TONGUE 14.4, MULTIPHONICS 22.0, BOCAL HOOK 33.1, LIGHTS OUT 38.6, MOUNTAIN KING 42.8.
- Colour check (first sheet, user's ask): slate reads as cool grey-blue glass + slate rim vs mint glass + green rim; in
  LIGHTS OUT the sunk ball is dark beside the lit green one. They read apart: no edge glow, no colour change.
- Band check failed first (bass clarinet 3-6 dB under the bed in 60-500 Hz: the 808's harmonics + the kick). Fixed by
  arrangement: the 808 a pure sub at C1..B1, lighter kick / snare body, then MIX lifts the bass clarinet bus (1.95):
  +2.3..+4.5 dB; bassoon +10..+13 dB while it sounds. Strings raised (they were inaudible).
- Checks: safe zone PASS, CoC 0, levels PASS (spread 2.5 dB, no dips), intro -0.8 LU vs the fight, flash PASS,
  loopcheck PASS (render + mp4). Render 7.1 min + loop-clean encode 3.8 min. mp4: 1080x1920, 61.0 s, h264 + aac.
- Feedback: (pending)
