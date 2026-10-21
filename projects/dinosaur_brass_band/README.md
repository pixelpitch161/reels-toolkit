# Dinosaur Brass Band (IDEAS #8)

Parasaurolophus's crest is a hollow air tube (CT-scanned 75-million-year-old skull, Sandia / NM Museum
simulation, 1997). Longer air column = lower note, so each animal's crest length sets its pitch: five
crest sizes = five registers, stacking into an original brass chorale in G with the big adult on the bass.

## Build

```bash
python audio.py                                            # -> audio.wav (+ level / band checks)
python -m reelkit.sheet render.py test/sheet.png 1.5 6 10 13.5 17.5 21.5 26 30 33 38 44 49 54 58.5
python -m reelkit.video render.py audio.wav dinosaur_brass_band.mp4
```

`python focus_check.py` checks all 1800 frames without rendering: the worst blur (CoC px) on any visible
dinosaur face must be 0 (the sharp band is sized from every vertex and face depth that touches the screen),
and no walking animal may be removed while still on screen. `FOCUS_LOG=test/focus.log` logs the same
number per frame during a real render.

## Files

| file | what |
|---|---|
| `timeline.py` | the single source of times: herd (sizes, crest air lengths, notes, colours, layout), honks, the chorale, walk paths, captions, pose curves |
| `dino.py` | low-poly Parasaurolophus from lofted rigid parts; neck / head pivots, leg swing; crest air passage; CT slice cards |
| `scene.py` | Late-Cretaceous floodplain: ground patches, river + sand banks, ferns (sway), cycads, tree ferns, araucarias, horsetails, flowering shrubs, rocks, log, far hills, forest, volcano, clouds, small-life data |
| `render.py` | per-frame renderer (sky, lit ground with herd shadows and river reflections, far layer, painter-sorted objects, X-ray / CT / air overlays, locked DOF, rings, god rays, HUD panels, captions) |
| `audio.py` | closed-tube honk synth (odd harmonics, crest pitch), timpani / pad bed, footsteps, scanner, test tones, bottle, ambience |

## Notes

- Layout uses a depth / lat frame (`scene.wpos`): the herd recedes toward the low sun so a band of five
  long animals fits a portrait frame; wide shots are auto-framed from the posed geometry (`render.fit`).
- Walk-ins arc in from behind-right and arrive along the heading (a straight walk along the heading went
  through the adult); walk-off curves away into the back-right, front animal first (walking straight on
  passed right beside the camera and blurred).
- Science wording follows the board flags: "could sound like", no Hz on screen, no "trombone", juveniles
  higher is "(PROBABLY)", a small "A MODEL, NOT A RECORDING" tag during the air-flow beat.
