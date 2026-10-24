# #28 BALLFIGHT #2: Drums vs Violin (`projects/ballfight_drums_vs_violin`)

Brute force vs finesse on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`, `fighters.py`,
`scene.py`, imported, plus `reelkit.ui.fight_hud`). DRUMS (red corner) blasts a drumhead ring out of its ball on every
beat (new weapon kind `pulse`; reach +0.07 per landed pulse), so its attacks are the groove. VIOLIN (blue corner) spins
its bow like a rapier (blade with the new `spin_grow` option: 2.4 -> 14 rad/s, +0.7 per landed hit), so slow strokes
become frantic spiccato, and the bow's quarter turns play the melody. Abilities: DRUM ROLL (haste x4 = pulses on the
16ths for a bar) / CRASH CYMBAL (shield; the bow bounces off) / DOUBLE KICK (slam: the beater taps an 8th early, then
stomps); PIZZICATO (arc of 5 darts on the 16ths) / TREMOLO (haste x2.5) / SCREECH (wave). Mood B, dark Balkan fire:
D harmonic minor at 141.64 BPM, a drum-less bed (upright-bass oom-pah, musette accordion, cimbalom shimmer).

Honest run, seed 568 (6 fits in 932; rules frozen after tune.py): Drums edge the opening, the speeding bow takes over at
18 s and builds a 62-26 lead, DOUBLE KICK (37.7 s) starts the comeback, SCREECH answers (43.6 s), and DRUMS win on the
last hit at 50.91 s with 2.8 HP. Balance over 767 decided runs: DRUMS 57 %, VIOLIN 43 %.

## Commands (from this folder)

```bash
python tune.py 40 D.dmg=2.4,2.6 V.dmg=1.8,1.9 scale=1.3   # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 2000 --stop 6                       # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 568                                           # one run's story (cached in check/run_568.pkl)
python render.py                                            # sprite cache (check/sprites_ballfight2.npz, ~20 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 9.0 12.9 22.2 32.8 38.45 43.9 50.95 53.5
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                       # subject CoC must be 0 px on every frame
python audio.py                                             # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the violin's 4-bar line, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (same windows as #27) |
| `tune.py` | Generic rule grid: `D.<weapon key>`, `V.<weapon key>`, `D.u1.<ability key>`, `scale`; balance, K.O., lead changes, fit rate |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions for the new styles: `bow`, `drumhead` pulse rings, pizzicato `dart`, the `cymbal` shield, the kick `beater` |
| `render.py` | #27's renderer, copied, with this pair's weapons, abilities, shards (strings, wood, bow bits / skins, sticks, bronze), the drum's reach circle and the drum-solo rings |
| `audio.py` | Drum-less Balkan bed; pulses = the kit (thins with the drums' HP), bow turns = the melody (scratchy and sparse with the violin's HP) |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- Lead changes are naturally rare for this pair (~0.5 per run; #27 ~0.7): both weapons grow on landed hits, so leads
  snowball. Smaller growth didn't help; the fit rate (~0.6 %) is fine for an early-stopping search (~1 min).
- `pulse` hits test the ring against the opponent like a shockwave; DRUM ROLL's extra 16th pulses hit for 0.45x and don't
  grow the reach. Pulses swat darts they pass (`clears`).
- The drum's beat thinning (audio): fills above 45 HP, one tom to 25, snare to 25 (then beat 4 only to 12), lonely kick.
