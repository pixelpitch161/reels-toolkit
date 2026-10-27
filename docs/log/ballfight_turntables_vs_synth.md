# Log: `projects/ballfight_turntables_vs_synth`

Append new entries at the end (newest last).

## BALLFIGHT #5: Turntables vs Synth (`projects/ballfight_turntables_vs_synth`)
- #31, produced in parallel with #29-#35 on the shared engine. TURNTABLES throw vinyl frisbees that ricochet off the
  ropes (new volley option `bounce`, up to 2; `on16` so SCRATCH's haste x4 fires them on the 8ths); SYNTH is an
  arpeggiator: note gems orbit the ball and hit what they touch (new weapon kind `orbit`, 2 -> 8 satellites on fixed
  seats, they also swat records). REWIND is a new ability kind: the ball keeps its own 2 s record, HP goes back to its
  value 2 s ago (`heal` event) while it zips back along that path in one beat, intangible. Mood: dark breakbeat,
  F minor, 137.70 BPM (35 bars).
- Honest run, seed 129 (5 fits in 666; TURNTABLES 47 % / SYNTH 53 % over 631 decided): satellites edge the opening, the
  records take over at 21.2 s, two REWINDs (7.2 / 8.8 HP undone), 8 satellites take the decks from 51 to 4 HP in four
  seconds, WHITE NOISE and BASS DROP both fire, TURNTABLES win at 49.72 s with 1.5 HP. All six abilities get captions.
- Parallel-production lessons: seven sessions edited fight.py / fighters.py at once; every engine commit hit a merge
  conflict in the same spots (new methods before the abilities section, the roster's closing brace). Both sides were
  pure insertions, so keeping both and re-running tools/ballfight_replay_hash.py before the push was enough. Reusing
  recorded fields (`orbit`: TH = angle, NV = satellites, BL = radius) kept the recorder and every renderer untouched.
- Bridge: appending to pp_bridge.ENTRANTS changes #26's 16-entrant roster (its run, roster grid and sprites iterate
  NAMES); later instruments now go in EXTRA (entrant() searches both; ACOUSTIC moved there). The organ / synth /
  turntables modules cache notes by event index, so voice() clears that cache (otherwise every note = the first one).
- Rules lessons: the first orbit (0.4 beyond the surface) barely reached the decks (12 hits vs 200 records); at 0.9 it
  balances. +1 satellite per landed hit snowballed, so +1 per two. Lead changes ~0.6 per run, fit rate ~0.75 %.
- Look lessons: flat instruments need tipping toward the 3/4 camera; the episode renders sprites with an extra pitch
  (deck 45 deg, synth 28) and lifts the black deck (+30) so it reads through the lime glass. Satellites at 0.15 were
  specks; at 0.24 with a facet, a core and an orbit line they read.
- Sound lessons: the satellites crossing the top of the orbit = the arpeggio gives a figure that densifies by itself
  (1 note/s with 2 satellites, ~8/s with 8 under LFO). Scratches use the Pixel Pitch deck's own motion curves on a
  vocal-free F-minor stab (the deck's own 'fresh' source imitates a voice). The synth's voice has a built-in echo at
  its own tempo: notes are cut at 0.34 s before it.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 2.4x);
  flash 0 general (max 8.6 %); opening -16.4 dBFS, build +3.4 dB; leads 2-12 dB over the bed in 400-4000 Hz. Render
  9.9 min + mux.

## BALLFIGHT #5 v2: Turntables vs Synth (THE RING IS THE RECORD vs PATCH CABLES)
- BALLFIGHT v2 redesign (user, Oct 24): new attacks for both fighters, same hook, captions, colours, mood and tempo.
  Engine (additive, three small commits): `scratch_floor` (the floor jerks forward then back every bar; each jerk is a
  knock around the ring centre; the first rope hit within 0.8 s is a slam scaled by impact speed, detected from the
  engine's own 'rope' events so move() is untouched), `tether` (plugs -> cables -> pulses on their own 16th counter; a
  leash that bounces the enemy off its end or snaps under a hard fling) and `lash` (SAWTOOTH's saw-tooth sweep; the name
  `whip` was already taken by #33's straight flicks, and no two fighters share an attack kind). All three are rebuilt
  from events in the renderer, so the recorder is unchanged.
- Tuning lessons: v2 slams at 4.0 ended fights by 15 s. At slam 1.6 / pulse 0.2 the balance flipped from 55 % decks to
  0 % as the damage scale dropped: longer fights let the synth reach 4 cables, a full leash never snapped (the knock has
  decayed by the time the ball reaches the leash) and it also stopped the decks' rope slams. snap 0.15 x cables + pulse
  0.17 + scale 0.6 = 50 / 50 with ~1.1 lead changes per run (v1 ~0.6).
- Picture: the floor angle (a slow spin, 7 whole turns per loop, + each scratch's baby-scratch curve) drives the record's
  label, strobe dots and dust specks (grooves alone are rotation-invariant, so the spin needs marks that turn); the same
  curve is the record position the scratch audio plays, so picture and sound move together. Effects from reelkit.fx as in
  the ballfight README recipe; a landed BASS DROP record must go under the balls (it painted over them at first); jerk
  streak particles need a direction (the default is straight up, which read as rain).
- Checks: safe zone OK; CoC 0 on all frames; loopcheck PASS render + mp4; flash peak 14.8 % (BASS DROP); opening
  -16.7 dBFS, build +3.0 dB; leads 4-11 dB over the bed in 400-4000 Hz. Render 8.8 min + mux.
- Feedback: (pending)

## BALLFIGHT #5 v3 (#31): the VS title card intro (series rule)
- Shared card, no fork: `projects/ballfight/title.py` (#35's session moved #30's card there minutes before my own move;
  my push conflicted, so I dropped my version and built on theirs). The episode only registers its panel patterns in
  `TT.PATTERNS` (lime record grooves + label, a cyan circuit board) and pre-fills `TT._SPR` with its own pitched sprites,
  so the shared module and #30 stay untouched.
- Seed: re-ran the honest search with the fight at 2 bars (rules unchanged): 6 fits in 1339 runs, TURNTABLES 617 /
  SYNTH 665 of 1282 decided. Seed 1339 (v2's 781 is not a time shift): the decks lead for 24 s, the lead swaps back and
  forth from 28 to 42 s, a 9.9 HP REWIND keeps the decks ahead to the wire, and the cables' pulses take it at 50.05 s:
  SYNTH wins at 50.79 s with 2.8 HP.
- Git: a rebase stripped a commit subject that began with '#' (git reads it as a comment on `rebase --continue`); start
  subjects with a word ("IDEAS 31 v3: ...").
- Flash test before the render: decimating frames (`[::2, ::2]`) counted 3 "flashes" in the card (the grooves / circuit
  textures shaking on the slams); `reelkit.flashcheck` area-downscales to 360x640, and with the same downscale the card
  is 0 flashes (textures 15-18 %, the doors 79 % as one transition). Pre-render flash tests need the checker's own scaling.
- Safe zone: FIGHT!'s slam (punch 0.25) poked 7 px into the button column for one frame; the punch is 0.12 now.
- Checks: safe zone OK; CoC 0 on all frames; loopcheck PASS render 0.6x / mp4 1.1x (audio step 3.2 dB, no click); flashcheck PASS (one doors pair at 1.77 s, worst 1/s); opening -16.4 dBFS, build +3.1 dB. Render 11 min + 10 min mux.
- Feedback: (pending)
