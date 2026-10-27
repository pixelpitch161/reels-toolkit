# Log: `projects/ballfight_trumpet_vs_sax`

Append new entries at the end (newest last).

## BALLFIGHT #3: Trumpet vs Sax (`projects/ballfight_trumpet_vs_sax`)
- Third BALLFIGHT episode (#29), produced in parallel with #30-#35 on the shared engine. TRUMPET (yellow) fights with a
  new weapon kind `beam`: the bell charges for a beat with the aim locked on the sax's predicted spot (`lead` 0.7, a
  dashed sight on screen), then a straight blast that hits once, pierces smoke rings, and grows longer + wider per landed
  blast; the trumpet sprite rolls inside its ball to point along the aim (side-on, valves up). SAX (teal) blows smoke
  rings (volley + new `curve` option: homing at 110 deg/s, 2.6 s life, own hit radius). Abilities: FANFARE / DOUBLE
  TONGUE / HIGH C; SMOKE SCREEN / BEBOP RUN / HONK (a giant sax bell hovers over the predicted spot, then blasts).
- Honest run, seed 337 (18 fits in 899; TRUMPET 43 % / SAX 57 % over 893 decided): Trumpet up 16 at 15 s, the rings
  snowball back, lead changes at 40.6 / 44.4 / 45.9 / 49.3 s, HIGH C 42.6 + HONK 43.5, TRUMPET wins at 49.34 s, 5.4 HP.
- Engine lessons: a beam's aim locked at charge start with linear prediction lands ~2/3 of shots (walls, clashes and
  knockbacks make it miss); keep beam state opt-in (no RNG draws, no new snap keys: charges + blasts are events, the bell
  angle rides in `TH`). Parallel sessions all appended to fighters.py: resolve rebase conflicts by keeping every entry,
  then re-run tools/ballfight_replay_hash.py (and compare your own chosen seed with its cached run) before pushing.
- Rules lessons: balance is touchy on beam damage (4.0 -> 4.4 = 28 % -> 68 % trumpet wins at scale 0.5). Lower the
  weapons' damage rather than the global scale, so desperation moves keep their punch: lead changes 0.55 -> 0.8 per run.
- Look lessons: a smoke ring standing across its flight is edge-on from this camera (a sliver): draw it as a donut turned
  three-quarters to the camera. An effect hovering above the ring (the honk bell) leaves the sharp band above ~2.9 units
  near the front corner: keep it low (focus audit caught a 1.2 px CoC).
- Loop lessons: corner-light beat pulses light the whole floor, so a beat whose 70 ms ramp falls inside one frame is a
  whole-frame step; the mp4 seam measured 2.7x (raw 2.4x). Ramping only the frame-0 downbeat over 0.14 s (4 frames)
  fixed it with a 4-frame patch render: seam 1.9x.
- Sound: horns as call and response on the engine's own grid (volleys on beats 1 + 3, blasts on 2 + 4); horn buses
  needed +6.4 dB (trumpet) / +3.2 dB (sax) to sit 3-12 dB over the band in 300-5000 Hz.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 1.9x); flash
  0 general; opening -17.5 dBFS, build +3.5 dB. Render ~0.19 s/frame/core (~6 min) + 3.5 min mux.
- Feedback v1 (Oct 24): the trumpet model was inaccurate (made before the series used Chrome references), and the
  laser had a hard cutoff at the ball. v2: the Pixel Pitch modern trumpet rebuilt from a Yamaha YTR-8335LA side view +
  top view (the old one had the leadpipe on top and a small low bell; a real trumpet has the bell pipe on top, the
  leadpipe in front of the valves, the main slide looping down at the bell end, the bell bow at the mouthpiece end).
  Lesson: a light effect should start at its source inside the subject (the bell, inside the glass) with blurred
  halos, not at a hard polygon edge on the ball's rim. Full re-render (the model is in every frame).
- v3 (BALLFIGHT v2 redesign, Oct 24): new signatures, the instrument's body as the weapon. TRUMPET VALVE COMBO (engine
  kind `combo`: three pistons punch 1-2-3 on the 8ths, aimed at the enemy, out / hold / back, the third harder; reach
  grows) and SAX STRAP FLAIL (kind `flail`: the head spins up toward `spin_max`, damage grows with momentum, a landed hit
  wraps the enemy, reels it in to 2.3 until a beat two on, then yanks); HIGH C became an ability kind `beam` (v2's blast as
  the finisher). Three small engine commits + a roster commit, each checked with tools/ballfight_replay_hash.py (all nine
  episodes; #29's own entry moved to the new seed).
- v3 rules lessons: reeling the trumpet to 1.6 put it inside its own piston reach and it won 100 % by 27 s; reeling to
  2.3 (just outside the starting reach) makes the growing reach the trumpet's way back in. Balance flips fast on the
  flail's damage (7 -> 8 = 83 % -> 22 %). Close fights level both fighters together, so unlocks pair up and ~1 % of runs
  fit the hard windows; the search still took a minute. Seed 24 had a later lead change but a 4.3 s hitless opening,
  so seed 191 (longest gap 2.6 s) won.
- v3 look lessons: pistons need ~5 px heads to read at ball size; smoke trails need short lives or they carpet the
  ring; a wide beam's halo must not scale with its width (cap it) or the finisher blinds the frame; the giant beam's
  visible length ends at the ropes to stay in the sharp band. Flash test on the busiest stretch: 16.6 % peak.
- Feedback v3: (pending)
