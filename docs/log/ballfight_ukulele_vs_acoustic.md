# Log: `projects/ballfight_ukulele_vs_acoustic`

Append new entries at the end (newest last).

## BALLFIGHT #7: Ukulele vs Acoustic Guitar (`projects/ballfight_ukulele_vs_acoustic`)
- Seventh BALLFIGHT episode (#33) on #27's engine, ring, HUD and count-in. Little brother vs big brother: UKULELE
  (pink, left) body-slams on every 2nd beat (new weapon kind `dash`: a speed burst that decays like a knockback, so
  power = speed + distance; contact within 0.5 s hits once; +0.08 power per landed dash), ACOUSTIC (wood tan, right;
  "ACOUSTIC" everywhere, never GUITAR) strums a 26 deg cone of sound out of its sound hole on every beat (new kind `cone`:
  +0.06 length per landed strum). Abilities CHUNK / SPEED STRUM / UKE-NADO (a spinning whirl); CAPO (a capo clamps
  across the ball, cones on the 8ths) / FINGERPICK / BIG STRUM (a giant tortoiseshell pick's downstroke). The uke
  sprite sits smaller in its ball (0.70 vs 0.86 of the radius). Spaghetti-western standoff: A minor, Am - G - F - E,
  129.84 BPM (33 bars).
- Honest run, seed 874 (6 fits in 974; UKULELE 57 % / ACOUSTIC 43 % over 894 decided): the uke bullies early, it stays
  within ~10 HP from 20 s, BIG STRUM 43.9 and UKE-NADO 44.8 swing the lead twice (44.9 / 46.5 s), 9.4-9.4 at 50 s,
  UKULELE wins at 51.87 s with 6.2 HP. Captions CHUNK 7.9, CAPO 13.9, SPEED STRUM 21.7, FINGERPICK 27.3, BIG STRUM 43.9
  (UKE-NADO 0.9 s later goes to the HUD status line, 4 s rule).
- Engine lessons: seven sessions edited fight.py / fighters.py within the same hour; every push hit a conflict, always
  adjacent additions (new list attributes in __init__, new blocks in run()). Keep both sides, re-run
  tools/ballfight_replay_hash.py after every rebase, then push. A new kind on its own 16th counter with its own local
  names (`dc_*`) merged cleanly next to others' blocks. Comparing cached runs: the `K` array is NaN-padded, so compare
  with equal_nan (a plain array_equal reports a false mismatch).
- Rules lessons: the first tune on 64 seeds said 55 % for the uke; the search (826 runs) said 64 %. Re-tuned on 160
  seeds a cell (dash damage 5.2 -> 4.8). Balance swings ~8 points per 0.4 dash damage.
- Sound lessons: the fighters are the rhythm section (uke strums on its dashes, acoustic boom-chick on its cones), so
  the bed has no rhythm guitar of its own; the whistle at the first level sat level with the fighters - at -8.5 dB and
  the twang -3 dB the fighters sit 7.5-12.5 dB over the bed in 300-5000 Hz.
- Loop lesson: the mp4 seam failed at 2.6x (render 2.4x): the "3" flying in on the last frame was 97.4 px against 97.5 at
  frame 0, so its nearest-neighbour rows shifted. The fly-in now lands on exactly frame 0's size one frame early;
  patched the last beat into the kept raw (patch_frames.py) and re-muxed: 2.3x PASS. No second full render.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 2.3x, 2.7 dB,
  no click); flash 0 general; opening -19.7 dBFS, build +3.5 dB, no quiet windows. Render 16.5 min + 5 min mux.
- Feedback: (pending)

### v2 (BALLFIGHT v2 redesign, user Oct 24): SURF'S UP vs SIX-STRING RAKE
- New signatures on new engine kinds (additive, small commits; #27 / #28 replay identically; replay guard moved to the
  new seed): `surf` (a speed burst + a wave front that crashes once and pushes the enemy along it; height grows), `rake`
  (six string lines sweeping sideways across the front, alternating down / up; one hit per stroke flicks the enemy
  along the stroke; length grows), abilities `whip` (FINGERPICK), `sweep` (BIG STRUM, ring-wide), `spout` (UKE-NADO, a
  homing waterspout, 4 ticks; its path is logged in out['X33_SPOUT'] for the renderer).
- Rules lessons: aimed dead on, both signatures landed ~100 % (uke 71 hits, acoustic 129): a deterministic damage race
  with no variance and no lead changes. Option `aim_k` (the attack goes partly where the ball is heading) brought hit
  rates to ~55 % / ~64 %; slower growth (0.07 -> 0.03 both) raised lead changes from ~0.3 to ~0.5-0.7 per run. The
  tuner now prints the share of decided runs (timeouts had been counted as acoustic wins).
- Look lessons: from the 3/4 top-down camera a wave reads by its white foam crest, a bowed front and a turquoise body;
  dark blue flat faces read as a box. Strings need one wide warm glow under the bundle to read on the lilac canvas.
  Claw marks from every stroke (life 2.6 s) keep the canvas busy without noise.
- Honest run, seed 1248 (6 fits in 1248; UKULELE 54 % / ACOUSTIC 46 % over 1104 decided): UKULELE wins at 52.33 s with
  4.2 HP after BIG STRUM flips the lead at 41.9 s. Checks: flash 0 (max 16.4 %); loopcheck PASS render 1.8x, mp4 2.1x;
  CoC 0; safe zone OK; opening -19.4 dBFS, build +3.4 dB. Render 23 min CPU (~12 min wall) + 4 min mux.
- Feedback: (pending)

### v3 (series standard, Oct 24): the VS title card intro
- Bar 0 = the shared VS title card (`projects/ballfight/title.py`; #35's session moved it from #30 a minute before my
  own move, so I dropped mine and used theirs, then added UKULELE / ACOUSTIC panel patterns in a small commit; #30's card
  stays pixel-identical). 3-2-1 on bar 1, FIGHT! on bar 2 (3.70 s); top caption, series-tag strip and the end-of-file
  '3' fly-in removed; HP numbers + HUD hidden while the card is shut. Chords re-laid so the card sits on Am, the count-in
  on E, and bar 32 (E) resolves into the slam.
- Seed 1248 at 2 bars is a different fight (K.O. 54.2 s), so the honest search was re-run with the rules unchanged:
  seed 1130 (UKULELE wins at 50.97 s with 4.3 HP, two lead changes, UKE-NADO captioned).
- Audio: the louder card lifted the first half, so the early fight is pulled back 0.4 dB to keep the build at +3.3 dB.
  (A comment pasted mid-line had swallowed five GAIN_PTS points; caught by the levels re-check.)
- Checks: loopcheck render 0.9x / mp4 1.5x; flashcheck 1 doors pair (1/s); CoC 0; safe zone OK. Render ~9.5 min wall
  + 3.5 min mux.
- Feedback: (pending)
