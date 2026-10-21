# Pixel Pitch: instrument history series

60-second vertical (1080×1920) pixel-art reels. Each one tells the history of one instrument on a spinning
low-poly pedestal, playing a riff in each era's style. This is the second series in this repo. The science
reels live in `projects/` and `templates/` at the repo root; both series share `reelkit/`.

Everything here is flat on purpose: video scripts import each other as libraries (for example, the tuba
borrows drum sounds from `drums60_lowpoly.py`). Keep every file in this one folder.

## Files

| File | What it is |
|---|---|
| `pp_framework.py` | The engine: timeline, scene leveling, mastering (−11 K-weighted), frame layout, captions, progress dots, `run()` |
| `pp_light.py` | **Shared lighting** (from `reelkit`): key light with real shadow-map shadows, background-tinted ambient, rim light, brass shine, contact AO, glow |
| `pp_parallel.py` | Renders a video on all CPU cores: `python3 pp_parallel.py tuba60_lowpoly.py tuba.mp4` |
| `piano_lowpoly.py` | Original engine core (Geo, Cam, render_geo, fonts, pedestal, synths) and the piano video |
| `violin_lowpoly.py`, `cello_lowpoly.py` | Older self-contained videos, also used as libraries (bowed synth, cello helpers) |
| `pp_guitar.py`, `pp_shapes.py` | Model helpers (guitar bodies and stands; drums, cymbals, rods, bells, tripods) |
| `*_lowpoly.py` | One file per video (see [STATUS.md](STATUS.md)) |

## Commands

```bash
pip install -e ../..                                 # once: installs reelkit (pp_light needs it)
python3 tuba60_lowpoly.py --still 5.0 10.0            # check frames -> tu_5.0.png, tu_10.0.png
python3 pp_parallel.py tuba60_lowpoly.py tuba60.mp4   # full render, all cores
python3 tuba60_lowpoly.py tuba60.mp4                  # full render, single process
```

## Optional caption hooks

- `INTRO_STEPS = ((0, lines, colours), (.42, lines, colours), (.75, lines, colours))` replaces the two-step showcase caption
  (hook, then "BUT HOW DID WE / GET HERE?") with any number of steps, each starting at a fraction of the showcase. The ukulele
  uses it to show the modern instrument's own fact between the hook and the question.
- `OUTRO_NAME = "..."` shows a different instrument name during the outro (the ukulele returns to the modern soprano there).

## Lighting

Add one line to a video script:

```python
LIGHTING = True                         # shared lighting with defaults
LIGHTING = dict(key=0.7, rim=0.4)       # ...or override any value in pp_light.DEFAULTS
```

The background, pedestal palette, beat and captions stay exactly as the video defines them. The light
colours come from each video's own background: ambient from its average colour, and rim, key tint and
brass reflection from its brightest parts (sun, floodlights, neon). Each video therefore lights
differently with no extra setup. Leave `LIGHTING` out to get the original flat look.
