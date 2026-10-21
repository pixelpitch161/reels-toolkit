# Mozart's Dice Waltz (IDEAS #7)

Two dice compose a waltz from the 1792 Berlin table (Simrock; sold as Mozart's, authorship never
authenticated). Every roll's sum picks one bar; the bar lands on a parchment staff in a moonlit
Viennese salon and a self-playing fortepiano plays it. Back half: ten cups roll at once, their
waltzes stack on different timbres, and the sum histogram on the wall locks into the 2-12 triangle.

Build order:

```bash
python audio.py                                            # -> audio.wav (+ level and dice-band checks)
python -m reelkit.sheet render.py test/sheet.png 0.6 6.8 9.4 18 25 31 49
python -m reelkit.video render.py audio.wav dice_waltz.mp4
```

| file | what |
|---|---|
| `waltz_table.py` | the 1792 waltz table (11 x 16) and all 176 bars in a compact own format, with provenance + checks |
| `timeline.py` | single source of times: rolls, bars, captions, HUD, histogram events, loop |
| `scene.py` | salon geometry: window (moon key light), staff, chart, histogram, fortepiano, cat, dice, cups |
| `render.py` | per-pixel room shell (floor/walls/window backdrop), objects, overlays, locked DOF, moonbeam, HUD |
| `audio.py` | synth fortepiano / music box / pizzicato / flute / celesta / harp / clarinet, dice clacks |

Facts checked: 2 x 11^14 = 759,499,667,166,482 distinct waltzes (column 8 has 1 real bar,
column 16 has 2); 11^16 = 45,949,729,863,572,161 roll sequences; at 30 s per waltz, ~722 million
years to hear them all; P(7) = 6/36 vs P(2) = 1/36.

Note data: re-entered from three independent transcriptions with a 2-of-3 vote per bar
(details in `waltz_table.py`). Every column's 11 bars share one harmony, so the stacked climax
stays consonant (verified from the table, not assumed).
