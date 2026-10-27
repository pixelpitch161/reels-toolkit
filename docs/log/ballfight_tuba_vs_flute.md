# Log: `projects/ballfight_tuba_vs_flute`

Append new entries at the end (newest last).

## BALLFIGHT #6: Tuba vs Flute (`projects/ballfight_tuba_vs_flute`)
- Sixth BALLFIGHT episode (#32), produced in parallel with #29-#35 on the shared engine. TUBA (indigo, left) blasts a
  slow, huge brass wave every bar (#28's `pulse`, every=4) and pulls the flute in all the time (new weapon option
  `pull`: gravity that only steers, speed stays shared; +0.08 per landed wave); FLUTE (silver, right) fires tiny fast
  darts every beat (volley with the new option `on16`, so FLUTTER TONGUE's haste can double it). Abilities: OOMPAH /
  BRASS WALL / SUBSONIC (a giant tuba bell hovers, then slams); FLUTTER TONGUE / TRILL / PICCOLO. Villain march: C
  minor, 125.90 BPM (32 bars), i-VI-N6-V with the B natural over Db as the villain's augmented-sixth snarl.
- Honest run, seed 1097 (6 fits in 1099; TUBA 49 % / FLUTE 51 % over 1057 decided): neck and neck, the flute's chip
  damage leads until 44.9 s, flute back ahead at 48.6, both desperation moves on the same beat (48.1), the bell lands
  at 49.1, TUBA wins at 51.54 s with 4.5 HP. Captions: FLUTTER TONGUE 14.3, OOMPAH 19.5, TRILL 24.8, BRASS WALL 29.5,
  SUBSONIC 48.1 (PICCOLO in the status line, same beat).
- Engine (shared, additive, two small commits): weapon option `pull` (+ `pull_grow` / `pull_max`, grows with a landed
  main pulse) and volley option `on16`. #27 (seed 88) and #28 (seed 568) replay identically
  (`tools/ballfight_replay_hash.py`), re-checked after every rebase onto the other sessions' engine commits (beam,
  orbit, rewind, aura, bellows). Roster commit conflicted with #30's (both appended at the dict's end): keep both.
- Rules lessons: without the pull the tuba landed ~5-8 waves a fight and never won (a once-a-bar weapon needs reach
  or help getting there); reach 1.8 + pull 1.2 balanced it in one grid (`tune.py`, 20 seeds per cell).
- Look lessons: the flute's Pixel Pitch model is a 1 px line at ball size; a pixel-art flute (silver tube, lip plate,
  keys, ink outline) in `styles.flute_sprite` reads, and a light (silver) fighter needs a cooler, darker glass tint.
- Sound: the tuba fighter IS the march's bass (main waves = root "oom", OOMPAH's beat-3 waves = the fifth, landed waves
  = a walk-up), the flute's volleys are runs up the chord (n darts = n notes); bed kick turned down and the pad's low
  note dropped so the tuba sits +7-9 dB over the bed in 40-260 Hz; flute +7-13 dB in 0.9-6 kHz.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 2.2x);
  flash 0 general (max 6.6 % at the bell slam); opening -18.9 dBFS, build +3.1 dB. Render 0.19 s/frame/core (6 min) + mux.
- Feedback: (pending)
- v2 (BALLFIGHT v2 redesign, user Oct 24: attacks the instrument's body / playing, more visual character, no
  full-screen flashes). TUBA: OOM-PAH SWALLOW (new kind `swallow`: oom = the bell inhales for a beat and steers the
  enemy in, touching it = swallowed and held inside, pah = spat out like a cannonball, a rope hit within 0.9 s = extra
  damage; the reach grows per landed swallow); BRASS WALL = a brass dome, SUBSONIC = the floor ripples like water.
  FLUTE: SNAKE CHARMER (new kind `striker`: a basket every bar, up to 2, a cobra per basket striking on the 16th grid;
  new ability kind `dance`: SNAKE DANCE sends the cobras slithering after the enemy for a bar); PICCOLO adds glass-crack
  marks. Engine commits additive and alone; the swallowed ball is skipped in `move()` via a `held` attribute.
