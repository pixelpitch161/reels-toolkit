# Log: `projects/falling_sand_wars`

Append new entries at the end (newest last).

## Falling Sand Wars (`projects/falling_sand_wars`)
- Battle-family reel in mood A (dark, hypnotic): four sands with their own physics (IRON sinks, PUMICE floats and rolls,
  CLAY piles steep and clings to walls, GLASS runs flat) poured into a glass box set in a turning brass ring, hung on
  chains under a cracked lintel in a night desert ruin (low full moon, two braziers, dunes, palms). Falling-sand engine
  (`sands.py`, 128 x 128 in the box's own frame, gravity snapped to 8 directions); the plug pops at bar 8 (20.33 s) in a
  seeded bottom corner; 45 deg turns on downbeats; the drained sand builds a strata dome on the dune. Seed 176: IRON out
  32.7, PUMICE 44.7, GLASS 51.1, CLAY wins keeping 388 grains; the closing roll pours the winner out and lands the empty
  box upright on 61.0 = frame 0 (the inlet springs open). Soundtrack: D Phrygian minimal techno at 94.43 BPM, drone +
  heartbeat from frame 0, each team's 16th-slot activity steps its part, turn hits, avalanche toms, drain tones, a stark
  D minor(b9) at the win.
- New reelkit pieces: `ui.share_bar` (fixed name slots + leader lines that slide with the segments, OUT state) and
  `ui.draw_captions(..., style='impact')` (drop, scale punch, text-only shake, sand puff) + `audio.thud()`.
- Lessons (sand sim):
  - Four separate streams make four side-by-side columns: different colours never touch, so nothing sorts. One shared
    inlet (a braided stream) mixes them, then density swaps sort the pile (iron core, clay on top, glass wide, pumice
    outside) - the "IRON SINKS / PUMICE FLOATS" shot needs that.
  - Draining a layered pile through any hole cuts across the layers (the crater exposes every layer), so all teams drain
    in proportion and the last ones go out together with almost nothing left. Selective elimination doesn't emerge.
    A knockout threshold (12 % of the start; the team's last grains go dark and still drain) gives pacing and a winner
    you can see. The physics doesn't depend on the threshold until the win, so one run per seed can be re-scored at
    many thresholds (`check/ko_sweep_*.txt`) instead of re-running the search.
  - Mid-wall holes leave pockets in the corners that no tilt drains; a corner notch is the one hole a tilt can make the
    lowest point. The turn rule "tip the hole lower with p 0.7" keeps the drain going; pure random turns stall for 10 s+.
  - A pile peaking at the inlet starves whichever hopper injects last: shuffle the hopper order every substep.
  - Single-file grains along a wall drain one per two passes: the closing pour opens the whole side wall and runs 3x
    substeps (the rattle), so the box is empty by ~59.5 s.
  - Balance is not even under the final rules: across 90 runs clay and pumice win most, glass a few, iron almost never
    (it sinks onto the bottom-corner notch). An earlier rule set had all four winning but no story. Next episode: add top
    corners to the seeded hole positions or lower the turn greed.
- Lessons (look): a big sky glow around a low moon washes the disc to white; a long lens (fov 36) compresses the sky, so
  the lintel's underside sat exactly at the moon's height until the ruin was raised. The strata pile reads as bands only
  when built ring by ring (lathe), not from a square mound grid.
- Time: ~4 h, far over the 60-75 min target; nearly all of it went into sim rules and balance, not the picture.
- Feedback: (pending)
