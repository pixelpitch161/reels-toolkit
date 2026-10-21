# The Lowest Note in the Universe (IDEAS #2)

The black hole in the Perseus cluster blows bubbles in hot gas; each bubble launches a pressure wave,
and the waves make a B-flat about 57 octaves below B-flat4 (one crest every ~9.8 million years).
An octave counter climbs from -57: every crest becomes a kick at -9 (55 BPM), doubles to 109 and
219 BPM, blurs into a flutter, and at -4 (29.1 Hz) is heard as a pitch, B-flat0. B-flat1..4 stack
under an original B-flat-major melody at 109.3 BPM (the hole's note 8 octaves down). Ends on
"HUM A B-FLAT / YOU'RE 57 OCTAVES UP" and loops to the opening frame.

## Build

```bash
python audio.py                                   # -> audio.wav (+ section and band level checks)
python -m reelkit.sheet render.py test/sheet.png 1 7 12.7 21.3 30.15 36 41 47.6
python -m reelkit.video render.py audio.wav lowest_note.mp4
```

## Files

| file | what |
|---|---|
| `timeline.py` | every event time: octave counter steps, wave-crest launches (the same list drives rings on screen and kicks in the mix), ring travel, captions, melody |
| `scene.py` | floating-island diorama, gas basin, rim props (observatory + lamp, dishes, crystals, rocks), backdrop, builders for crests, jets, galaxies, satellite |
| `render.py` | per-pixel rim floor (starlight, core light, lamp, each with shadow masks) and gas basin (haze, crests and their shadows, octave colour bands), painter-sorted faces, DOF locked on the black hole, octave HUD |
| `audio.py` | crest kicks tuned to B-flat1 with a B-flat4 "thock" (audible on phones), band-limited pulse stack B-flat0..4, original melody |

Glyphs `♭`, `≈`, `→` are added to `reelkit.font.GLYPHS` at runtime in `render.py`; `reelkit/` itself is unchanged.

## Science notes

Sources: [NASA Imagine (2003)](https://imagine.gsfc.nasa.gov/news/12sep03.html),
[Chandra Chronicle](https://chandra.harvard.edu/chronicle/0303/perseus/),
[NASA sonifications (2022)](https://science.nasa.gov/learners/highlights/data-sonifications).
Wording rule: the note is inferred from X-ray ripples, never "recorded"; we say "humming".
