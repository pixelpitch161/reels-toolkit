# The Perfect Loop (IDEAS #11) - v2: 100 pendulums

100 uncoupled pendulums in a clockmaker's long gallery at sunset. Pendulum k swings 51 + k times a
minute (51 → 150), so the strings run from 34.4 cm down to 4.0 cm. They drift through travelling waves,
group patterns, apparent chaos and two opposite halves, and every 60 s of physics each one has done a
whole number of swings, so they all line up again.

**Phase offset:** the video starts 8 s into the physics cycle (physics time = video time + 8 s). It opens
mid-pattern, the group ladder arrives every 2-5 s (6 @2 s, 5 @4, 4 @7, 3 @12, 2 @22, then 3 @32, 4 @37,
5 @40, 6 @42, 8 @44.5, 10 @46, 15 @48, 20 @49, 30 @50, 50 @50.8), and all 100 snap into one line at
52 s. Frame 1800 = physics 68 s = physics 8 s = frame 0: a true seamless loop.

Each lantern-bob plucks its own note (4 octaves of C major pentatonic, five neighbouring pendulums per
note, longest = lowest = red) and flashes every time it is back at its release side: 10 050 plucks,
~167 per second. Plucks are short and quiet, so alignment punches through by itself (the 2 teams are a
50-voice chord, the line-up a 100-voice strum), and the last plucks before 52 s fall in rolled sweeps
that tighten into the strum. The groove is 120 BPM = pendulum n = 120 (one pluck per beat); every
milestone lands on that grid and the hi-hat subdivision equals the current group count. Bass + pad enter
at 27 s and the mix builds ~7 dB into the 52 s drop.

v1 (15 pendulums, `perfect_loop.mp4`) is kept as `*_backup.py`; those files import each other, so
`python -m reelkit.sheet render_backup.py ...` and `python audio_backup.py` (writes `audio_v1.wav`) still rebuild v1.

Build order:

```bash
python timeline.py                                          # physics checks: lengths, group milestones, beat grid, loop error
python audio.py                                             # -> audio.wav (+ 2 s levels, strum peak, pluck-band margin)
python focus_check.py                                       # every frame: worst blur on any on-screen bob must be 0
python -m reelkit.sheet render.py test/sheet.png 0.6 2 6.6 12 19 22 32 37 46 50.5 52 56
python -m reelkit.video render.py audio.wav perfect_loop_v2.mp4
```

| file | what |
|---|---|
| `timeline.py` | the physics (exact analytic motion, 8 s phase offset), every pluck/flash time, group milestones + the racing counter, chords, hat subdivisions, captions, HUD times |
| `scene.py` | the long gallery: window wall with four round windows (key light), gable end with clocks, sloped ceiling + rafters, workbench, lamps, cat, the 2.5 m rig and the lantern-bob builders |
| `render.py` | per-pixel gallery shell (four sun pools + the bobs' moving shadows + coloured bob light), objects, the 100 bob slots, light trails, locked DOF, sunbeams + motes, group tie-lines, tags, phase-ring HUD with group counter |
| `audio.py` | lantern plucks from the trigger times, arrangement locked to the groups, build, lookahead limiter, mix + band check |
| `focus_check.py` | focus audit without rendering |

Facts checked (calc): T = 2π√(L/g) with T = 60/n gives L = 34.4 cm (n = 51) to 4.0 cm (n = 150) for
ideal simple pendulums. At physics time 60 m/g s the pendulums form exactly g groups (verified by
`timeline.py`). Release offset 8.5 cm (long end) to 2.0 cm (short end, capped at 30°), exaggerated for
readability; the motion is exact analytic θ = θ₀ cos(2π n t / 60), so the loop is perfect. A real
100-pendulum rig would need sub-millimetre tuning and drifts with large swings, so the "try it" stays the
15-washer version. Source: Harvard Natural Sciences Lecture Demonstrations, "Pendulum Waves".
