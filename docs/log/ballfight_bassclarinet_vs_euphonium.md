# Log: `projects/ballfight_bassclarinet_vs_euphonium`

Append new entries at the end (newest last).

## BALLFIGHT #12 (#38): Bass Clarinet vs Euphonium, two new fighters on the levels rule
- Reference photos in the user's Chrome (Claude in Chrome worked this time): Commons category pages read as text for
  the file lists + sizes, then each candidate drawn into a canvas with a 50 px grid (same-origin via `crossOrigin`) and
  screenshotted in zoomed slices to measure. Bass clarinet: Yamaha YCL-622II front shot (no official dimensions, so
  scaled to a 40 mm body), checked against a Buffet Prestige 3/4 view. Euphonium: Yamaha YEP-321 front shot, scaled with
  Yamaha's published 280 mm bell / 664 mm height (the two agree within 4 %). A bass clarinet is 26:1 long:thin, so the
  `ball` scene fattens radii x2 (neck + mouthpiece x2.4, the bell rim x chunk^0.75 so it doesn't balloon).
- Engine (5 small commits + a per-arm cooldown fix, all 11 finished episodes bit-identical each time): `anchor` (the
  whirl drives the ball's position directly and `move()` skips it; the clash still pushes the enemy out), `mortar` +
  `barrage` (shells with flight times; `predict()` follows an anchored enemy round its circle), `twin`, `copy` (new
  optional `trigger(k, slot, ab=, quiet=)` args + `Ball.last_ab`; a copied snip steps through `extra_snip`),
  `lights_out` (a `shadow` attribute read by `damage()`, `_stunned()` and the clash test; absent = as before).
  The #39 session pushed six engine kinds in parallel: no conflicts (all additive), my seed replays the same after the
  rebase.
- Tuning: first pass the anchor won 95 % (1.5 turns hit twice per whirl); a 0.8 s rest per arm (the MULTIPHONICS ghost
  arm has its own) + exact-lead shells fixed it. Lead changes stay rare (~0.7 a run: both weapons snowball), so fits
  are 0.4 % (8 in 2,171). SLAP TONGUE reuses #37's `snip` kind (no new kind needed).
- Title card: BASS CLARINET is the longest name yet (one line only fits at scale 4), so `title.py` got opt-in
  `TITLE_LINES` (BASS / CLARINET at scale 6) + `TITLE_BALLS` (the left ball moved down); every older card hashes the same.
- Picture: the endpin only reaches 0.35-0.7 beyond the glass at the base orbit, so it reads as a bright silver rod over
  the ball's edge (drawn after the balls), not a black arm under them. Shell landings that miss only kick dust: scorches
  for every shell turned the canvas into a carpet of splats. LIGHTS OUT = an eased 40 % dim with keep circles on both
  balls (flash test 0) + the ball sinking into a dark pool. IDENTITY CRISIS shows an amber bass clarinet sprite in the
  euphonium's ball for a bar.
- Music: the bass clarinet (140-600 Hz) first sat only -2..+2 dB over the drumline: tenors retuned up (294-494 Hz,
  hp 260), softer bass-drum beaters and timpani overtones, a held growl between whirls (it was silent half of every
  bar), +7..+9 dB after. Post-K.O. bars were 3 dB under the fight (the fighters carry the loudness, not the cadence):
  both fighters play their lines while revving in the corners, levels spread 3.1 dB. The seam needed a louder roll +
  riser (first 20 ms -8.7 vs last -13.1 dBFS). #37's lesson checked: no 0.5 s silent windows in either fighter's stem
  while it's fighting (a thinned note on the 2-beat salvo grid is now clipped, not dropped; the bass clarinet growls
  from FIGHT! to its first plant).
- Checks: levels PASS (wav + mp4); band check +7..+12 dB; safe zone OK; CoC 0; loopcheck PASS render 0.3x / mp4 0.9x;
  flashcheck PASS (the doors). Render 7.7 min + mux.
- Feedback: (pending)
