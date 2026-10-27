# #42 BALLFIGHT #16: Viola vs Violin (`projects/ballfight_viola_vs_violin`)

The orchestra's oldest joke: the violin gets every melody, the viola gets the rests and the punchlines. The viola's
revenge. VIOLA (teal, left) is a new fighter; VIOLIN (violet, right) comes exactly as built in #28 (crossover rule: the
only per-episode lever is its damage handicap).

- **VIOLA** COUNTING RESTS (engine weapon kind `rests`, the only charger): it rests for 3 beats (a giant quarter rest
  over the ball, the count "1 2 3" above it, half damage taken), then on beat 4 a giant whole note appears and rolls at
  the violin like a heavy wheel for a whole bar, crushing it once and flattening its pizzicato darts; every landed note
  makes the next bigger and heavier. LV2 DOUBLE STOP (`double_stop`: two notes at once, at the violin and where it is
  heading), LV3 MELODY AT LAST (`melody`: for 2 bars the rests shrink to one beat, a note every other beat, and the music
  hands the viola the melody), LOW VIOLA JOKE (`joke`: a rimshot and the whole crowd bursts out laughing; the violin
  freezes for a beat in #37's stun state while the biggest whole note drops on it). No joke text on screen.
- **VIOLIN** (#28): the bow spun like a rapier, faster per landed hit; PIZZICATO / TREMOLO / SCREECH; plus the allowed v2
  look upgrade (rosin dust off the bow, attack lights).

## Commands (from this folder)

```bash
python tune.py 150 hv=0.7,0.75 hn=1.9,2.1             # tuning (done once; VIOLA numbers frozen in fighters.py)
python seed_search.py 1 6000 --stop 8                 # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 8219                                    # the run's story (cached in check/run_8219.pkl)
python render.py                                      # sprite cache (check/sprites_ballfight16.npz, ~20 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.2 2.2 4.4 5.2 15.1 24.6 31.0 41.7 42.0 50.42 54.0
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                 # subject CoC must be 0 px on every frame
python flash_test.py 39.5 51.5                        # the busiest stretch
python audio.py && python -m reelkit.levels audio.wav && python band_check.py && python loudness.py
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## The new fighter

- **Model** (`series/instrument-history/viola_lowpoly.py`, own commit): built from reference photos found with Claude in
  Chrome on Wikimedia Commons: "Bratsche.jpg" (a 16.5" viola, front + side views, public domain), measured per pixel
  row in a canvas: the outline (bouts 46 / 31 / 59 % of the body length, corners, f-hole eyes, bridge at 45 %,
  tailpiece, fingerboard end at 63 %), the rib depth from the side view (11.5 % + ~2.5 % arching each side);
  "Violin-Viola.jpg" (CC BY-SA 3.0) for the size ratio side by side; "Viola Aldo Gallipoli 1990.jpg" (CC BY-SA 4.0)
  to check the outline and fittings. In the violin model's units: body 1.118 vs .95, ribs .128 + arched top and back
  (the violin is a flat .1), darker varnish. The model has no bow (the violin's model includes one, which shrinks it
  inside its ball), and the bridge entry sizes it x1.12: in the ring the viola reads clearly bigger and, seen three
  quarters on (view yaw 34), deeper. Its close-up / ring sprites use a wider crop (`render.render_padded`).
- **Voice** (own commit): a darker bowed voice a fifth below the violin (C3 G3 D4 A4): lower air / body resonances
  (~300 / 620 Hz), a bright band near 1.65 kHz instead of the violin's ~3 kHz, a warmer low-pass, slower attack, more
  bow noise, wider slower vibrato. Spectral centroid ~1.1 kHz vs the violin's ~2.2 kHz a fifth higher.
- **Engine** (`projects/ballfight/fight.py`, one commit per kind, all opt-in; the replay guard identical on all 80
  runs after each): `rests` (+ `guard` in `damage()`, records REST / NOTES), `double_stop`, `melody`, `joke`.
- **Roster** (`fighters.py` VIOLA, own commit): numbers tuned once against the #28 violin, frozen.

## Tuning and the run

The rests halve the bow's damage most of the time, so the violin's handicap is large: `DMG_SIDE` VIOLA x0.75, VIOLIN
x2.1 (150 seeds: 49 % / 51 %, median K.O. 49.5 s). Whole notes land ~84 % (a slow wheel the size of the ball, rolling
for a whole bar, is hard to dodge for a constant-speed ball; an opt-in "topple at the ropes" rule was tried and barely
changed it, so it was dropped). Lead changes ~0.6 a run, fits ~0.3 %: the honest search took 2,164 runs for 8 fits
(VIOLA 44 % / VIOLIN 56 % of 2,098 decided), a second range 3,765 runs for 10 more (45 % / 55 %).

**Seed 8219** (top score of both ranges, 8.91): the viola leads early (its notes vs the slow bow), MELODY AT LAST at
24.1 s, TREMOLO (30.5 s) claws the violin back and it takes the lead at 41.3 s; VIOLA JOKE at 41.5 s freezes it and the
giant note lands at 41.9 s (SCREECH answers the same instant); the viola retakes the lead at 44.0 s, and its last whole
note K.O.s the violin at **50.41 s** in the same instant a pizzicato dart drops the viola to **0.9 HP**. Captions
DOUBLE STOP 14.8, PIZZICATO 19.5, MELODY AT LAST 24.1, TREMOLO 30.5, VIOLA JOKE 41.5 (SCREECH 41.9 lights in the HUD
list only: 4 s rule).

## Picture

`render.py` = #41's pipeline (ring, fx kit, title card, HUD ability list, crowd, shards) with #28's violin (bow, motion
fan, TREMOLO's violet blur, darts, SCREECH's jagged ring; `styles.py` imports #28's draw functions) and the viola:
- the whole note (`styles.draw_note`): the engraved notehead's hollow oval (outer 1 x .68, a narrow hole leaning "/"),
  ink with a teal-lit rim and a lit top edge, standing on the canvas and facing the viewer (a true upright wheel is a
  sliver from this 58-degree camera whenever it rolls toward or away from it); it rolls with the travel (turning the
  oval bobs it, the hole spins), drawn at 1.3 x its hit radius so a crush lands as the glass touches its edge; casts a
  shadow; pops in with a teal flash; fades over its last 0.14 s (a whole note's length);
- the rests: a giant pixel quarter rest over the resting ball (settling in on each count), the count "1", "1 2", "1 2 3"
  above it (the newest bright; beside the glyph when a caption is up), a pulsing teal pool on the canvas;
- MELODY AT LAST: eighth notes rising off the ball on each note, a teal halo; the champion's solo does the same;
- VIOLA JOKE: the crowd rocks with laughter (heads bobbing "ha-ha-ha", arms up), the violin's glass goes grey and still,
  the giant note falls from 4.6 up (a target ring under it), lands with a crash, cracks and a scorch;
- the shatter: scroll, strings, bridge, wood in each one's varnish (the violin also its bow).

## Music (mood A, orchestral drill)

C minor at 141.64 BPM (36 bars). A UK drill beat (sliding 808, skittering hats, half-time clap on beat 3, rims) under
staccato cellos (the Pixel Pitch cello voice) + basses only. The violin's bow turns sample its line (as in #28); the
viola is the inner harmony a fifth under it: muted plucks on its counted rests, one sustained note per whole note;
MELODY AT LAST flips the arrangement (the viola sings `VIOLA_LEAD`, the violin harmonises softly). VIOLA JOKE: a
rimshot (ba-dum-tss), a synthesised crowd laugh (34 voices of formant-filtered "ha"s), then the open C + G fortissimo.
Checks: levels PASS (opening -0.3 dB vs the ending, spread 2.3 dB); band viola +3.7..+10 dB, violin +10..+17 dB over the
bed; no silent 0.5 s windows in either stem while fighting; intro -1.8 LU vs the fight (K-weighted), level with the ending.
