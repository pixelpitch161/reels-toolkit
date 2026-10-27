# Log: `projects/ballfight_clarinet_vs_oboe`

Append new entries at the end (newest last).

## BALLFIGHT #11 (#37): Clarinet vs Oboe, two new fighters + the first levels rule episode
- Reference photos: Chrome wasn't reachable from the session, so the built-in browser (the desktop app's pane) found
  them on Wikimedia Commons. The Yamaha YOB-831 product shot is a straight-on 863 x 6937 px strip: measured with a
  canvas script in the page (same-origin image) plus scaled screenshots, at 590 mm body length. At ball-sprite size
  the true oboe is a 1-2 px line (and the clarinet nearly the same), so `oboe_lowpoly.py` has a `ball` scene with every
  radius x1.6 about the bore (lengths true); the true model stays the default for a future Pixel Pitch video.
- Engine: a teleport needs no per-frame state (positions jump in P; events carry both spots). The jump / beam timing
  decides everything: the beam locked after the beat's jump hit ~80 % (oboe ran away with it), locked before it hit
  ~0 % (clarinet 100 %); lock 3 / fire 3 sixteenths after the bar line with no lead gives ~64 % and real dodges when
  CLARION jumps on the 8ths. Lead changes stayed low (~0.6 a run) until OUT OF TUNE's weakening was strong (x0.4 at
  full depth) so detuned stretches swing the fight; fit rate 1.7-2.5 %, close to #36's.
- Stun is general: every weapon dispatch skips a stunned fighter (getattr default, so old episodes are untouched);
  blades already spinning keep spinning.
- Picture: the zip line in lavender vanished on the lilac canvas under the spot: it needed an ink edge, a white core
  and a luminous halo, and an arc (it reads as a leap). DOUBLE REED's blades draw over both balls while they snip
  (the enemy is always close then, so painter order hid them). Beams stop at the ropes. OUT OF TUNE = a doubled, warped
  olive outline + a sour glass tint + a seasick sprite roll in 4-degree steps (cached: 1672 sprites).
- Music: consistent loudness from frame 0 with no automation at all: one master gain + compressor + soft limiter, the
  storm already full under the title card, building with parts (violin II, violas, arpeggios, timpani on 3). First
  levels run passed (spread 4.8 dB); trimming the title tutti and thickening bars 34-35 brought it to 4.3 dB.
- Checks: levels PASS; band check +7..+12 dB; safe zone OK; CoC 0; loopcheck PASS render 0.5x / mp4 1.0x; flashcheck
  PASS (the doors). Render 7 min + 4 min mux.
- Feedback (Oct 26): "in the last couple of seconds of the fight, the clarinet stops making noise". Cause: the HP-based
  thinning (drop some of a hurt fighter's notes) used #36's `h01`, which is linear in its input: on consecutive slots
  its values creep up by ~0.007, so all 15 of CLARION's 8th jumps after 48.9 s (clarinet at ~10 HP) fell under the
  threshold and the line went silent; the oboe's stings had 1 s holes too. Fix: a splitmix64 hash + never two dropped
  notes in a row (max 30 %). Re-muxed only. Lesson: check each fighter's stem has no silent stretch while it's acting
  (0.5 s windows), not just the mix levels; #36 and older episodes use the same hash (posted, not re-rendered).
- Feedback: (pending)
