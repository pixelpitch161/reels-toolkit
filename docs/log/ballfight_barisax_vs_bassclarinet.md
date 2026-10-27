# Log: `projects/ballfight_barisax_vs_bassclarinet`

Append new entries at the end (newest last).

## #40 BALLFIGHT #14: Bari Sax vs Bass Clarinet (session_01CzTfqe, Oct 26)
- Second crossover after #36: both fighters straight from the roster, sides reversed vs their own episodes. No engine
  option was needed for the sides: `Fight(a, b)` seats a on the left, b on the right, and nothing reads the roster's
  `corner`. Spin is mirrored by side (`spin_dir`), so a right-corner anchor whirls clockwise.
- New pairings can expose engine assumptions: the crane (#39 v2) predicted its target along the enemy's velocity, which
  was fine against a hopper but, against an anchored whirler, meant extrapolating the spin (~10 units/s). CABLE SNAP
  landed 3 % (96 % in #39). Measured it, flagged it (crossover rule), the user chose the fix: lead an anchored enemy
  with the engine's existing `predict()`. Gated on an anchorer being in the fight, so the guard stayed identical.
  Lesson for crossovers: before tuning, measure each signature's hit rate against the new opponent vs its own episode.
- Tuning was quick and sensitive (old engine: the bari won 0 % at its #39 / #38 scales x0.32 / x0.68, 46 % at x0.6 /
  x0.6, 95 % at x0.9 / x0.6). With the fix: x0.35 / x0.68. The honest search took 617 runs for 8 fits. Picked 617 over the top score (309), whose CABLE SNAP
  misses and LIGHTS OUT falls in the 4 s caption gap.
- Title card on swapped sides: the shared two-line option only covered name A (added for name B, opt-in); the panel
  patterns are drawn for one side of the divider, so mirror them (`styles.mirrored`, idempotent: title.py imports the
  renderer by name, so the episode's module-level patch ran twice and un-mirrored itself the first time).
- Renderer = #39's + #38's bass clarinet (styles imported, not copied). LIGHTS OUT's dim keeps a band along the crane
  line. Focus: the bari hoisted 3 up for CABLE SNAP needed sharp 0.19 (0.17 gave 0.10 px).
- Music: the bari as the whole bass line needed two things: a pump between swings (swing honks alone leave a bar in
  every two without bass), and capping the "loser thins" high-pass for a bass fighter (~110 Hz, not ~240): while the
  bari trailed 17-50 s the rule was removing its fundamental. The bed's every-beat kick (late density) sat on the pump:
  replaced by a high anvil ghost. The tail was 2-3 dB under the fight: thickened (hold bar, solo bars at density 0.85,
  bigger pump honks into the seam) rather than raised. Levels spread 4.7 -> 3.3 dB; bari over the bed +1..+4 -> +5..+7.
- Checks: levels PASS (wav + mp4); band check bari +5..+7, bass clarinet +4..+7 dB while sounding; safe zone OK; CoC 0;
  flash test 0 on 38.4-51.6 s, flashcheck PASS; loopcheck PASS render 0.4x / mp4 1.1x. Render 5.7 min + mux. Guard: 70
  runs identical + #40 added (75).
- Feedback: (pending)

### #40 v2: a detailed bari, swings over the ropes, a quieter intro
- "Completely smooth" was a sprite-scale bug as much as a lack of detail: the `ball` scene fattens every tube x1.35 for
  small sprites, but the keys were placed on the unscaled body, so they sat inside it; gold keys on gold also gave no
  contrast, and rods / guards / engraving were lines, which `pp_bridge.model_faces` drops. Lesson for new Pixel Pitch
  models used in BALLFIGHT: place details on the scaled surface, make them faces, and build contrast in value (dark
  pad rings, white pearls, deeper-gold rods), not hue. Checked by rendering the sprite at 20 / 44 / 120 px.
- Swing over the ropes: an engine opt-in (`overhang`, a corner-post cap, a slung-back `return`), made the bari's roster
  standard, with #39's sim pinning it off (the #39 v1 pattern). The swing's ends were clipped to the ring, which cut 41 %
  of swings below 25 deg; now 6 %. Frame limit: the ring's left / right corners sit at the frame edges, hence the
  |x| + |z| cap. The stronger swing needed a re-tune (bari 75 % at the v1 handicap -> x0.30).
- Picture: ropes pressed down under the ball's underside where it passes over them, a spring-back wobble (synthetic
  rope events) + a low twang when it lifts off; drawn after the front ropes when over them.
- Intro loudness: RMS (reelkit.levels) can't see brightness. The card measured +0.7 LU K-weighted over the fight (peaks
  2-3 LU over) with three stacked sheet-metal crashes; thinning by arrangement first overshot to -3.5 LU (the old "too
  quiet" problem), settled at -1.4 LU, level with the ending. `loudness.py` (BS.1770 K-weighting) is worth reusing for
  any bright, percussive bed.
- Focus: this run's CABLE SNAP hoists the bari 3 up near the camera: sharp band 0.26.
- Checks: levels PASS (wav + mp4, spread 3.3 dB); band bari +5..+7, bass clarinet +4..+7 dB; safe zone OK; CoC 0; flash
  test 0 on 36.5-51.2 s, flashcheck PASS; loopcheck PASS render 0.4x / mp4 1.1x. Render 10.7 min (the guard ran
  alongside) + mux.
- Feedback: (pending)

### #41 BALLFIGHT #15: Flute vs Trombone (crossover, session_01CzTfqb, Oct 27)
- Crossover with an undefined interaction: the lance's `parry` only knew projectiles, and cobras are basket state the
  enemy never checked. Made it an opt-in rule on `Rules` (`swat_cobras`, own commit, off by default) rather than a
  roster change, so neither fighter changes and every earlier fight replays identically; told the user. It matters:
  15-23 swats a fight. Lesson for future crossovers: check every pair of "summoned things" (cobras, hounds, strings,
  shells) against the other fighter's weapon before tuning, and add the interaction as a rule.
- Balance: this pair is "whoever's ahead stays ahead" (0.6 lead changes per fight), so fits were ~1 % (8 in 968) even
  at 49 / 51 %. Damage alone balanced it (FLUTE x2.0 / TROMBONE x0.40); no need to flag.
- Sides swapped from both fighters' own episodes: the shared title patterns (slide tubes, key cups) run parallel to the
  divider, so no mirroring was needed (unlike #40's).
- Scripted tail: when the loser falls on the winner's side, the straight glides home cross and the balls pass through
  each other (caught on the tail sheet at 59.6 s). Bowed both glides sideways (sin(pi gl): ends unchanged, so the loop
  seam doesn't move). Worth checking on every episode's tail sheet.
- Music: with the trombone as the bass (thrusts every beat), the circus groove holds itself up; the bed only needed the
  off-beat calliope. The first mix dipped 3-5 dB after the 2-bar solo (the band was "light" and nothing played the
  bass): fixed by arrangement (a 3-bar solo over the normal bed, then the champion back on the bass line), not level.
- Checks: levels PASS (wav + mp4, spread 3.6 dB); band trombone +6.6..+7.8, flute +12..+16 dB; intro -1.5 LU vs the fight;
  safe zone OK; CoC 0; flash test 0 on 40.5-51.5 s, flashcheck PASS; loopcheck PASS render 0.4x / mp4 1.1x. Render
  10.8 min + mux. Guard: #41 added; seed 392 re-verified after pulling #42's engine kinds.
- Feedback: (pending)
