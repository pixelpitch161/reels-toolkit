# Maze Race (IDEAS #23)

Four explorers race one maze from its four corners to the centre. The maze has four-fold rotational symmetry, so
every corner faces exactly the same maze; the only difference is how each one explores. Mood B (heist chase), 61 s.

| explorer | how it explores | plays |
|---|---|---|
| FLOOD (blue ink) | breadth-first: spreads ring by ring (thorough, spread thin) | strings: staccato ostinato, every 4th ring surges |
| DIVER (red thread) | depth-first, seeded choice at junctions; winds its thread back at dead ends | bass: picked spy line, climbs with its depth; dead end = bonk + run down |
| SEEKER (gold light) | greedy best-first: always the frontier cell nearest the centre (a straight-line guess) | marimba: higher the closer to the centre; dead end = wooden bonk |
| MARBLE (steel ball) | a real ball: circle-vs-wall physics under the board's tilt (the tilt steers only the marble) | drums: every wall hit is a hit |

## Rules (the same for every run)

- **Fairness:** the three paint explorers uncover exactly the same number of new cells per beat: 1 per beat in the
  race (each on its own 16th: FLOOD on the beat, SEEKER on the &, DIVER on the a), 2 per beat in the final stretch
  (bar 20, 39.35 s). Bar 1 is a canon (2 cells each, entering on beats 1/2/3) so the instruments come in one by one
  with their labels. Moving through cells already uncovered (backtracking, darting back) is free.
- **Progress** (the legend's bars and ranks): 1 - true maze distance to the goal of the best cell reached / that
  distance from the corner. Finishers rank by finish time.
- **The board rocks on the beat:** each beat it tips toward a new seeded compass direction (eased over 0.3 s, at most
  1.8 deg on screen). Slow sweeps left the marble parked in pockets (median progress 0.15-0.3, ~1 wall hit/s).
- **Race end:** bar 26 (51.16 s). Anyone not in the centre by then is LOST.

## The run (maze 12382, tilt 1094; `seed.txt`)

MARBLE leads early (8.5-11.5 s), FLOOD takes over, DIVER leads 23.5-32 s, FLOOD (the tortoise) leads from 32 s,
SEEKER hits a dead end right next to the centre at 34.7 s ("SO CLOSE..."), then finds the way and wins at 44.4 s.
FLOOD is 2nd at 46.5 s, DIVER 3rd at 49.9 s (after winding back 18 cells at 39.4 s), the marble ends LOST at 56 %.
Played exactly as simulated.

Balance across 30,000 mazes (paint explorers only, these rules): SEEKER wins 74 %, DIVER 25 %, FLOOD 1 %. Under a
fair cell budget breadth-first is the tortoise. The 46 s finish is in the hard-maze tail (median first finish
25.5 s), so the search keeps mazes where SEEKER gets trapped. The marble never finished in ~10,000 tilt runs over the
top 4 mazes (best ~0.64 progress): it is the wildcard that can lead early but gets lost.

## Build

```bash
python seed_search.py 0 30000 --stop 4 --mazes 4 --tilts 2500   # check/mazes_*.txt, check/races_*.txt
echo "MAZE TILT" > seed.txt && python sim.py                    # cache the run -> sim.pkl
python audio.py                                                 # -> audio.wav (+ bus and section levels)
python -m reelkit.sheet render.py test/sheet.png 1.6 9.5 24 35.5 44.6 54.5 60
python -m reelkit.safezone render.py
python focus_check.py
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav maze_race.mp4    # the one full render (keeps frames_raw.rgb)
python mux.py                                                # re-encode the raw seam-safe -> maze_race.mp4
python -m reelkit.loopcheck maze_race.mp4
python -m reelkit.flashcheck maze_race.mp4
```

Why `mux.py`: the toolkit's mux failed `loopcheck` on the mp4 (the renderer passed). Frame 0 is a keyframe and the last
frame the end of a GOP; x264's macroblock tree also gives frame 0 a finer quantiser. On this dense dithered board that
read as a 3.4x seam jump. `mux.py` turns mbtree off and keyframes the last frame too (1.5x on the seam frames; the file
is ~1.7x bigger). And AAC pads its last 1024-sample frame with zeros (61.0 s isn't a whole number of them): a 13 ms
dropout before the loop. `audio.py` builds the soundtrack as a loop exactly 2860 AAC frames long (61.013 s) and
`mux.py` muxes it untrimmed, so the decoded audio wraps sample-perfectly; the file reads 61.01 s.
`python mux.py --patch A:B` re-renders A-B seconds into the kept raw before muxing.

| file | what |
|---|---|
| `maze.py` | the symmetric maze, the three step rules, the marble physics, the tilt, the fairness schedule; `Race(maze, tilt).run()` |
| `seed_search.py` | stage A ranks mazes by the paint race's story, stage B searches tilts for the whole four-way race |
| `sim.py` | runs the chosen seeds and caches every event (`sim.pkl`) |
| `timeline.py` | one clock for picture and sound: cell uncover times, DIVER's thread, ranks, the route, captions from events |
| `scene.py` | the desk: board parts (board-local), lamp, candles, clock, books, window + moon, each explorer's corner prop |
| `render.py` | back wall -> per-pixel desk -> props -> per-pixel board floor (texture: wood, baked wall shadows, paint) -> walls, rim, heads -> DOF -> glows -> HUD |
| `audio.py` | the race-driven soundtrack, mixed as a loop (tails wrap onto the start) |
| `focus_check.py` | board CoC = 0 on every frame |
| `mux.py` | seam-safe re-encode of the kept raw frames (+ `--patch A:B`) |

New reelkit piece: `reelkit.ui.race_lanes` (a race legend: one slim lane per racer, live rank with sliding lanes,
NAME + instrument, progress bar, 1ST/2ND badges, LOST).
