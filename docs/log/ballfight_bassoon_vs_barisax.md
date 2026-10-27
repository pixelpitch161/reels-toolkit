# Log: `projects/ballfight_bassoon_vs_barisax`

Append new entries at the end (newest last).

## BALLFIGHT #13 (#39): Bassoon vs Bari Sax, the sneak vs the ring rattler
- Reference photos in the user's Chrome (Claude in Chrome): Commons' search API from a Commons page (same origin) for the
  file lists, then the candidates drawn into a canvas (`crossOrigin`) and measured per row (red-wood spans, silver /
  ivory bands). Bassoon: the Yamaha YFG-812 II front product shot, scaled to the usual 134 cm (0.815 mm/px): boot 425 mm
  (an oval 73 -> 63 mm), long joint to 1012 mm, bell 47 mm with a bulge and a 62 mm pale cap ring, wing joint 34 mm with
  its crown at 849-894 mm; "Bassoon parts.jpg" for the bocal's shape. Bari: the Amati baritone product shot, for what
  changes from the alto (long straight bell with the rim at ~80 % of the height, the pigtail loop, big mouthpiece,
  low-A key). At ball size a 1.34 m bassoon is a 3-px stick: the `ball` scene widens it x2.3 about its centre line and
  thickens the crook x3.4, so the hooked bocal is what reads (it vanished into dots at 16 px until then).
- The bari is a new file adapted from the alto (`altosax60_lowpoly` imported read-only for its swept tubes, cups,
  touches, palette); its voice reuses the alto's recipe, but the alto rolls harmonics off by number (`exp(-k/3)`), which
  at 65 Hz left a dull hum: the bari rolls them off by frequency through its own formants (~560 Hz / ~1.35 kHz).
