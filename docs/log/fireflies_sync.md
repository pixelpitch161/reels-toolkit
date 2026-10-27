# Log: `projects/fireflies_sync`

Append new entries at the end (newest last).

## Fireflies That Sync (`projects/fireflies_sync`)
- *Photinus carolinus* at Elkmont: moonlit low-poly cove (tulip poplars, creek, ferns, red-lit trail, owl, layered
  ridges). One flash schedule (`sim.py`) drives glows, flash light on the litter, the HUD raster and the bells: ~60
  random flashers (sparkle, ~60 notes/s), a Kuramoto lock-in in the beat-grid frame (spread 274 -> 8 ms over burst A =
  smear -> flam -> unison), burst 2 as a lit band moving at 0.5 m/s (each fly flashes while the front is within
  0.69 m = 5 flashes; pitch follows x so the chord climbs and pans), burst 3 as two fronts meeting + a 2500-fly
  valley. Females answer with kalimba notes in the gaps. 109 BPM from the 0.55 s flash spacing; video = 109 beats.
- Lessons:
  - A 100-150 ms flash is only ~2 bright frames at 30 fps: random sheets miss it. Check sheets at flash-aligned times
    (`T.grid(b)`), and draw glows with a perceptual curve (I^0.55) so flashes linger without changing their timing.
  - Big zooms (7 m -> 9 cm macro): interpolate distance in log space AND slide the look-at target in proportion to
    the distance, or the subject passes behind the lens mid-push.
  - Populations scattered "around the clearing" mostly landed off-frame; place them in each shot's view wedge.
  - Thousands of far flashes as halos merged into a glowing hedge on the horizon; below ~0.1 px size draw a 1 px core.
  - DOF: `sharp = max(need, min(0.28, 1/zf))` (template-style) keeps wides crisp and still gives macro bokeh;
    flashes with CoC > 1.6 px become octagonal bokeh discs (beautiful behind the macro hero).
  - Photosensitivity: splatted ground light from a synchronous valley flash crossed 25 % area (28 %); lowering the
    light map gain and bloom brought it to 22 % (0 general flashes on the mp4).
  - Audio: cap the summed gain per 12 ms pitch bin (thousands of bells otherwise set the peak and push the opening to
    -25 dBFS); run the compressor envelope around the loop seam; fold the >60 s tail onto the start.
- Feedback: (pending)
