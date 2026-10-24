# #33 BALLFIGHT #7: Ukulele vs Acoustic Guitar (`projects/ballfight_ukulele_vs_acoustic`)

Little brother vs big brother on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`, `fighters.py`,
`scene.py`, imported, plus `reelkit.ui.fight_hud`). UKULELE (left corner, pink) fights up close: every 2nd beat it
throws itself at the acoustic in a body-slam dash (new weapon kind `dash`: a speed burst that decays like a knockback,
so power = speed + distance; +0.08 power per landed dash) and strums as it lands. ACOUSTIC (right corner, wood tan;
called ACOUSTIC everywhere so it never reads as #27's GUITAR) keeps it at range: on every beat it strums a cone of
sound out of its sound hole, aimed at the uke (new weapon kind `cone`: a 26 deg wedge, +0.06 length per landed strum).
Abilities: CHUNK (shockwave) / SPEED STRUM (haste: dashes every beat) / UKE-NADO (wave, a spinning whirl); CAPO (haste:
cones on the 8ths, a capo clamps across the ball) / FINGERPICK (arc of 6 string darts) / BIG STRUM (slam: a giant
tortoiseshell pick's downstroke). The uke sits smaller in its ball than the acoustic (sprite scale 0.70 vs 0.86).
Spaghetti-western standoff: A minor, Am - G - F - E, 129.84 BPM (33 bars), gallop + twang baritone + whistle + choir + bell.

Honest run, seed 874 (6 fits in 974; rules tuned once with tune.py, frozen): the uke bullies early (92-78 at 10 s),
it stays within ~10 HP from 20 s on, BIG STRUM (43.9 s) and UKE-NADO (44.8 s) swing the lead twice (44.9, 46.5 s),
9.4-9.4 at 50 s, and UKULELE wins at 51.87 s with 6.2 HP. Balance over 894 decided runs: UKULELE 57 %, ACOUSTIC 43 %.

## Commands (from this folder)

```bash
python tune.py 160 U.dmg=4.8,5.2 A.grow=0.06,0.075 scale=0.68   # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 4000 --stop 6                         # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 874                                             # one run's story (cached in check/run_874.pkl)
python render.py                                              # sprite cache (check/sprites_ballfight7.npz, ~25 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 4.1 7.95 14.0 22.3 27.6 44.1 45.25 51.9 54.6 59.6
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                         # subject CoC must be 0 px on every frame
python audio.py                                               # audio.wav (2860 AAC frames) + check/stem_*.wav
python bandcheck.py                                           # fighters vs bed, 300-5000 Hz, per section and bed bus
python zoom.py test/zoom.png 2.85 4.08 22.3 27.6 44.1 45.25   # 2x crops of the ring at given times (detail check)
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
python patch_frames.py 60.4 61.0 && python mux.py                               # a fix that touches only some frames: patch the kept raw, re-encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords + both fighters' voicings, the whistle theme, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (same windows as #27) |
| `tune.py` | Generic rule grid: `U.<weapon key>`, `A.<weapon key>`, `U.u1.<ability key>`, `scale` |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions for the new styles: the `soundhole` cone, the `uke_dash` afterimages, the capo, FINGERPICK darts, the BIG STRUM pick, the UKE-NADO whirl |
| `render.py` | #28's renderer, copied, with this pair's weapons, abilities and shards (strings, spruce / koa chips, frets) |
| `audio.py` | The western bed; dashes = the uke's strums, cones = the acoustic's strums (boom-chick when leading, thinning with HP) |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |
| `patch_frames.py` | Re-renders a time span into the kept raw frames (this project's renderer), for `mux.py` |

## Rules worth knowing

- Engine kinds (shared, additive, #27 / #28 replay identically): `dash` and `cone` fire on their own 16th-grid counter
  (`every` beats; haste divides it, so SPEED STRUM = every beat, CAPO = 8ths); haste shots hit for `sub_dmg` x and don't
  grow the weapon. A dash hits on contact once within `window` s; a cone hits once when its front crosses the enemy
  inside the wedge. `blade_len` holds the dash power / cone length (recorded as `BL`); the renderer rebuilds cones
  and dashes from the `cone` / `dash` events.
- Balance is touchy on the dash damage: 4.8 -> 5.2 moved the uke from ~51 % to ~59 % of decided runs. Tune on 160 seeds
  a cell; 64 seeds misread it by ~9 points. The natural story is the brief's: the uke leads at 15 s in ~75 % of runs and
  the acoustic comes back in ~30 % of those.
- Lead changes stay rare (~0.5 per run, as #27 / #28): both weapons grow on landed hits. Fit rate ~0.6 %.
