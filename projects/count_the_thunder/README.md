# Count the Thunder (IDEAS #13) — v2

Light arrives essentially instantly; sound travels ~0.343 km/s. Count from the flash to the thunder:
seconds ÷ 5 = miles (NWS rule), seconds ÷ 3 = km (Environment Canada rule). A storm crosses a night
prairie toward a cut-away log cabin; the cat watches from the windowsill (indoors the whole time), and
the video says "TRY IT: COUNT FROM INSIDE".

**v2 arc (the "v2 rework" in `docs/IDEAS.md`): chaos → lock-in, straight from the physics.**
- **0–3 s cold open on the climax:** the overhead strike on the barn's lightning rod, flash + clap fused,
  full groove, downpour; the cat bolts off the sill.
- **3–5 s rewind:** "HOW FAR WAS THAT?" — the world runs backwards (camera pulls back to the horizon,
  rain falls up, windmill spins backwards, the cat hops back onto the sill) with a tape-rewind of the
  cold open, a scratch and a reverse cymbal; it lands on a far flash at 5.0 s.
- **5–30 s storm chaos:** many strikes at different distances (`timeline.CHAOS`, ~1 flash/s). Each
  flash sends a sound ring across the prairie and its rumble sounds exactly when the ring reaches the
  cabin, so rumbles land out of step with the flashes. The 5.0 s flash is tagged ("THIS ONE", amber
  ring) and counted on the beat: 15 s = 3 MI ≈ 5 KM. Delays shrink as the storm nears (5 s, then 3 s).
- **30–50 s lock-in:** one flash per bar on the downbeat (bright snare); the thunder IS the kick, late by
  2 → 1 → ½ → ¼ → ⅛ s, fusing at the overhead strike at 50 s. The far smear thins out underneath.
- **52–60 s loop:** a second cell races in: thunders 3 s, 2 s, 1 s late ("THE NEXT ONE'S CLOSER"), a
  riser, and its overhead strike is frame 0 — the cold open.

How the science drives the music (all from `timeline.STRIKES`):
- every thunder lands at flash + delay, with a distance low-pass fc = 180 Hz + 6 kHz·e^(−km/0.5):
  centroid ~130 Hz far (rumble) → ~4.9 kHz overhead (clap);
- count ticks are the hats: one pitched tick per counted second, climbing the D-minor pentatonic one
  octave per 5 s (= 1 mile);
- the music bed is low-passed while the storm is far and opens as it nears (fully open at 32 s).

v1 is kept beside it as `*_backup.py` (timeline, scene, render, audio and the three checks).

Build order:

```bash
python audio.py                  # -> audio.wav + stems/ ; prints section levels
python audio_check.py            # key thunders vs music (40-400 Hz), ticks vs music (1.5-4 kHz), phone band
python engagement_check.py audio.wav   # LUFS, 2 s window levels + hits/s (never quiet, >= 3 hits/s)
python focus_check.py            # subject CoC = 0 on all 1800 frames (no rendering)
python -m reelkit.sheet render.py test/sheet.png 0 1 3.6 5.6 9 14 19.6 24.6 33.1 38.05 50 55.3 57.2 59.95
python -m reelkit.video render.py audio.wav count_the_thunder_v2.mp4
python flash_check.py --mp4 count_the_thunder_v2.mp4     # photosensitivity (general-flash test)
```

| file | what |
|---|---|
| `timeline.py` | single source of times: the strike list (pre / chaos / count / lock / rod / cell2), rule conversions, captions, HUD windows, the rewind + phase clock, storm/rain/wind/moon curves, bed low-pass curve, bars, cat schedule |
| `scene.py` | cabin cut-away, barn + rod, windmill, fence + distance markers (1 KM … 3 MI), the distance mapping, strike positions, storm cells (main, two far flanks, the second cell), one bolt per strike |
| `render.py` | sky, per-pixel prairie (moon + lightning ground masks, sound rings with counted ones in amber), cells, ray-traced cabin shell, cat, fire, locked DOF, bolts, window beams, rewind overlay, HUD (count, lock-in grid, next-one panel) |
| `audio.py` | distance-filtered thunder, count ticks, groove with a time-varying bed low-pass, beat pump, tape rewind, riser, lookahead-limited master |
| `audio_check.py`, `engagement_check.py`, `focus_check.py`, `flash_check.py` | the verification passes |

Safety and photosensitivity: nobody is ever outdoors (the cat stays inside); one luminance pulse per
strike with no restrike flicker, ≤ 2 flashes/s in the chaos and far bolts small and dim; the warm lamps
keep the room lit (no full-frame white). Distances on screen use the rules, which are approximations
(~4.7 s/mile, ~2.9 s/km at 343 m/s); no single "max hearing distance" is shown. Strikes on downbeats
are stylised, but every delay matches the distance it is drawn at and captioned as.

Sources: [NWS: science of thunder](https://www.weather.gov/safety/lightning-science-thunder),
[NWS: lightning safety](https://www.weather.gov/safety/lightning-safety-overview),
[ECCC: how thunder sounds](https://www.canada.ca/en/environment-climate-change/services/lightning/science/how-thunder-sounds.html).
