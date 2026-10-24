# Falling Sand Wars (IDEAS #21)

Four sands with different physics are poured into a glass box with one hole. The box turns on the beat, the piles
avalanche, sand pours out of the hole onto the dune below, and the last colour still standing in the box wins.
A battle-family episode (see #20), but gravity and drainage instead of border fights, in a dark, hypnotic mood.

| sand | physics (its strategy) | plays |
|---|---|---|
| IRON (deep red) | heavy: sinks through everything (density swaps along gravity) | low piano strikes + sub |
| PUMICE (pale cyan) | light: floats up, drifts down slowly, rolls over the top of the pile | high glassy bells |
| CLAY (amber) | sticky: piles steeply (slides only off an overhang), clings to walls | muted frame drum |
| GLASS (violet) | slippery: runs sideways, flat, almost like water | plucked dulcimer |

## The engine

`sands.py` is a falling-sand grid (128 x 128) in the box's own frame. Gravity is the world's down turned into the box
frame and snapped to 8 directions (the box turns in 45 deg steps). Per substep: density swaps along gravity, a fall,
diagonal slides, sideways flow for glass and pumice, clay adhesion to walls. Grains that step into the open hole
leave and are recorded (time, material, cell), so the picture draws them falling and the soundtrack plays the
streams. `episode.py` holds the schedule and the rules; nothing in it knows about the video beyond the times.

Rules (the same for every run):
- **Pour (0-19 s):** four hoppers pour the same number of grains each (42 % of the box in total) through one shared
  inlet, so the colours mix and then sort themselves (iron core, clay on top, glass spread wide, pumice outside).
- **The hole (20.33 s, bar 8):** the plug pops out of a seeded bottom corner (a notch: a corner is the one place a
  tilt can make the lowest point, so no sand is trapped for good).
- **Turns:** 45 deg on downbeats (every 2 bars, then every bar, then every half bar). Each turn tips the hole lower
  with probability 0.7 (a seeded coin), else the other way.
- **Knockout:** a team that drops under `KO_FRAC` of its starting sand is out: its last grains go dark (they still
  behave as sand and drain normally). The third knockout is the win: the box stops, the plug snaps back.
- **Closing roll (57.19 s):** the plug pops again, the box rolls over until the notch corner points straight down,
  holds while the winner pours out (the box rattles, so every grain lets go), then turns the last 45 deg and lands
  upright and empty at exactly 61.0 s = frame 0, where the inlet springs open and the next pour starts.

The seed decides the hopper order, the hole corner, the pour noise, every turn's coin and every random choice in the
grain rules. `seed_search.py` ranks honest runs by the beat sheet; the chosen run is played exactly as simulated.

## The run (seed 176, knockout at 12 %)

Hole in the bottom-left corner. IRON is out at 32.7 s, PUMICE at 44.7 s, then CLAY vs GLASS until GLASS goes dark at
51.1 s: CLAY wins with 388 grains (23 % of its start) still in the box. The physics doesn't depend on the knockout level
until the win, so the 90-run sweeps in `check/ko_sweep_*.txt` re-score one run per seed at several thresholds.
Balance under these rules: clay and pumice win most runs, glass a few, iron almost never (it sinks onto the notch).
Earlier rule sets had all four winning but no usable story; see docs/LOG.md for what to try in episode 2.

## Build

```bash
python seed_search.py 0 400 --stop 5       # rank honest seeds (check/seeds_A_B.txt)
echo SEED > seed.txt && python sim.py      # cache the chosen run -> sim.npz
python audio.py                            # -> audio.wav (+ section levels, sand over the music bed)
python -m reelkit.sheet render.py test/sheet.png 1 5 8 14 20.5 26 33 40 47 51 55 58.5
python -m reelkit.safezone render.py
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav falling_sand_wars.mp4
python -m reelkit.flashcheck falling_sand_wars.mp4
```

| file | what |
|---|---|
| `sands.py` | the falling-sand engine (materials, passes, the corner notch, exit records) |
| `episode.py` | the schedule, the balance knobs, the turn rule, the knockout rule, the closing roll; `Run(seed).run()` |
| `seed_search.py` | runs seeds in parallel and ranks their stories (nothing is ever hand-edited) |
| `sim.py` | runs the chosen seed and caches every frame (`sim.npz`) |
| `timeline.py` | one clock for picture and sound: captions (impact style), shares, knockouts, the strata pile |
| `scene.py` | desert ruin: pillars, cracked lintel, ring + box (turning parts in box-local coords), hoppers, braziers, dunes |
| `render.py` | sky + moon -> floor with moon shadows of everything -> objects -> per-pixel sand in the box -> pile -> grains |
| `audio.py` | mood A soundtrack: drone + heartbeat, sand ticks, gated parts, turn hits, tom rolls, drain tones, chord |

New reelkit pieces built for this video: `reelkit.ui.share_bar` (named leader lines) and
`reelkit.ui.draw_captions(..., style='impact')` with `reelkit.audio.thud()`.