- Engine (4 small commits, then a timing fix; all finished episodes bit-identical each time, 12 once #38 landed):
  `tiptoe` + `mountain`, `hook`, `rattle` + `offbeat`, `quake`. A hopper is a new kind of mover: `move()` skips it
  mid-hop (`air`) and moves it only by its knock boost on the ground (`hopper`), the clash skips airborne balls. The
  rattle reads new 'rope' events from the event list to find slings (no change to `move()`'s bounce).
- Timing bug: a hop landing a fraction of a substep after its 16th tick was missed at the tick, so the next launch
  slipped a 16th: MOUNTAIN KING's accelerating schedule came out as eight 3-16th hops then a ragged mix. Landing a hop
  due within one substep at the tick fixed it, and the on-time dance became decisive (82 % wins): re-tuned once
  (MOUNTAIN KING 1.6 -> 0.4 a stomp) and re-searched (the first pick, seed 668, no longer replays).
- Tuning: smooth damage streams (a stomp every beat landing ~95 %) gave ~0.2 lead changes a run. What helped: a
  small, growing stomp radius with an imperfect lead (about half the stomps land early, ~95 % late), backstabs x2.5, and a rattle
  that needs ~10 landed hits to reach all four ropes, so the bari's slings ramp up mid-fight while the bassoon ramps up
  later: lead changes ~0.7 a run, fits ~1-2 %, the search is cheap (275 runs for 8 fits). A random search over the
  grid (`tune_search.py`) was a waste here: the caption rule (>= 5 first uses 4 s apart) and the lead changes are
  structural, not numeric.
- Picture: the ropes ringing only read once they were tinted coral with a single standing-wave mode per side (three
  ropes in different modes looked like a glitch). Squash and stretch about the ball's bottom point; the hop's shadow
  stays on the canvas, which sells the height.
- Music: the honk sat under the bed in its own band until the downbeat kick stepped back under it (the honk IS the
  downbeat) and the incidental thuds were thinned; more ringing ropes = a fuller bass chord scaled by 1/sqrt(n), not a
  louder one (levels: 6.2 -> 3.6 dB spread). The bassoon's thinning only drops notes when its line is dense (a tumbling
  bassoon was going silent for 1.75 s).
- Checks: levels PASS (wav + mp4); band check +5..+17 dB; safe zone OK; CoC 0; flash 0 on the busiest stretch; loopcheck
  PASS render 0.4x / mp4 1.0x. Render 10 min + 3.5 min mux.
- Feedback (user, Oct 26): "The bari sax is really boring." From four attack sets the user picked WRECKING BALL: v2.

### #39 v2: the bari sax as a wrecking ball
- v1 kept first: its sources copied to `v1/` (paths one level deeper; `v1/sim.py` patches the frozen v1 BARI SAX into
  the roster at import), the guard repointed at `v1/` (seed 190 replays bit-identically), the mp4 renamed `_v1.mp4`.
- Engine (additive, one commit each, the guard identical on all 65 runs every time): weapon `wrecking`, ability
  `snap`, then `len_pow` (a longer cable hits harder). The pendulum is a real half period (phi = -theta cos(pi u)), the
  ball airborne all the way (the clash and `move()` already skip `air`), hoisted back first so it swings *through* the
  enemy instead of starting on top of it.
- The first wrecking ball hit 100 % of its swings: the bassoon hops behind the bari every beat, so it is always within
  reach of anything that sweeps the bari's surroundings. What made dodges possible: hits only on what is in front of
  the swing, an enemy mid-hop sails over it (the clash's rule; except the first / last tenth of the hop), the aim reads
  where a hop will land, and the bottom of the arc aimed a little short of the enemy so the contact comes at full
  speed. The hit rate then hangs on the beat grid (the bassoon is airborne on 16ths 0-2 of every beat): normal swings
  bottom out on beat 3 and land 84 % (200 seeds), SKANK's off-beat swings 23 % (they meet the bassoon in mid-hop; they
  aim further short to catch it landing). So SKANK reads as a flurry the bassoon mostly hops over.
- Tuning: lead changes were the hard part again (the bari leads early, the bassoon's growing stomps take over). A
  random search (`tune_search.py`, 120 x 60 seeds) did find the shape this time: a lower damage scale (0.32) for a
  longer back-and-forth, a cable that grows fast and hits harder with length (`len_pow` 2.5), a big CABLE SNAP as the
  bari's comeback. Fits 3.5 % over the tuning seeds; the honest search took 166 runs for 8.
- Picture: at the 58 deg view the pendulum's real height (0.7 units) read flat, so the crane system is drawn 1.6x
  taller (pivot and ball together; ground positions exact). The cycle is rebuilt from the run's events (cable, swing,
  smash) and the WRECK record, so the ball is placed exactly between frames (smear ghosts, trail). Hops that sail over a
  swing are drawn as leaps (1.45 units) with the crane drawn under them: at the normal hop height the bassoon sat on
  top of the bari and the cable seemed to hook the bassoon.
- Music: no honk on every bar any more; the bass line is one long honk per swing, swelling to the bottom and bent like
  a doppler pass, the kick stepping back on the beat where it bottoms out. Levels PASS without changes (spread 4.3 dB).
- Checks: levels PASS (wav + mp4); band check bari +3.6..+7.0 dB over the bed while sounding; safe zone OK; CoC 0;
  flashcheck PASS; flash test 0 on 41.5-52 s; loopcheck PASS render 0.4x / mp4 1.0x. Render 7 min, then patch renders for the leaps
  (67 frames, the crane drawn under a leaping ball), mux. Guard: 65 runs identical + v2's seed 27 added (70).
- Feedback (user, Oct 27): "Changes have been made to the Bari Sax's model and attack. Re-render this video": v3.

### #39 v3: the detailed bari, swinging out over the ropes
- What changed came from #40 v2 (session_01CzTfqe): the Pixel Pitch bari's detail pass and the wrecking kind's opt-in
  `overhang` (made the roster's BARI SAX standard, with #39's sim pinning it off). v2 first went to `v2/` with its
  BARI SAX frozen in full (the #39 v1 pattern) and its sprite caches: its frames re-render byte-identical to the
  delivered raw, the guard runs seed 27 there.
- Balance: with full swings almost anywhere the bari won 83 %. The roster's numbers now serve #40 too, so the re-tune
  used the episode's own knobs, as #40 did: a per-fighter damage handicap (`Rules.dmg_side`, the bari x0.8) and the
  shared scale (0.34). Fits 2 %, lead changes ~0.6 a run; 262 honest runs for 8 fits.
- Sprite caches (`check/sprites_ballfight13.npz`, `check/sprites_title.npz`) must be deleted when a model changes; the
  title card builds its big sprites lazily, so render a frame with both balls on the card once in one process before the
  parallel render (otherwise the workers race to write the cache).
- Ported #40 v2's picture for the swing over the ropes (pressed ropes, spring-back wobbles as synthetic rope events,
  drawn after the front ropes) and its sounds. Its slung-back whoosh read its length from the wrong event fields
  (`e[7] - e[6]` is t0 minus a position, so whooshes seconds long, growing through the fight); here it is
  `e[8] - e[7]` (0.27 s). #40's audio.py still has the old line.
- Checks: levels PASS (wav + mp4, spread 4.3 dB); band check bari +3.4..+7.0 dB while sounding; safe zone OK; CoC 0;
  flashcheck PASS; flash test 0 on 41.5-52 s; loopcheck PASS render 0.4x / mp4 1.1x. Render 7.2 min + mux.
- Feedback (user, Oct 27): "The Bari Sax is supposed to be able to swing over the ropes so it can get a full swing
  even if its not in the middle of the ring." -> v4.

### #39 v4: a full swing near the corners too
- Why v3 still had short swings: `overhang` lets the ends go out over the sides, but the corner-post cap (|x| + |z|,
  because the ring's left / right corners sit at the frame edges) cuts any swing that runs into a corner. The swing ran
  along the bari -> target line, so a fight near a corner (the bassoon backs into them) got 9-20 deg swings: 25 % of
  swings below full, 9 % under 25 deg.
- Fix (engine opt-in `steer`, own commit): when the cap would cut the swing short, try turning its plane in 10-deg steps
  each way and take the direction with the most room (the full swing if any fits), with the bottom moved so it is still
  just short of the target along the new direction (so the smash still comes at full speed). No randomness added, so
  the jitter stream is unchanged. 98 % full swings, none under 25 deg; the smash rate is unchanged (59 %).
- Made it the roster's standard for the bari (steer 90) and pinned it off in #40 v2's sim.py (the convention #40 used on
  #39); #39 v3 to `v3/`. Balance stayed inside the window with v3's knobs (56 / 44), so no re-tune.
- A turned swing can hoist the ball up to ~2.9 units in one 16th: the motion smear now also covers a long hoist.
- Every swing honks now (the thinning used to drop up to 30 % of SKANK's honks on a losing bari, but a swing on screen
  without its honk reads as a dropout); with it the bari sits +3.7..+6.9 dB over the bed while sounding.
- Toolkit note: caption puffs (`reelkit.ui._sand_puff`) are seeded with Python's `hash()` of the caption text, which
  is randomised per process, so the two render workers draw different puff dots on alternate frames and a patch or
  snapshot re-render of a caption frame differs there. A stable hash (e.g. `zlib.crc32`) would fix it.
- Checks: levels PASS (wav + mp4, spread 4.7 dB); band check bari +3.7..+6.9 dB; safe zone OK; CoC 0; flashcheck PASS;
  flash test 0 on 41.5-52 s; loopcheck PASS render 0.4x / mp4 1.0x. Render 7 min + mux.
- Feedback: (pending)
