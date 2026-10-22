# The Metronome Choir (IDEAS #16, "100 Metronomes, part 2")

The sequel to #1, built on `reelkit`: 199 metronomes on 5 tiered risers (bass, tenor, alto, soprano, high
bells; 4 x 10 each, one empty seat on the bass riser) plus a golden soloist on a podium, in a low-poly concert
hall lit by a big round skylight. Same physics as #1, twice: each riser locks when it goes onto its own 2 soda
cans, then the whole loft goes onto one platform on giant cans and the 5 in-sync-but-out-of-step sections pull
together; the soloist (15 % fast, no riser) is dragged in last = the drop at 38 s. LED tip show, original song,
then the giant cans get kicked out and the thump throws everyone back to the opening chaos (seamless loop).

Build order:

```bash
python sim.py          # -> sim.npz (two-layer Kuramoto, calibrated: section flams, ticks on the grid, soloist, loop reset)
python audio.py        # -> audio.wav + stems/ (prints section levels)
python audio_check.py  # loudness, 2 s windows, hits/s, bells vs music, loop seam
python focus_check.py  # every metronome on screen must have CoC 0 on all 1800 frames
python flash_check.py --times 2.3 9.6   # photosensitivity on a window (or --mp4 metronome_choir.mp4 for the deliverable)
python sheet_par.py test/sheet.png 1.5 7.3 11 23.2 26.5 32.5 36.6 39.2 45.2 49.2 55 58.15
python -m reelkit.video render.py audio.wav metronome_choir.mp4
```

| file | what |
|---|---|
| `timeline.py` | single source of times: lifts, platform, drop, kick-out/jolt, captions, song chords, the soloist's melody carousel |
| `layout.py` | who sits where: 5 tiers, boards, cans, giant cans, podium; metronome indices per section |
| `sim.py` | two-layer coupled-metronome model + calibration (see its docstring); writes every tick |
| `motion.py` | lifts onto cans, hoist, sway, kick-out + thump, pendulum angles, in-sync bookkeeping (shared by picture + sound) |
| `scene.py` | the hall (skylight, organ, wings, balconies), #1's metronome model, steps/boards/cans, podium, piano + cat, timpani, seats + pawns, truss |
| `render.py` | ray-cast hall shell, per-pixel floor/platform/carpet/board tops with every metronome's and rod's shadow, metronomes (phase tips, LED panels in the drop), locked focus, sunbeam, HUD, captions |
| `flash_check.py` | Harding-style general-flash test (adapted from Fireflies); camera moves over the striped loft are the risk |
| `audio.py` | bells from every tick (impulse trains), soloist carousel, motion fx, pulse + riser, the drop arrangement, loop-aware master |

How the science drives the sound: every tick is a bell at its exact simulated time. Each section rings a two-note
pair (left end / right end of the swing): left ticks stack into D major (D3 F#3 A3 D4 F#4), right ticks into
Asus2 (A2 E3 B3 E4 A4). Unsynced risers = a short, quiet bell-tick wash (~400 ticks/s); a locked riser's 40
bells land together, so its line rises out of the wash. The soloist's carousel steps one melody note per tick of
its own, so the melody drags against the beat until it locks; slot 0 rings on the drop. After the global
rotation in `sim.py`, left ticks fall on whole seconds, so the song (120 BPM, D major, I-vi-IV-V) is locked to
the ticks.

Facts / flags: the physics is real (metronomes on a free board sync; boards on a shared platform pull each other
in), the mechanisms are invented (two-note bells, the melody carousel) and never captioned as science. The jolt
reset is staged (a comic beat). Lock times from `sim.py`: bass 9.6 s, tenor 15.9 s, alto + soprano 20.0 s, high
bells 23.5 s, all 199 with the choir by 33.1 s, soloist for good at 37.8 s (its drop tick 1-3 ms from the grid).
