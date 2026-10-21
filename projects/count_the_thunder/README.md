# Count the Thunder (IDEAS #13)

Light arrives essentially instantly; sound travels ~0.343 km/s. Count from the flash to the thunder:
seconds ÷ 5 = miles (NWS rule), seconds ÷ 3 = km (Environment Canada rule). A storm crosses a night
prairie toward a cut-away log cabin; the cat watches from the windowsill (indoors the whole time), and
the video ends on "TRY IT: COUNT FROM INSIDE".

How the science drives the music (all from `timeline.STRIKES`):
- **First half (real time):** a pitched tick for every counted second (the count climbs a D-minor
  pentatonic) and each thunder lands at flash + delay: 10 s, 7.5 s, 5 s, 3 s. Every strike also sends a
  pale sound ring crawling across the prairie that reaches the cabin exactly when the thunder does.
- **Back half (120 BPM):** one flash per bar on the downbeat (bright snare); the kick IS the thunder, late
  by the real flash-to-bang time (2 s = a whole bar, 1, ½, ¼, ⅛ s) until it fuses with its flash on the
  overhead strike at 50 s (lightning rod on the barn). The HUD bar shows the kick walking back.
- **Timbre:** thunder low-pass fc = 180 Hz + 6 kHz·e^(−km/0.5): spectral centroid rises from ~130 Hz
  (3.4 km rumble) to ~4.9 kHz (overhead clap). Rain is the shaker.

Build order:

```bash
python audio.py                  # -> audio.wav + stems/ ; prints section levels
python audio_check.py            # thunder vs music (40-400 Hz), ticks vs music (1.5-4 kHz), phone band
python focus_check.py            # subject CoC = 0 on all 1800 frames (no rendering)
python -m reelkit.sheet render.py test/sheet.png 0.55 7.2 19.6 31.2 38.05 46 50 57
python -m reelkit.video render.py audio.wav count_the_thunder.mp4
python flash_check.py --mp4 count_the_thunder.mp4     # photosensitivity (general-flash test)
```

| file | what |
|---|---|
| `timeline.py` | single source of times: strikes (flash, delay), rule conversions, captions, HUD windows, storm/rain/wind/moon curves, cat schedule |
| `scene.py` | cabin cut-away (window, fireplace on the back wall, lamp table, shelf, rocking chair), barn + rod, windmill, fence + distance markers, storm-cell blobs, the distance mapping, one bolt per strike |
| `render.py` | sky, per-pixel prairie (moon + per-strike lightning ground masks, sound rings), cells, ray-traced cabin shell (window pool + flash window shadows on the floor), cat, fire, locked DOF, depth-tested bolts, window beams, HUD |
| `audio.py` | count ticks, distance-filtered thunder (crack / boom / rolling rumble / phone layer / kick body), groove, rain, wind |
| `audio_check.py`, `focus_check.py`, `flash_check.py` | the verification passes |

Safety and photosensitivity: nobody is ever outdoors (the cat stays inside); one luminance pulse per
strike with no restrike flicker, ≤ 0.5 flashes/s; the flash lights sky, window, floor patch and prairie
while the warm lamps keep the room lit (no full-frame white). Distances on screen use the rules, which
are approximations (~4.7 s/mile, ~2.9 s/km at 343 m/s); no single "max hearing distance" is shown.

Sources: [NWS: science of thunder](https://www.weather.gov/safety/lightning-science-thunder),
[NWS: lightning safety](https://www.weather.gov/safety/lightning-safety-overview),
[ECCC: how thunder sounds](https://www.canada.ca/en/environment-climate-change/services/lightning/science/how-thunder-sounds.html).
