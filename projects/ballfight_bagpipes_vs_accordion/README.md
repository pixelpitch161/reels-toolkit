# #30 BALLFIGHT #4: Bagpipes vs Accordion (`projects/ballfight_bagpipes_vs_accordion`)

The "most hated instrument" final on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`,
`fighters.py`, `scene.py`, imported, plus `reelkit.ui.fight_hud`). Two new weapon kinds, both on the 16th grid:
BAGPIPES (left, tartan green) wears its drone as an **aura** (`aura`: a ring of radius R + reach around the ball that
hurts the opponent on every 8th while it is inside; +0.12 reach on the first landed tick of each contact, 0.9 -> 1.9),
ACCORDION (right, magenta) fights with its **bellows** (`bellows`: a stroke every 2 beats alternating a push ring that
shoves and a collapsing suction ring that yanks the bagpipes back in; +0.09 reach per landed main stroke, 0.9 -> 2.2,
knock grows with it). Pulling the bagpipes in drags the accordion into the drone, so every suck is a gamble (~6 times a
fight). Abilities: WAR CRY (shockwave) / DOUBLE DRONE (haste x2: ticks on the 16ths, a second tartan band) / BAG BURST
(wave); POLKA (haste x2: a stroke every beat) / SQUEEZEBOX (shield: a pleated collar folds shut) / FULL BELLOWS (slam: a
giant accordion, keyboard on top, squeezed shut on the predicted spot). Mood: dark sea shanty, A dorian at 121.97 BPM
(31 bars), stomps + gang claps + a low male-choir hum; the drone (seamless: whole cycles in the file) swells only while
the aura lands, every landed tick plays the chanter's shanty line, every stroke is an oom (push) or pah (pull) chord.

Honest run, seed 1581 (6 fits in 1854; rules frozen after tune.py): the accordion takes the lead at 16.9 s, the bagpipes
retake it at 28.8 s, FULL BELLOWS misses at 39.9 s, BAG BURST at 44.3 s, 8-7 at 48 s, and BAGPIPES win on the last tick
at 50.55 s with 2.1 HP. Balance over 1637 decided runs: BAGPIPES 51 %, ACCORDION 49 %.

## Commands (from this folder)

```bash
python tune.py 48 P.max_reach=1.9 A.pull_k=1.7 P.dmg=0.8,0.85 scale=1.0,1.1   # rule tuning (done once, frozen)
python seed_search.py 1 8000 --stop 6                       # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 1581                                          # one run's story (cached in check/run_1581.pkl)
python render.py                                            # sprite cache (check/sprites_ballfight4.npz, ~17 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 3.1 12.4 21.4 27.2 39.6 44.5 50.6 53.5 56.5 60.7
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                       # subject CoC must be 0 px on every frame
python audio.py                                             # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the chanter's 4-bar shanty line, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (same windows as #27) |
| `tune.py` | Rule grid: `P.<weapon key>`, `A.<weapon key>`, `P.u1.<ability key>`, `scale`; balance, K.O., lead changes, fit rate split by window, damage by kind, how often the suction drags the accordion into the drone |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions for the new styles: the `drone` tartan aura + tick ripples, `bellows` push / pull rings, the `squeezebox` collar, the `giant_bellows` |
| `render.py` | #28's renderer, copied, with this pair's weapons, abilities and shards (pipes, tartan, reeds / buttons, pleats, keys) |
| `audio.py` | Dark sea shanty bed; the drone + chanter = the bagpipes' part, push / pull chords = the accordion's |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- The aura only bites when the balls are close, so contacts (~18 a fight) come in bursts of ticks; growing the reach per
  contact (not per tick) keeps it from snowballing. A small base aura (0.35) gave almost no contacts.
- The bellows were first tried as the brief's literal "alternate bars" (a stroke per bar): the accordion went idle for
  ~2 s at a time; a stroke every 2 beats (push on 1, suck on 3) reads as breathing and keeps it in the fight.
- A bigger suction range (`pull_k` 1.3 -> 1.7) and a bigger max aura raised lead changes (0.5 -> 0.8 a run): every suck
  can backfire. Captions are the tight window here (first uses cluster): ~27 % of runs have 5 captions >= 4 s apart.