- Parallel production: the shared fight.py took 10+ other sessions' commits during this work; every rebase conflicted
  in the same elif chains (setup / beat / step / trigger). Resolving by hand (and moving #32's setup into its own loop)
  and re-checking all 45 guarded runs against upstream origin/main before each push kept #27, #28 and #29-#35
  identical. A helper (rebase, keep-both on additive conflicts, verify vs a worktree of origin/main, push, retry on a
  push race) saved several rounds.
- Rules lessons: the cobra bite is the touchiest number (1.05 -> 1.25 moved the tuba 53 % -> 17 %); 32-48 seeds per cell
  misled once (21 % in the search), 90-120 seeds got it right. The first v2 opening was slow (first hit 5-6 s, 5 s
  stretches without a hit): a basket every bar, a longer starting reach and a hard "no 4 s without a hit" window fixed
  the pacing. Final: TUBA 53 % / FLUTE 47 % over 515 decided, damage scale 1.06.
- Honest run, seed 514: tuba leads to 33 s (four swallows by 26 s), flute takes over, tuba back ahead at 47.6 s,
  FLUTE wins at 49.37 s on a double cobra strike with 5.7 HP.
- Look lessons: thin cobras vanish on the lavender canvas; an ink outline, a 0.16-unit body and a hood turned partly
  toward the camera made them read at ball scale. Additive canvas light doesn't show on a bright canvas: the suction
  cone reads as darker wedge + bright dashes racing in. `reelkit.fx` wired per projects/ballfight/README.md (flash
  probe on the busiest stretch peaked at 15.8 % of the frame).
- Loop lesson: less motion than v1 lowered loopcheck's "normal step", so the same beat-pulse snap at frame 0 failed
  (2.6x); a longer pre-ramp on `beat_pulse` (120 ms) brought the seam to 1.5x.
- Sound: oom = the tuba's root held through the inhale (+ air sucked in), pah = the fifth as a blast (+ brass blast and
  whoosh on a spit), gulp = a low bubble pop; the flute plays an original charmer line in C harmonic minor (augmented
  2nd, grace notes, phrygian-dominant turn over G) while its cobras are up, flutter-tongued in FLUTTER TONGUE, an
  octave up in SNAKE DANCE, with a hiss-click per strike; the music box went to quarter notes so the charmer line leads.
- v2 checks: safe zone OK; subject CoC 0 on all 1830 frames; loopcheck PASS render (1.5x) + mp4 (1.9x); flash 0
  general (max 15.6 %); opening -17.9 dBFS, build +3.7 dB. Full render 313 s on 2 cores + mux. Feedback: (pending)
- v3 (VS title card intro, series rule Oct 24): bar 0 = the shared title card (projects/ballfight/title.py, already moved
  there by #35; this episode added opt-in CLOSEUP / GLASS registries so the flute's close-up is the pixel-art flute in cool
  glass; #30's card checked pixel-identical on 13 frames). Panels: indigo bell-mouth rings / steel-silver key cups (a
  near-white silver panel read as white: scaled to ~0.7). 3-2-1 on bar 1, FIGHT! at 3.81 s, top caption + series tag
  code dropped, HUD hidden while the card is shut; chords Cm (card) . G (count-in) . the march loop . Ab . G into the
  slam; the charmer line and music box re-anchored to bar 2.
- Seed search re-run at 2 bars (rules unchanged): 6 fits in 753, TUBA 53 % / FLUTE 47 %. Seed 514 ranked first again;
  unlike #30's 1442, its run at 2 bars is bit-identical to v2's shifted by a bar (no float drift for this roster), so
  the fight is v2's (FLUTE 51.28 s, 5.7 HP).
- Loop: seam 0.4x render / 1.1x mp4 (the title slam as the reset beats v2's beat-pulse snap), audio step 2.0 dB; flash
  check PASS with the single doors pair (1.9 s), max 51.9 % of the frame changing as the doors shut (one transition).
  Full render 5.5 min + mux. Feedback: (pending)
