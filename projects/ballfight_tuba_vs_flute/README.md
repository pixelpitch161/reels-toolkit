# #32 BALLFIGHT #6: Tuba vs Flute (`projects/ballfight_tuba_vs_flute`)

Biggest vs smallest, lowest vs highest, on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`,
`fighters.py`, `scene.py`, imported, plus `reelkit.ui.fight_hud` and the shared effects kit `reelkit.fx`). Mood: villain
march, C minor at 125.90 BPM (32 bars).

**v2 (BALLFIGHT v2 redesign, user Oct 24): the instrument's body / the way it's played is the weapon.**
- **TUBA (indigo, left): OOM-PAH SWALLOW** (engine kind `swallow`). Every bar the giant bell goes "oom": it inhales
  for a beat (air streaks pour into it, the suction cone steers the flute in); touching the bell, the flute is
  swallowed and rides inside the bulging tuba; on "pah" (beat 3) it is spat out like a cannonball toward the open ring,
  and a rope hit within 0.9 s is the cannonball damage. A landed swallow lengthens the suction reach. OOMPAH (haste:
  oom-pah every 2 beats) · BRASS WALL (shield: the bell becomes a brass dome) · SUBSONIC (slam: a ring-shaking low
  note, the floor ripples like water under the flute).
- **FLUTE (silver, right): SNAKE CHARMER** (engine kind `striker`). It drops a woven basket where it stands every bar
  (up to 2; the oldest is taken back); a silver cobra rises from each, sways to the charmer line and strikes on the beat
  at the tuba when it's in reach; landed bites lengthen the reach and quicken the lunge. FLUTTER TONGUE (haste: strikes
  on the 8ths) · SNAKE DANCE (new ability kind `dance`: the cobras leave the baskets and slither after the tuba for a
  bar) · PICCOLO (wave: a piercing ring-wide note; glass-crack marks).
- Honest run, seed 514 (6 fits in 618 under the frozen rules; balance over 515 decided runs: TUBA 53 % / FLUTE 47 %):
  the tuba leads to 33 s (four swallows by 26 s), the flute takes over, the tuba claws back at 47.6 s with two late
  swallows, FLUTE wins at 49.37 s on a double cobra strike with 5.7 HP. Captions: FLUTTER TONGUE 15.3, BRASS WALL 21.9,
  SNAKE DANCE 28.1, SUBSONIC 40.0, PICCOLO 44.8 (OOMPAH 17.2 in the status line).
- v1 (big brass pulse + gravity pull vs darts, seed 1097, tuba won): `*_v1_backup.py`, `ballfight_tuba_vs_flute_v1.mp4`.

## Commands (from this folder)

```bash
python tune.py 120 F.dmg=1.08 scale=1.06                          # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 8000 --stop 6                              # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 514                                                  # one run's story (cached in check/run_514.pkl)
python render.py                                                   # sprite cache (check/sprites_ballfight6.npz, ~30 s)
python -m reelkit.sheet render.py test/sheet_v2.png 0.0 1.95 6.05 6.4 6.75 7.42 15.4 22.3 28.6 40.5 41.25 45.1 49.4 54.0 58.4 60.7
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                              # subject CoC must be 0 px on every frame
python audio.py && python band_check.py                            # audio.wav + stems; fighters' bands over the bed
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py ballfight_tuba_vs_flute_v2.mp4
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets (+ the 4 s no-hit window), scripted tail, chords, the music box line, the charmer line, both solos |
| `sim.py` / `tune.py` / `seed_search.py` | #28's, with this pair's names (`tune.py`: `T.<key>`, `F.<key>`, `scale`; reports the share of decided fights and the longest gap without a hit) |
| `styles.py` | v2 looks: the tuba's bell (flare, open on oom, punch on pah), the suction cone, the brass dome, the water ripples, woven baskets, silver cobras (outlined, hood turned partly to camera); v1's looks kept above them; #28's helpers imported |
| `render.py` | #28's renderer with this pair's v2 weapons, wired to `reelkit.fx` (attack lights, ink bursts, capped shake, shimmer, fringe, particles: wind streaks / puffs / scales / glass / brass sparks / foam, fading marks: fang claws, cracks, smoke rings, water, glass, ability moments) |
| `audio.py` | Villain march bed (snare ostinato, low brass pah, horn pad, a softer music box, timpani); oom = the tuba's low inhale note (the root), pah = the fifth as a blast (a brass blast + whoosh when it spits); the charmer line while the cobras are up (flutter-tongued in FLUTTER TONGUE, an octave up in SNAKE DANCE), a hiss-click on every strike |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- The flute's Pixel Pitch model is a 1 px line at ball size, so the ball draws `styles.flute_sprite` and the flute's
  glass is a cooler, darker tint so the silver reads.
- Balance is very sensitive to the cobra bite (1.05 -> 1.25 damage moved the tuba from 53 % to 17 %): tune on 90-120
  seeds per cell; 32-48 seeds gave a wrong answer once (21 % in the search).
- The first v2 rules opened slowly (first hit 5-6 s, stretches of 5+ s without a hit): a basket every bar instead of
  every 2 bars, a longer starting reach, and a hard window "no 4 s without a hit" in the seed search.
- The swallowed ball is skipped in `move()` (attribute `held`); it rides at the swallower's centre and is drawn
  gulped into the bell, hidden, then popped out at the spit.
