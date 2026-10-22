# The Lowest Note in the Universe (IDEAS #2)

The black hole in the Perseus cluster blows bubbles in hot gas; each bubble launches a pressure wave,
and the waves make a B-flat about 57 octaves below B-flat4 (one crest every ~9.8 million years).

**v2 (current):** opens on the climax (every octave, rings blasting, camera shake), tape-stops to real
speed (frozen, muffled groove + gas crackle), then speeds the black hole back up in beat-synced slams
(year, day, hour, minute, with a calendar/clock widget) to the drop at 11.7 s: one crest per second is
the kick (55 BPM), doubling to 109 and 219 BPM, a flutter, and at octave -4 (29.1 Hz) a note, B-flat0.
B-flat1..4 stack one per bar, the original melody lands on B-flat4, "HUM A B-FLAT", and the last bar
slams back into the cold open (the file loops). A log-frequency ruler shows the B-flat marker racing
57 octaves into the "YOUR HEARING" window.

The grid: one beat = the hole's note 8 octaves down (109.3 BPM). Every octave step from -9 to -4 falls
exactly on 2 beats / 1 beat / 8ths / 16ths / 32nds / 64ths, and 60.0 s is a bar line 22 bars after the drop.

v1 (approved look, too quiet for most of its length) is kept as `*_backup.py`.

## Build

```bash
python timeline.py                                # section times, loop checks
python audio.py                                   # -> audio.wav (+ section, phone-band and science-vs-music checks)
python focus_check.py                             # subject CoC on all 1800 frames (expect 0)
python -m reelkit.sheet render.py test/sheet.png 0.6 2.95 4.3 5.5 7.4 9.2 12.6 14.6 19.5 27 30.5 34.4 41 47 55.5 59.6
python -m reelkit.video render.py audio.wav lowest_note_v2.mp4
```

## Files

| file | what |
|---|---|
| `timeline.py` | the grid and every event time: octave counter, crest launches (the same list drives rings, ripples, shake, hops and the crest-kicks), scene clock (freezes at the tape-stop, one extra lap in the spin-up so orbits loop), ring travel, captions, melody |
| `scene.py` | floating-island diorama, gas basin, rim props (observatory + lamp, dishes, crystals, rocks), backdrop, builders for crests, jets, galaxies, satellite (unchanged from v1) |
| `render.py` | v1 pipeline + camera shake, screen-space crest ripple, prop/dust hops, frozen 'real speed' grade, calendar/clock widget, octave HUD with speed-up line and hearing-range ruler |
| `audio.py` | cold open + tape-stop dive, crackle bed, muffled groove with a filter that opens per slam, crest-kicks (B-flat4 'thock' + click for phones), B-flat0..4 stack, original melody, hum, loop-aware mastering |
| `focus_check.py` | focus audit without rendering |

Glyphs `♭`, `≈`, `→` are added to `reelkit.font.GLYPHS` at runtime in `render.py`; `reelkit/` itself is unchanged.

## Science notes

Sources: [NASA Imagine (2003)](https://imagine.gsfc.nasa.gov/news/12sep03.html),
[Chandra Chronicle](https://chandra.harvard.edu/chronicle/0303/perseus/),
[NASA sonifications (2022)](https://science.nasa.gov/learners/highlights/data-sonifications).
Wording rule: the note is inferred from X-ray ripples, never "recorded"; we say "humming".
Speed-ups are always labelled (HUD line: "REAL SPEED" / "SPED UP ...×"). 2^48 ≈ 281 trillion (octave -9) and
2^57 ≈ 144 quadrillion (B-flat4) are calc; 144 quadrillion is the low end of NASA's 2022 sonification range.
The crackle is texture, not data. Ripple, shake and hops are stylisation tied to the real crest times.
