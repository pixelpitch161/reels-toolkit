# #36 BALLFIGHT #10: Trombone vs Tuba (`projects/ballfight_trombone_vs_tuba`)

Who owns the low end? The **first crossover**: TROMBONE (left corner, mint, from #34) and TUBA (right corner, indigo,
from #32 v2) fight **exactly as built** in `projects/ballfight/fighters.py` (weapons, numbers, abilities, looks, Pixel
Pitch voices, idioms). The lance shoots out on every beat; the tuba inhales on "oom", swallows, and spits the enemy into
the ropes on "pah". Also the **first episode with the HUD ability list** (series rule, Oct 25).

## Crossover rule: the only lever is a damage handicap

Shared engine change (own commit): `Rules(dmg_side=(left, right))` scales the damage each fighter deals on top of the
shared `dmg_scale`; without it every rule runs as before (`tools/ballfight_replay_hash.py` identical for all finished
episodes). Tuned once with `tune.py` and frozen in `timeline.DMG_SIDE`:

- The lance lands ~70 hits a fight, the swallow ~12, so the pair needs a big split: **TROMBONE x0.35, TUBA x1.55**
  (shared scale 1.0). 300 seeds: TROMBONE 49 % / TUBA 51 % of 261 decided, median K.O. 50.8 s, fit rate 2.7 %.
- For reference, the trombone ran at 0.60 in #34 and the tuba at 1.06 in #32 (different opponents).

## The run: honest seed 358

Search with the fight at 2 bars and #27's hard windows + the 4 s no-hit window (`seed_search.py 1 4000 --stop 6`):
6 fits in 416 runs, balance 48 % / 52 % over 367 decided. Seed 106 scored highest (7.81) but its K.O. at 52.94 s
leaves ~1 s for "COMMENT THE NEXT MATCH-UP" and cuts the solo to 5 beats, so the episode takes the next one, **358**
(7.60): the lance builds an early lead, the tuba's swallows drag it back (lead changes 19.1 / 26.7 / 45.1 s),
SUBSONIC at 40.0 s, PEDAL TONE at 44.3 s, and the tuba gulps the trombone at 48.6 s and spits it into the ropes for
the K.O. at 49.10 s with 5.9 HP left. Captions: SAD TROMBONE 11.4, BRASS WALL 19.5, POWER SLIDE 24.3, SUBSONIC 40.0,
PEDAL TONE 44.3 (OOMPAH 15.3 lights in the HUD list only: 4 s rule).

## HUD ability list (`reelkit.ui.fight_hud(..., abilities=...)`, own commit)

Under each HP bar: `LV2 <unlock 1>`, `LV3 <unlock 2>`, `LOW <desperation>`, aligned to the fighter's outer edge. Grey
while locked; the fighter's colour once unlocked (a glow sweeps across the name at the level-up); white-hot on a
rounded glowing pill in the fighter's colour on every use, held while a shield / haste lasts (a slam until it lands),
easing back over 0.5 s. LOW stays grey until it fires. The list replaces the LV pips and the status line; the HUD grows
18 px upward and still ends at the bottom of the safe zone. The rows come from the run's `level` / `ability` events
(`render.ability_rows`); frozen through the hit-stop, back to grey when HP refills on the glide home (as on frame 0).
The balls never get below y 417 (internal) in this run, so the ring didn't need nudging up.

## Music, mood C: brass house

F minor at 125.90 BPM (32 bars). The bed has no bass synth and no brass: four-on-the-floor kick (short, high-passed at
48 Hz so the tuba stays on top), claps, offbeat hats, a pumping filtered pad, high syncopated synth stabs; the back half
opens up after a sweep into bar 17. The TUBA is the bass line (oom = root, pah = fifth, OOMPAH doubles them); the
TROMBONE's thrusts are the tenor hook (`timeline.TROMBONE_LINE`, F3-Ab4, gliding into each note). Band check
(`band_check.py`): tuba 40-260 Hz +6.9..+8.6 dB over the bed, trombone 170-1200 Hz +7.9..+11.8 dB.

## Commands (from this folder)

```bash
python tune.py 300 ht=0.35 hu=1.55                         # handicap tuning (done once, frozen in timeline.DMG_SIDE)
python seed_search.py 1 4000 --stop 6                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 358                                          # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight10.npz, ~45 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 2.6 11.6 14.0 19.8 24.6 40.6 44.7 48.8 50.6 60.8
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py && python band_check.py                    # audio.wav + stems; fighters over the bed
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | Fighters, title, tempo, count-in, the frozen handicap, story targets, scripted tail, chords, the trombone hook, the tuba's roots, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine (with `dmg_side`); story analysis + score |
| `tune.py` | Handicap grid (`ht=` TROMBONE, `hu=` TUBA damage scale): win share, K.O. time, fit rate |
| `seed_search.py` | #32's early-stopping search (hard windows incl. no 4 s without a hit) |
| `styles.py` | No new styles: imports #34's (slide lance, PEDAL TONE's bell) and #32's (tuba bell, suction cone, BRASS WALL dome, SUBSONIC ripple) draw functions |
| `render.py` | #34's renderer with #32's tuba merged in, the TUBA title panel centred on the right-hand ball, PEDAL TONE's bell mint-lit, and the HUD ability list |
| `audio.py` | Brass house bed + both fighters from the run |
| `band_check.py`, `focus_check.py`, `mux.py` | Checks and the loop-clean encode |
