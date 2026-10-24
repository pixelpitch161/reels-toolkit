# #29 BALLFIGHT #3: Trumpet vs Sax (`projects/ballfight_trumpet_vs_sax`)

Jazz's oldest argument, who's the lead horn, on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`,
`fighters.py`, `scene.py`, imported, plus `reelkit.ui.fight_hud`). TRUMPET (left corner, yellow) fights with a **beam**
(new weapon kind): its bell charges for a beat with the aim locked on where the sax is heading (a dashed sight shows
it), then fires a straight piercing blast that hits once, pierces smoke rings in its path, and grows longer and wider
with every landed blast. SAX (right corner, teal) blows **smoke rings** (volley with the new `curve` option): wobbly
rings that home in on the trumpet at a limited turn rate and fade after 2.6 s; half a ring more per landed volley.
Abilities: FANFARE (shockwave) / DOUBLE TONGUE (haste: a blast every beat) / HIGH C (wave); SMOKE SCREEN (shield) /
BEBOP RUN (arc of 8 notes) / HONK (slam: a giant sax bell hovers over the predicted spot, then blasts).
Noir jazz: C minor, i - bVI7 - iv - V7, 133.77 BPM (34 bars), walking bass, brushes then sticks, muted piano comping.

Honest run, seed 337 (18 fits in 899 runs; rules frozen after tune.py): Trumpet leads from the first blast
(up 16 at 15 s), the sax's rings snowball back, four lead changes from 40.6 s, HIGH C (42.6 s) and HONK (43.5 s) back to
back, TRUMPET wins on the last hit at 49.34 s with 5.4 HP. Balance over 893 decided runs: TRUMPET 43 %, SAX 57 %.

## Commands (from this folder)

```bash
python tune.py 100 T.dmg=2.05,2.1 S.dmg=0.6 scale=0.95   # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 3000 --stop 6                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 337                                          # one run's story (cached in check/run_337.pkl)
python render.py                                           # sprite cache (check/sprites_ballfight3.npz, ~15 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.2 3.2 10.42 17.2 24.0 29.75 42.75 44.0 49.45 51.6 53.6 58.6 60.7
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py                                            # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (same windows as #27) |
| `tune.py` | Rule grid: `T.<weapon key>`, `S.<weapon key>`, `T.u1.<ability key>`, `scale`; balance, K.O., lead changes, beam accuracy |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions for the new looks: the beam's sight + blast (light out of the trumpet's bell: soft halos, rounded tip, ripples, a muzzle flash; it lets go of the bell as it fades), `smoke` rings, bebop `note`s, the smoke screen, the honk bell, solo notes |
| `render.py` | #28's renderer, copied, with this pair's weapons and abilities; the trumpet sprite rolls inside its ball to point along its aim |
| `audio.py` | Noir-jazz band; volleys (beats 1 + 3) = sax licks, blasts (beats 2 + 4) = trumpet stabs climbing per landed hit |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- Beam: aim locks when the charge starts at the opponent's spot predicted `lead` (0.7) x the charge time ahead; walls,
  clashes and knockbacks make it miss (~2/3 land). The bell tracks the opponent at 5 rad/s between shots (recorded in
  `TH`, so the renderer can point the trumpet).
- Balance is touchy on beam damage (4.0 -> 4.4 swung the trumpet from 28 % to 68 % at a 0.5 damage scale). Weapon damage
  was lowered instead of the global scale so the desperation moves keep their punch (they create the late lead changes).
- The trumpet leads at 15 s in ~97 % of runs and wins ~45 %: the rings' growth is the natural comeback.
- The honk bell stays low (mouth <= 2.9, 1.0 tall): any higher and its throat leaves the sharp band near the front corner.
- v2 (user, Oct 24): the Pixel Pitch trumpet model was rebuilt from reference photos (series file, own commit), and the
  blast now starts at the trumpet's bell inside the glass (`styles.BELL_IN` = 0.76 R along the aim) instead of with a
  hard edge at the ball's rim; the brass stub on the glass is gone (the trumpet itself turns to its aim).
