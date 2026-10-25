# #33 BALLFIGHT #7: Ukulele vs Acoustic Guitar (`projects/ballfight_ukulele_vs_acoustic`)

**v3 (series standard, Oct 24): the VS title card intro.** Bar 0 is the shared `projects/ballfight/title.py` card
(pink UKULELE panel with surf crests, wood-tan ACOUSTIC panel with grain + six strings; UKULELE slams in on frame 0
with a uke chord blast, VS cracks the divider with a thunder crack, ACOUSTIC slams in on beat 3 with an open-chord
blast, the doors open on beat 4); the 3 . 2 . 1 is on bar 1, FIGHT! on bar 2 (3.70 s); no top caption. The title
slam is the frame-0 reset (doors shut over the last 0.30 s + riser + whoosh; seam 0.9x render). The fight starts at
2 bars, so the honest search was re-run (rules unchanged): seed 1130 (6 fits in 2170; UKULELE 53 % / ACOUSTIC 47 %
over 1779 decided): the uke leads early, the acoustic leads 35.6-48.7 s, UKE-NADO (44.4 s) and BIG STRUM (45.8 s)
trade, UKULELE wins at 50.97 s with 4.3 HP. Chords re-laid: card on Am, count-in on E, the cadence from FIGHT!.
v2 kept as `*_v2_backup.py` / `_v2.mp4`.

**v2 (BALLFIGHT v2 redesign, user Oct 24): SURF'S UP vs SIX-STRING RAKE, on `reelkit.fx`.** Little brother vs big
brother on #27's engine, ring, HUD and count-in; the way each is played is the weapon. UKULELE (left, pink): every 2nd
beat its strum raises a rolling low-poly wave it rides at the acoustic (weapon kind `surf`: a speed burst + a wave front
that crashes into the enemy once and pushes it along the wave; the wave height grows per landed wave). ACOUSTIC (right,
wood tan; "ACOUSTIC" everywhere, never GUITAR): on every beat six glowing string lines sweep sideways across the ring in
front of it, alternating down / up strokes, leaving six-line claw marks on the canvas (weapon kind `rake`; the length
grows per landed stroke). Both attacks go partly where the ball is already heading (`aim_k`), so they can miss.
Abilities: CHUNK (shockwave, a splash ring) / SPEED STRUM (waves every beat) / UKE-NADO (ability kind `spout`: a
waterspout that hunts the enemy across the ring, 4 ticks); CAPO (strokes on the 8ths, a capo clamps on) / FINGERPICK
(ability kind `whip`: six single strings flick out like whips, one per note) / BIG STRUM (ability kind `sweep`: all six
strings sweep the whole ring). Effects from `reelkit.fx`: attack lights, ink-starburst impacts, sea foam + wood-chip
particles, claw / water marks, shimmer on the splash, sweep and spout, ability moments, capped shake.
Spaghetti-western standoff: A minor, Am - G - F - E, 129.84 BPM (33 bars); the audio strum direction follows each stroke.

Honest run, seed 1248 (6 fits in 1248; rules re-tuned once with tune.py, frozen): the uke leads early (91-86 at 10 s),
29-30 at 40 s, BIG STRUM flips the lead at 41.9 s, the waterspout catches the acoustic for all 4 ticks, and UKULELE wins
at 52.33 s with 4.2 HP (the second lead change is the K.O. itself). Balance over 1104 decided runs: UKULELE 54 %,
ACOUSTIC 46 %. v1 (dash vs cone, seed 874) is kept as `*_v1_backup.py` and `ballfight_ukulele_vs_acoustic_v1.mp4`.

## Commands (from this folder)

```bash
python tune.py 160 scale=0.72 U.dmg=4.8 A.dmg=2.1 A.aim_k=0.47 U.grow=0.03 A.grow=0.03   # v2 rule tuning (done once, frozen)
python seed_search.py 1 8000 --stop 6                         # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 1130                                            # one run's story (cached in check/run_1130.pkl)
python render.py                                              # sprite cache (check/sprites_ballfight7.npz, ~25 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 3.32 9.3 13.5 20.0 25.1 41.95 42.7 52.35 55.0 59.6
python flash_test.py 40.5 46.0                                 # flash test on the busiest stretch before the full render
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                         # subject CoC must be 0 px on every frame
python audio.py                                               # audio.wav (2860 AAC frames) + check/stem_*.wav
python bandcheck.py                                           # fighters vs bed, 300-5000 Hz, per section and bed bus
python zoom.py test/zoom.png 2.85 3.79 13.62 25.12 41.85 42.75  # 2x crops of the ring at given times (detail check)
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py ballfight_ukulele_vs_acoustic_v3.mp4
python patch_frames.py 60.4 61.0 && python mux.py                               # a fix that touches only some frames: patch the kept raw, re-encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords + both fighters' voicings, the whistle theme, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (same windows as #27) |
| `tune.py` | Generic rule grid: `U.<weapon key>`, `A.<weapon key>`, `U.u1.<ability key>`, `scale` |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions: v2 the rolling low-poly `surf` wave, the six `rake` strings (+ motion fan), FINGERPICK whips, BIG STRUM's ring-wide strings, the waterspout; the capo; v1's cone / dash / pick / whirl kept |
| `render.py` | #28's renderer, copied: v2 attacks rebuilt from the run's events (closed-form motion), reelkit.fx wiring, shards (strings, spruce / koa chips, frets) |
| `audio.py` | The western bed; waves = the uke's strums + crashes, rake strokes = the acoustic's strums in the stroke's direction |
| `flash_test.py` | Renders a stretch and runs reelkit.flashcheck on it (before the full render) |
| `*_v1_backup.py` | v1 (dash vs cone) renderer, styles and audio |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |
| `patch_frames.py` | Re-renders a time span into the kept raw frames (this project's renderer), for `mux.py` |

## Rules worth knowing

- v2: aimed dead on, surf and rake both landed nearly every time (a deterministic damage race); `aim_k` (0.55 surf, 0.47
  rake) and slower growth (0.03 both) gave misses, variance and lead changes. Tune on decided runs only (tune.py prints it).
- v1 notes below (dash vs cone).
- Engine kinds (shared, additive, #27 / #28 replay identically): `dash` and `cone` fire on their own 16th-grid counter
  (`every` beats; haste divides it, so SPEED STRUM = every beat, CAPO = 8ths); haste shots hit for `sub_dmg` x and don't
  grow the weapon. A dash hits on contact once within `window` s; a cone hits once when its front crosses the enemy
  inside the wedge. `blade_len` holds the dash power / cone length (recorded as `BL`); the renderer rebuilds cones
  and dashes from the `cone` / `dash` events.
- Balance is touchy on the dash damage: 4.8 -> 5.2 moved the uke from ~51 % to ~59 % of decided runs. Tune on 160 seeds
  a cell; 64 seeds misread it by ~9 points. The natural story is the brief's: the uke leads at 15 s in ~75 % of runs and
  the acoustic comes back in ~30 % of those.
- Lead changes stay rare (~0.5 per run, as #27 / #28): both weapons grow on landed hits. Fit rate ~0.6 %.
