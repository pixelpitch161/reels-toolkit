# Domino Symphony (IDEAS #17)

A sunlit low-poly gym floor, 10,000 dominoes, one push. A cat's paw taps domino #1; the line explodes into
64 rays out of step (a chaotic clatter), the rays funnel back together in pairs until the clicks lock into
steady 16ths, a bell lane peels off and every 4th domino rings a chime (the melody), it splits into 4 tuned
lanes (chords), a 4,096-domino field falls in a chevron sweep and its painted faces reveal a pixel music note,
a 2-lane spiral winds inward as the clicks speed up (16ths -> triplets -> 32nds), the wave reaches a red
holdout on a plank ledge that wobbles, settles, wobbles and teeters, then tips into a staircase of dominoes
each 1.5x bigger (booming D C# B A G F# E) and a 1.28 m giant slams down on the drop. Eight lanes race round
the whole layout in sync, playing the song on chime bars while the camera cranes up over the giant note;
the frame closes at the bottom on the final chord (10,000 in 52 s). Then everything rewinds and the paw
hovers over domino #1 again (seamless loop).

Build order:

```bash
python layout.py           # -> layout.npz (every box + strike time), test/map.png, counts, spacing, overlap pairs
python overlap_check.py    # footprint audit: standing overlaps must be 0 (fallen overlaps only at splits/merges)
python audio.py            # -> audio.wav + section levels, domino-band margin, quietest window
python engagement_check.py audio.wav     # 2 s windows: level + hits/s
python loudness_check.py audio.wav       # approx. K-weighted loudness per 2 s
python focus_check.py      # subject CoC must be 0 on all 1800 frames
python sheet_par.py test/sheet.png 0.02 1.6 4.4 6.2 9.5 15.5 21.6 25 29.6 34 39.3 42 47 54.5 57.4 59.3
python -m reelkit.video render.py audio.wav domino_symphony.mp4
```

| file | what |
|---|---|
| `timeline.py` | the 120 BPM grid, section times, the scene clock (rewind), harmony (D major), melody, captions |
| `layout.py` | builds every domino and chime bar with its strike time (no physics sim). Fan = binary tree of radial edges; funnel = polar "meridians" whose lateral spread halves between merge rows (no crossings); bell/chord lanes; the 64 x 62 field; 2-lane spiral; plank + staircase; the 3+3-lane frame. The funnel's domino count is solved so the total is exactly 10,000 |
| `boxes.py` | all boxes vectorised: fall model (rotate about the front edge to contact, then rest on the next), corners, normals, the holdout's wobble curve and its knock times |
| `scene.py` | gym shell constants, windows + `win_lit` (sun through the windows for floor, walls, beams, dominoes), maple floor albedo with court lines and tape marks, props, robot vacuum |
| `cat.py` | the cat's route and poses (tap, sit, walk with careful steps, startle), face details |
| `render.py` | ray-traced shell, screen-space sun shadow mask, per-pixel floor light, domino splats (tiny) + polygons in painter chunks, props, locked DOF, glows, window beams + dust, captions, counter |
| `audio.py` | clicks (density-balanced, bus-compressed), chimes from the bars, staircase booms, knocks/thuds, original arrangement, reversed un-clicks for the rewind, loop-aware master with a lookahead limiter |

Facts and flags:
- Choreographed, not real: every fall time comes from the music grid. Never caption it as a real build.
- The 1.5x caption is real (Lorne Whitehead, 1983: a domino can topple one about 1.5x its size); 8 steps from
  5 cm reach 1.28 m (calc).
- Field reveal trick: field dominoes are cream on every face while standing and show their pixel-art colour
  once they tip (their painted backs would otherwise face the camera before the sweep).
- Exactly 10,000 dominoes are struck by 51.97 s (last frame-lane pair lands on the final bar at 52.0 s).
- The cat never touches a lane; it steps over the spiral at ~29.6 s and crosses the left frame lane ~1.5 s
  ahead of its wave at ~41 s.
