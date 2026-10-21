# The Perfect Loop (IDEAS #11)

15 uncoupled pendulums in a clockmaker's attic at sunset. Pendulum k swings 51 + k times a minute,
so they drift through travelling waves, groups, apparent chaos and two opposite halves at 30 s,
and at exactly 60 s every one has done a whole number of swings: the video is one cycle of the
physics, so the last frame flows into frame 0 (a true seamless loop).

Each lantern-bob plucks its own note (3 octaves of C major pentatonic, longest = lowest) and flashes
every time it is back at its release side. The groups ARE the rhythm (2 groups = eighths, 3 = triplets,
4 = sixteenths ...). From 35 s an original bass + pad change chord with each pattern and a soft groove
locks to the middle pendulum (n = 58 -> 58 BPM, one pluck per beat).

Build order:

```bash
python timeline.py                                          # physics checks: lengths, group milestones, loop error
python audio.py                                             # -> audio.wav (+ section levels and pluck-band margin)
python focus_check.py                                       # every frame: worst blur on any bob must be 0
python -m reelkit.sheet render.py test/sheet.png 0.6 6.5 10.2 15 20 25 30.5 40 45 50.3 53 59.9
python -m reelkit.video render.py audio.wav perfect_loop.mp4
```

| file | what |
|---|---|
| `timeline.py` | the physics (exact analytic motion), every pluck/flash time, group milestones, chords, captions, HUD times |
| `scene.py` | the attic: gable wall with the round sunset window (key light), sloped ceiling + rafters, workbench, clocks, lamps, cat, the rig and the lantern-bob builders |
| `render.py` | per-pixel attic shell (sun pool + the bobs' moving shadows + coloured bob light), objects, the 15 bob slots, locked DOF, sunbeam + motes, tie-lines, tags, phase-ring HUD |
| `audio.py` | lantern plucks from the trigger times, bass/pad/groove, build, mix + band check |
| `focus_check.py` | focus audit without rendering |

Facts checked (calc): T = 2*pi*sqrt(L/g) with T = 60/n gives L = 34.4 cm (n = 51) to 21.2 cm (n = 65)
for ideal simple pendulums. Groups at t = 60/g s: 6 @10, 5 @12, 4 @15, 3 @20, 2 @30, then 3 @40,
4 @45, 5 @48, 6 @50, one line @60 (verified by `timeline.py`). Release amplitude 6.2 cm for every bob
(10-17 degrees). Source: Harvard Natural Sciences Lecture Demonstrations, "Pendulum Waves".
