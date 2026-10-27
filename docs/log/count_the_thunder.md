# Log: `projects/count_the_thunder`

Append new entries at the end (newest last).

## Count the Thunder (`projects/count_the_thunder`)
- Flash-to-bang counting: seconds ÷ 5 = miles (NWS), ÷ 3 = km (ECCC). Night prairie, cut-away log cabin, the
  cat watches from the sill (indoors throughout; ends on "TRY IT: COUNT FROM INSIDE"). One strike list drives
  flashes, bolts, a sound ring crawling across the prairie (reaches the cabin exactly at the thunder), the HUD,
  captions and the mix. First half: pitched count ticks + real-time thunder (10, 7.5, 5, 3 s). Back half at
  120 BPM: snare on each flash, the thunder is the kick, late by 2 s, 1, ½, ¼, ⅛ s, fusing at the barn-rod
  strike (50 s). Thunder low-pass by distance: centroid ~130 Hz (3.4 km) -> ~4.9 kHz (overhead).
- Lessons:
  - Portrait cut-away + sky is an angle budget: seeing the floor needs ~20°+ down, the storm needs the horizon
    in frame. fov 60 / pitch 21 fit horizon (y≈185), room (y≈265-527) and the HUD; a numeric probe of key points
    (floor corners, wall tops, rod, strikes) found it far faster than test renders.
  - Behind a cut-away wall the ground is hidden for ~8-10 units; storytelling on the prairie (ring, markers)
    needs a high wide shot (used as the 6-9 s establishing move).
  - Fireplace on a side wall is seen edge-on; moving it to the back wall made the fire (and the chimney above
    the wall) read. Black cat vs a dark window vanished; ginger reads.
  - A far storm must be squat to fit the sky band; lift it as it nears (cell_lift) so near bolts drop from the
    top of frame.
  - Flash envelope must peak ON the strike frame (a 1-frame ramp put the picture 33 ms behind the snare).
  - Smoke as opaque polys read as floating rocks at night; banded GLOW-kernel sprites blended after the lens work.
  - Photosensitivity (`flash_check.py`, Harding-style on the mp4): 0 general flashes; max 4 % of frame per strike.
  - Mix: high-passed "bright" snare (280 Hz) + deep bass ducking keep the thunder-kick 8-15 dB over the music.
- v1 feedback: not engaging enough, especially at the start (first 30 s ~1 hit/s at ~-25 dBFS, whole mix 6-9 dB under
  the other reels).
- v2 re-cut (`count_the_thunder_v2.mp4`, same scene/renderer, new timeline + audio): cold open on the climax (rod strike,
  fused clap, full groove), a 2 s rewind back to the horizon, then ~1 flash/s of storm chaos at 1-5 km: every ring
  reaches the cabin when its rumble sounds, so the rumbles land out of step. A tagged far flash is counted on the beat
  (15 s = 3 MI ≈ 5 KM), delays shrink (5 s, 3 s), then v1's lock-in, and a second cell (3 s, 2 s, 1 s) whose rod strike
  is frame 0. One strike list with kinds (pre / chaos / count / lock / rod / cell2) drives everything.
- Lessons:
  - v1 sat low because the big claps set the peak normalisation: a 3 ms lookahead limiter master (drive +5 dB) gets
    -12 LUFS (other reels ~-11) and lets section gains shape the build (chaos -17…-19 dBFS -> climax -12).
  - A 2 s-window "hits/s" probe (`engagement_check.py`: >= 6 dB energy jumps per band, within 20 dB of the loudest)
    matched the ~1 hit/s measured on v1. Sustained pads/rumbles mean the 250-2500 Hz band never registers a hit; a
    beat-synced sidechain pump on pads, rain and smear fixed that. Rain hiss masks hats: keep it down under a groove.
  - Count ticks starting at D4 vanished under the full groove (no energy in 1.5-4 kHz); start at D5, one octave per
    5 s (= 1 mile), and pull the flash snare back on downbeats that carry a tick.
  - Ring clutter: ~1 flash/s with 4-17 s delays keeps <= 10 rings on screen; chaos rings at 0.6 strength so the amber
    counted ring reads.
  - Rewind: a "phase" clock that runs 10 s backwards inside the 2 s rewind, so the whole video spans 48 s of phase (a
    multiple of the 3/4/6 s motion periods) and periodic motion still loops; windmill speed integrated and scaled to
    whole blade steps. A 2/4 bar (27-28 s) moves the downbeats from odd to even seconds after the rewind.
  - Loop: the second cell ends exactly where the main cell starts; seam diff 3.9 vs a normal 3.6 frame step (with the
    intended frame-0 flash removed). Focus CoC 0 on all 1800 frames; flash check 0 general flashes, <= 5.1 % per strike.
- Feedback: (pending, v2)
