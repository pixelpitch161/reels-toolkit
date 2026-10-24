# #32 BALLFIGHT #6: Tuba vs Flute (`projects/ballfight_tuba_vs_flute`)

Biggest vs smallest, lowest vs highest, on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`,
`fighters.py`, `scene.py`, imported, plus `reelkit.ui.fight_hud`). TUBA (left corner, indigo) blasts a slow, huge brass
sound wave out of its ball on every bar (#28's `pulse` with `every=4`) and pulls the flute toward it all the time (new
weapon option `pull`: gravity that only steers, so the shared speed rule holds); every landed wave grows the reach and
the pull. FLUTE (right corner, silver) fires tiny fast darts on every beat (a light volley with the new option `on16`, so
haste can double it), half a dart more per landed volley. Same ball size and speed for both: "biggest vs smallest"
lives in the weapons and the sound. Abilities: OOMPAH (haste: waves every 2 beats) / BRASS WALL (shield) / SUBSONIC
(slam: a giant tuba bell hovers, then slams mouth-first); FLUTTER TONGUE (haste: darts on the 8ths) / TRILL (arc of 12)
/ PICCOLO (wave: a thin, shivering ring). Mood: villain march, C minor at 125.90 BPM (32 bars).

Honest run, seed 1097 (6 fits in 1099; rules frozen after tune.py): neck and neck all fight, the flute's chip damage
keeps it ahead until the tuba takes over at 44.9 s, the flute snatches it back at 48.6, both desperation moves fire on
the same beat (48.1 s), the bell lands at 49.1 and TUBA wins at 51.54 s with 4.5 HP. Balance over 1057 decided runs:
TUBA 49 %, FLUTE 51 %.

## Commands (from this folder)

```bash
python tune.py 20 T.base=1.8,2.3 T.pull=0.9,1.5 F.dmg=0.25,0.32   # rule tuning (done once, frozen in fighters.py)
python seed_search.py 1 2500 --stop 6                              # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 1097                                                 # one run's story (cached in check/run_1097.pkl)
python render.py                                                   # sprite cache (check/sprites_ballfight6.npz, ~30 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.95 8.0 14.4 19.7 24.9 29.7 45.0 48.3 49.15 51.6 54.0 58.4 60.7
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                              # subject CoC must be 0 px on every frame
python audio.py && python band_check.py                             # audio.wav + stems; fighters' bands over the bed
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the music box line, both solos |
| `sim.py` / `tune.py` / `seed_search.py` | #28's, with this pair's names (`tune.py`: `T.<key>`, `F.<key>`, `scale`) |
| `styles.py` | Draw functions for the new looks: `bell_wave`, the gravity rings, silver `darts`, `brass_wall`, the SUBSONIC bell, the piccolo ring, and the pixel-art flute; #28's ring / mask / dart helpers imported from its folder |
| `render.py` | #28's renderer, copied, with this pair's weapons, abilities, shards (brass chips, tubing, valve caps / silver tube, key cups) and the tuba's solo waves |
| `audio.py` | Villain march bed (snare ostinato, low brass pah, horn pad, music box, timpani); tuba waves = the bass line, flute volleys = the runs |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- The flute's Pixel Pitch model is a 1 px line at ball size, so the ball draws `styles.flute_sprite` (silver tube, lip
  plate, keys, ink outline) and the flute's glass is a cooler, darker tint so the silver reads.
- The pull is reconstructed for the picture from the recorded reach (both grow on exactly the landed main waves; with
  this roster both caps fall on the same count), so the engine's recorded arrays stay as #27 / #28 use them.
- Without the pull the tuba lands ~5-8 waves a fight and never wins; reach 1.8 + pull 1.2 (+0.08 per landed wave)
  brought it to ~49 %.
