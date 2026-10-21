# Prime-Number Cicadas (IDEAS #12)

Periodical cicadas spend 13 or 17 years underground, then emerge together. In 2024 Brood XIX (13-year)
and Brood XIII (17-year) emerged in the same year, the first time since 1803 for these two broods
(next: 2245); they only overlap in a thin strip of central Illinois. 13 and 17 are prime, so a 13/17
pair lines up only every 13 x 17 = 221 years, while 12 & 18 would meet every 36.

The music is the calendar: **1 year = 1 sixteenth at 120 BPM**, so 1803 -> 2024 is 221 sixteenths
(27.625 s). XIX calls every 13 sixteenths, XIII every 17: a real 13-against-17 polyrhythm over an
original A-major groove, landing together only on 2024 (the 51 s downbeat, the climax).

## Build

```bash
python audio.py                                  # -> audio.wav (+ section levels, call-band check)
python focus_check.py --loop                     # subject blur on all 1800 frames + loop diff, no rendering
python -m reelkit.sheet render.py test/sheet.png 1 5.2 11.1 17.4 29.8 51.4 52.6 59.97
python -m reelkit.video render.py audio.wav prime_cicadas.mp4
```

## Files

| file | what |
|---|---|
| `timeline.py` | single source of times: the year grid (`year_t`), XIX / XIII emergence lists (one list drives bursts, HUD ticks and calls), ghost 12/18 hits, nymph cycle state, captions, tags |
| `scene.py` | golden-hour Illinois woodland edge cut away like an ant farm: soil texture (strata, roots, tunnels, chambers + tunnel id/progress maps for the glow), trees with cicada perch spots, split-rail fence divider, lantern on a hook over the cut, farmhouse + porch lamp, barn + silo, cedars, far tree line, clouds |
| `render.py` | per-pixel field (sun shadows, lantern/porch pools, AO, emergence holes) and soil face, painter-sorted objects with cached box-occlusion shadows, half-tube nymph reliefs, cicada swarms (bezier take-off -> perch / orbit -> fall), ghost broods, birds, eggs, locked DOF, glows, HUD lanes |
| `audio.py` | synthesised "Pharaoh" calls (tone + tymbal AM + falling slur; XIX C#6, XIII E6), brood choruses, groove, pads, chord-tone plucks per hit, ghost 3:2 ticks, ambience |
| `focus_check.py` | worst CoC on every visible subject point, every frame (must be 0), plus frame 0 vs last frame |

## Notes

- The cut plane is z = 0; low cameras sit "in the pit" below ground level, so the face test is
  "the ray reaches z = 0 below y = 0", and anything behind the cut that painted over the face is
  restored afterwards (objects at z < 0 are hidden by the soil).
- With the sun at 6.5 degrees, crown shadows of nearby trees fall in front of the cut; the stripes on
  the visible field come from mid-field cedars and the near tree line (only parts lower than
  ~distance-to-cut / 8.8 can reach it).
- The sun disc only reads with an orange sky around it: narrow sky glow, small sprite, modest god rays.
- Captions follow the exact year grid: the near misses fall at 29.75 s (1854/1855) and 44.5 s (1972/1973),
  not where the backlog beat sheet guessed.
- Wording (board flags): "these 2 last met 1803" (never "first time two broods"), "why prime? still debated",
  predators only as a question, no "trillion" hype; XIX lower / XIII higher is a voicing choice grounded in
  M. tredecim singing slightly lower.

Sources: [Wikipedia: Brood XIII](https://en.wikipedia.org/wiki/Brood_XIII),
[Wikipedia: Brood XIX](https://en.wikipedia.org/wiki/Brood_XIX),
[UW-La Crosse](https://www.uwlax.edu/currents/cicadas-2024/),
[Toivonen & Fromhage 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC7319174),
[Wikipedia: Decim cicadas](https://en.wikipedia.org/wiki/Decim_periodical_cicadas).
