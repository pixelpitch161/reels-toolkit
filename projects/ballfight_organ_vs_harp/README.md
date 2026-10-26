# #35 BALLFIGHT #9: Organ vs Harp (`projects/ballfight_organ_vs_harp`)

Cathedral vs angels on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`, `fighters.py`, `scene.py`,
imported, plus `reelkit.ui.fight_hud`). ORGAN (left corner, wine) erupts rows of organ pipes out of the canvas toward the
harp on every second beat (new weapon kind `line`: the row runs out at 6 u/s, each pipe stays up 0.55 s, one hit per
row; +0.06 length per landed row). HARP (right corner, pearl) stretches a glowing tripwire string rope to rope across
the ring, square to the line between them, on every second beat (new kind `trap`): crossing an enemy string cuts for
1.8 x 0.6, the harp passes freely, it keeps int(n_vol) strings (1, +0.5 per landed crossing, max 7) and the oldest
fades. Abilities: CATHEDRAL (shield: an echoing gothic dome) / PULL ALL STOPS (haste x2: a row every beat) / TOCCATA
(slam: a forest of pipes erupts); PLUCK (arc of 7 darts) / ANGEL WINGS (shield: wings + halo) / 47 STRINGS (wave).
Mood: gothic horror, D minor at 125.90 BPM (32 bars): choir "aah", tolling bell, clock tick, heartbeat into a pounding
back half; the organ fighter plays the chord stabs, the harp the arpeggios.

**v4 (HUD ability list, series rule Oct 25):** the same fight (seed 237), title card and soundtrack, with
`fight_hud`'s `abilities` option (#36): ORGAN's and HARP's LV2 / LV3 / LOW abilities listed under their HP bars, grey
until unlocked (a glow sweeps across the name), the fighter's colour after, lit on a pill on every use (shields / haste
held while active, TOCCATA until it lands, else a flash easing back over 0.5 s), back to grey on the glide home; no
level pips or status line. The taller HUD (top y 448 internal) reached the front corner post, so the ring is nudged up
8 px (camera `SY` 288 -> 280): the fight stays >= 14 px clear of the HUD, the back post top just under the captions.
v3 kept as `render_v3_backup.py`.

**v3 (VS title card intro, series standard Oct 24):** bar 0 is the shared VS title card (`projects/ballfight/title.py`:
wine organ-pipe panel vs pearl harp-string panel, ORGAN / VS / HARP slamming in with a full-organ blast, a thunder crack
and a harp sweep over unison band hits; the doors open on beat 4), the 3 . 2 . 1 on bar 1, FIGHT! on bar 2 (3.81 s); no
top caption. The honest search was re-run with the fight at 2 bars (rules unchanged): **seed 237** (6 fits in 999 runs;
ORGAN 52 % / HARP 48 % over 813 decided): neck and neck all the way (29-28 at 42 s, 18-19 at 46 s), four lead changes
(33.7, 39.5, 46.5, 50.8 s), the organ takes it back at 50.8 s and wins at 51.12 s with 8.6 HP. The doors shut over the
last 0.30 s and the title slam on frame 0 is the loop's reset. v2 (`*_v2_backup.py`) and v1 (`*_backup.py`) kept.

v1 / v2 honest run, seed 1181 at 1 bar (10 fits in 1,179 runs; rules frozen after tune.py): the organ's rows bully early, the string web
fills and the harp takes the lead at 33.4 s, the lead flips at 38.1 and 43.9 s, 47 STRINGS (40.5 s) and TOCCATA (41.9 s)
trade blows, the organ trails 6-14 at 48 s and wins on its last hits at 51.66 s with 4.5 HP left (4 lead changes).
Balance over 1,042 decided runs: ORGAN 52 %, HARP 48 %.

## Commands (from this folder)

```bash
python tune.py 48 O.dmg=4 H.dmg=1.7,1.8 H.n0=1 H.grow=0.5 scale=0.6   # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 3000 --stop 6                                 # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 237                                                     # one run's story (cached in check/run_1181.pkl)
python render.py                                                      # sprite cache (check/sprites_ballfight9.npz, ~40 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 0.48 0.95 1.72 2.0 3.85 12.0 24.4 29.7 43.5 51.2 54.5 60.97
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                                 # subject CoC must be 0 px on every frame
python audio.py                                                       # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords and sections, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (same windows as #27); `hazards(run)` rebuilds the pipe rows and strings from the run's events |
| `tune.py` | Generic rule grid: `O.<weapon key>`, `H.<weapon key>`, `O.u1.<ability key>`, `scale`; balance, K.O., lead changes, fit rate |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions for the new styles: `pipes` rows, tripwire `strings`, PLUCK `dart`, the CATHEDRAL `dome`, ANGEL `wings` + halo, the TOCCATA `forest`, the 47 STRINGS `snap` |
| `render.py` | #28's renderer, copied, with this pair's weapons, shields, slam, wave, shards (pipes / case wood / gold, or strings / maple / gold) and solo looks |
| `audio.py` | Gothic-horror bed with no organ or harp of its own; pipe rows = organ stabs, strings = harp arpeggios, crossings ring the string; each side thins and dirties with its HP |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- Fast, wide pipe rows landed ~80 % of the time and won by ~28 s; slower rows (6 u/s, half-width 0.15) and a short
  starting length make the organ strong up close and early. Starting the harp with one string (growing +0.5 per landed
  crossing) makes it weak early and strong once the web fills: the natural story is "organ bullies, harp claws back",
  with ~0.6-0.8 lead changes per run (#27 ~0.7, #28 ~0.5) and a ~1 % fit rate.
- The engine reports hazards as events (`line`, `string`, `string_off`; hits `pipe` / `string` carry the hazard id), so
  the renderer and the audio rebuild them with `sim.hazards()`; no new recorded arrays.
- `timeline.beat_pulse` ramps over 0.16 s (#27 / #28: 0.07 s): the pearl corner light is bright, so its beat pulse moved
  enough of the frame to fail the loop seam (2.7x -> 1.3x). The balls' sprite yaw ignores the corner rev shake.
