# Log: `projects/bounce_royale`

Append new entries at the end (newest last).

## Bounce Royale (`projects/bounce_royale`)
- Battle-family reel #3, mood A (dark synthwave): four balls (RUBY drums, JADE bass, COBALT arp, AMBER lead) fight
  over a 48-tile wall of a holographic ring above a rain-soaked neon rooftop (magenta moon, stairwell door lamp, a
  neon sign, projector under the ring, city towers with lit windows, banners, beacons and drones). Every wall bounce
  splashes paint and plays its colour's instrument (pitch from how high on the ring, pan from the side, volume =
  share, repaint = accent, pop = cut-out). Ring shrinks from 16 s; the winner floods the ring and solos (its bounces
  step through a motif); the ring closes to a point that frame 0 snaps open.
- Engine `royale.py` (pure-Python billiards, ~40 ms per run) + `seed_search.py` (story ranking, early stop).
  Seed 25672 of ~12,700: RUBY leads early, JADE takes over 13.5 s, COBALT and AMBER both at 1 segment, COBALT pops
  24.85 while AMBER claws back 1 -> 21, RUBY hangs on 1-2 segments for 12 s (pops 39.63), JADE vs AMBER swings four
  times (24-24 at 45 s), JADE wins 47.58. Captions are placed by the run's events (a story beat every 2-4 s).
- Rule changes vs the backlog, and why:
  - One tile per bounce never eliminates anyone: hits spread evenly round a circle, so each colour sits at 12 +- 3
    of 48 (Ehrenfest-like restoring force; 0 pops in 24 runs). Splash width proportional to share makes each
    repaint a fair coin for both colours (neutral drift, like the voter model), so pops happen and comebacks too.
    Exponent 1.5 snowballed (all pops by ~20 s); ring-scaled splashes cascaded the pops together.
  - Equal masses with equal speeds don't keep equal speeds after an elastic collision: keep the elastic directions,
    reset both to the shared speed (the "same speed for every ball" flag).
  - A ball can end up gliding along the wall (in a stepped sim: thousands of bounces/s, which also let the literal
    rule pop colours by sweeping): a 12 deg bevel on grazing bounces fixes it.
  - Launch one ball per beat so the instruments enter in turn (simultaneous launches all hit the wall at 0.37 s).
  - Fit needed "close duel" + "near-death before pop 1" in the search, or the top seeds had one-sided duels.
  - Winner choice: drums won #20 and #21, so a seed with a non-drum winner and an equally strong story was searched
    for (second batch); JADE (bass) wins.
- Lessons:
  - Loop seam lighting: a collapsing ring's lights fading to 0, then the snap-open at full strength jumped the roof
    lighting (seam 3.2x). Let the closing point charge up to the same light (blend both ends to one lilac) -> 1.0x.
  - Audio build: section gains before the compressor got flattened (+1.3 dB); apply them after the compressor and
    lower the target so the limiter doesn't cap the loud half (+3.1 dB).
  - With fov 50 portrait the horizontal view is only +-15 deg: props near the camera must sit within |x| < 1.4 m.
  - City windows as 1 px points disappear under DOF; 2x2 px dots and a smaller aperture (K 3.8) keep the skyline lit.
- Feedback: (pending)
