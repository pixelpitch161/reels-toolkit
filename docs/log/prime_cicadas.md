# Log: `projects/prime_cicadas`

Append new entries at the end (newest last).

## Prime-Number Cicadas (`projects/prime_cicadas`)
- Brood XIX (13-yr) + Brood XIII (17-yr), together in 2024 for the first time since 1803 (next 2245). Golden-hour
  Illinois woodland cut away like an ant farm: soil face with strata, roots, tunnels and nymph chambers in front,
  XIX trees left / XIII right of a split-rail fence, lantern hanging over the cut, farmhouse porch lamp, cedars.
- Music = calendar: 1 year = 1 sixteenth at 120 BPM, so 1803 -> 2024 = 221 sixteenths and 2024 lands on a downbeat
  (51 s). XIX calls every 13 sixteenths, XIII every 17 (synth "Pharaoh" calls, C#6 vs E6) over an original A-major
  groove; each hit also plucks a chord tone. Ghost 12/18 broods at 36 yr/s = a 3:2 that meets every second.
  One emergence list drives bursts, crown flashes, HUD lane ticks and the calls.
- Lessons:
  - Low cameras in front of a cut sit below ground: test "ray reaches z = 0 below y = 0" for the face, and restore
    face pixels that objects behind the cut painted over (depth > face depth).
  - With a 6.5 degree sun, near crowns shadow the void in front of the cut; field stripes need low casters further
    back (cedars, tree line: only parts lower than distance-to-cut / 8.8 land on the field).
  - Glows need an occlusion test against the depth buffer (a porch lamp and fireflies leaked through the soil).
  - An orange crown flash over green leaves reads yellow-green: use a deeper red-orange (255, 104, 24) for XIX.
  - Calls sit 30+ dB over the music in 980-1450 Hz after carving 880-1750 Hz out of the music + drum buses.
  - Focus audit: subject CoC = 0 on all 1800 frames; frame 0 vs last frame differ by 1.9/255.
- Feedback: (pending)
