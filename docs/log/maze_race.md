# Log: `projects/maze_race`

Append new entries at the end (newest last).

## Maze Race (`projects/maze_race`)
- Four explorers race one 20x20 maze with four-fold rotational symmetry (one quadrant grown from the goal door, rotated;
  symmetric links between quadrants and braids add loops) from the corners to a brass goal inlay, on a carved wooden
  board on a scholar's desk at night (green banker's lamp as the visible key, two candles, mantel clock ticking on the
  beat, moth, dust in the beam, moon in the window, each explorer's prop by its corner). FLOOD = BFS ink (strings),
  DIVER = random DFS with a red thread that winds back (bass), SEEKER = greedy best-first gold light (marimba), MARBLE =
  real circle-vs-wall physics under the board's tilt (drums: every wall hit is a hit). One fair budget: the paint
  explorers uncover the same cells per beat, each on its own 16th. Mood B heist chase, D minor, 121.97 BPM.
- Run (maze 12382, tilt 1094; two-stage `seed_search.py`, never edited): MARBLE leads early, FLOOD, DIVER, FLOOD the
  tortoise from 32 s, SEEKER's dead end next to the centre at 34.7 s, SEEKER wins 44.4, FLOOD 46.5, DIVER 49.9, marble
  lost. Captions are written from the run's events (lead changes, traps, the tortoise).
- Lessons (sim):
  - Under a fair cell budget breadth-first is the slowest: over 30k mazes SEEKER wins 74 %, DIVER 25 %, FLOOD 1 %. At 2
    cells per beat everyone finished in 7-25 s; 1 per beat (2 in the final stretch) and the hard-maze tail give a 44-46 s
    finish. Quadrants grown from a random cell gave short routes (~18 cells); growing from the goal door and a 20x20
    board gives 35-50.
  - A slowly sweeping tilt leaves a marble parked in pockets (median progress 0.15-0.3, ~1 wall hit/s); a new seeded
    direction on every beat gives ~3 hits/s (real drum fills) but it still never reached the centre in ~10k tilts over
    the top 4 mazes (best 0.64). Treat it as the wildcard; a marble that can win needs a different board (or bowl).
  - The search is cheap (paint race 1 ms, marble 0.15 s): rank mazes on the paint story first, then search tilts.
- Lessons (loop/encode):
  - `reelkit.loopcheck` passed on render.py but failed on the toolkit mp4 (3.4x seam, 7.8 dB audio step). Causes: frame 0
    is a keyframe and the last frame a P-frame, and x264's mbtree gives frame 0 a finer quantiser (on a dense dithered
    board that's 1.2-1.8 of seam diff); AAC pads its last 1024-sample frame with zeros (61.0 s = 2859.4 frames), so the
    decoded file ends in 13 ms of silence. `mux.py` re-encodes the kept raw with mbtree off and a keyframe on the last
    frame (1.6x), and the soundtrack is built as a loop exactly 2860 AAC frames long and muxed untrimmed (file 61.01 s).
    Worth moving into reelkit.video if other dithered videos hit it.
  - Captions and labels that fade in on frame 0 add a real seam jump (0.81 vs 0.29 normal): start the hook caption on
    frame 1 and ramp labels from zero; GO still lands on frame 0.
- Render ~0.45 s/frame/core (6.6 min on 2 cores); `mux.py` ~2.5 min.
- Feedback: (pending)
