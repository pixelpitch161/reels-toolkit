# Chaos Makes a Perfect Curve (IDEAS #14)

A giant brass-and-walnut Galton board in a sunlit Victorian lecture hall. 12 rows of pegs, a real
50/50 bounce at every peg, 2^12 = 4,096 paths into 13 bins. Built to the engagement brief: chaos and
sound from frame 0 (a cold-open torrent), never quiet, building to the lock-in where 1,000 random
balls sit almost exactly under the ideal bell curve.

Arc: torrent (1 IN 4,096) -> freeze + rewind, one hero ball plays its 12-note melody -> establishing
shot (arched windows, sunbeam, audience) -> second ball with its L/R path drawn -> Pascal's numbers
row by row (each row's pegs ping left to right) -> 10 balls -> staged golden "what if" ball goes left
12 times, gong, then dissolves (never counted) -> 100 / 300 / 1,000 balls with the groove -> drop:
the ideal curve traces over the real bins -> trapdoors drain, the bucket elevator refills the hopper,
the torrent restarts at 57.5 s = frame 0 (seamless loop).

Build order:

```bash
python sim.py                        # -> balls.npz (seeded runs) + bin table, chi-square, timing checks
python audio.py                      # -> audio.wav (+ section levels, science-band margins, quietest window)
python focus_check.py                # every frame: worst blur on the machine must be 0
python sheet_par.py test/sheet.png 0 4.6 6.5 10.6 15.6 22.6 24.9 44.5 49 52 55.5 59.97     # 2-process contact sheet
python -m reelkit.video render.py audio.wav galton_board.mp4
```

| file | what |
|---|---|
| `board.py` | peg/bin layout, hop parabolas, bin packing (rows of 4 and 3), ball colours (pearl -> bin colour, cool left / warm right) |
| `timeline.py` | single source of times: ball schedules + seeds, explanation beats, hopper level, captions, scale + chords |
| `sim.py` | simulates every ball once (main run seed 1889, torrent seed 4096, staged gold) and caches it |
| `motion.py` | ball positions / colours / contacts / landings / bin counts at any time, from `balls.npz` (shared by picture and sound) |
| `scene.py` | the hall (three arched windows, gallery, chalkboard, bookcase, lamps, benches, pawn spectators, pigeon) and the machine |
| `render.py` | ray-traced hall shell + backplate (static and per-ball shadows), machine, locked DOF, sunbeams, overlays, captions |
| `audio.py` | pings, landings, bin chord, curve arpeggio, groove, drain/elevator, loop hit |
| `focus_check.py` | focus audit without rendering |
| `sheet_par.py` | contact sheet rendered on 2 processes |

Facts (calc): paths per bin = C(12, b): 1, 12, 66, 220, 495, 792, 924, 792, 495, 220, 66, 12, 1
(sum 4,096). Expected for 1,000 balls: 0.2, 2.9, 16.1, 53.7, 120.8, 193.4, 225.6, ... The honest seeded
run (1889) gave 0, 8, 9, 48, 119, 194, 229, 198, 116, 58, 20, 0, 1 (chi-square 17 on ~10 dof); one
real ball reached the right edge bin, which is fine (P = 22 % for 1,000 balls). Galton described the
board in *Natural Inheritance* (1889); the chalkboard says "GALTON, 1889" and nothing about 1874.
The render uses the idealised 50/50 model, not a real-board recording; the golden all-left path is
staged and captioned as a "what if".
