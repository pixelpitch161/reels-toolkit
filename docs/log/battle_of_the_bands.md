# Log: `projects/battle_of_the_bands`

Append new entries at the end (newest last).

## Battle of the Bands (`projects/battle_of_the_bands`)
- Satisfying-first territory battle, inspired by @algorithmbattles (4-team battles) but ours: four physical-process teams
  (SAND pours/piles/avalanches = drums, BALL bounces and smashes tiles = lead, RIPPLE one ring per beat = bass, VINE grows,
  branches, burrows = keys) fight over a round LED wall on a floating low-poly festival stage; legend pinned to each team's
  base, territory bar under the wall, one speaker stack per team. Every move = a note, stage share = volume, knocked-out
  bands power down and drop out, the winner solos; at 58 s the wall cracks and the next run's burst loops into frame 0.
- Reusable engine (`battle.py`): arena shape, team line-up + balance, seed and bell time are parameters; it records owner
  grids, claims, attempted moves, knockouts and each process's per-frame state. `seed_search.py` ranks honest seeds by story
  (knockouts near 31/41/50 s, the bell leader falls, underdog winner, lead changes, a real duel, no lingering). Seed 82 of 800
  (VINE leads at the bell, three lead changes, SAND wins from tied-last 17 %); never edited.
- Rules added beyond the backlog, and why:
  - Balanced per-cell churn never eliminates anyone (it needs drift), per-cell sand avalanches snowball, and a Pong-wars
    ball is anti-snowball (a small pocket flips tiles fastest, so it lingered at 2-4 % for 20 s). Fix: "volume = power"
    (attack odds and process speed scale with stage share, i.e. what you hear), a heat ramp that restarts after each
    knockout (rounds, so knockouts space ~10 s instead of cascading), a Go-style capture (7 of 8 neighbours) that cleans up
    speckle, and a knockout below 2.5 % of the stage instead of at 0 tiles.
  - FILL left 15-30 % enclosed gaps (ball arcs and the vine web box regions in), so the last 4 s of FILL close up (empty
    tiles next to a team fill in, accelerating) and the bell snaps the rest.
- Music: original A minor, 120 BPM, | Am | F | C | G |. Gating on claims alone left RIPPLE's bass nearly silent (boxed in
  for ~10 s while alive), so the engine also logs attempted moves: a landed move plays accented, a failed one a ghost note;
  a stalled team (no move at all) still stutters. Volume = share of the claimed stage (FILL counts like FIGHT); a slow band
  leveler (max +3.5 dB over 1.5 s) keeps energy without hiding stutters. Opening chaos: every 6th claim rings a random chord
  tone off-grid, locking to the grid by 4 s.
- Look lessons: a crowd standing on the stage read as a field of posts; a pit below the stage edge shows heads and raised
  hands and frees the boards for moon shadows, moving-head pools and a glossy reflection of the LED wall (sampled by
  mirroring the eye in the boards). LED wall = per-pixel ray/plane hit into a 4x texture of the grid (grains, ball + trail,
  ring bands from each ring's geodesic distance field, vine tips, claim flashes, knock-out embers, note sparkles).
- Checks: safe zone OK (bar lifted 3 px: the panel's drop shadow crossed y 416); LED wall + props CoC 0 on all 1800 frames;
  flash check 0 general flashes (max 14 % at the crack); audio -14 dBFS fill -> -12 fight -> -10.8 solo, quietest 0.5 s
  -16.9 (the breath before the bell); first render's loop seam jumped 4.6 (nebula drift, twinkle, pennants, spot pools,
  a searchlight term not whole cycles over 60 s) -> every periodic motion made whole-cycle, seam 2.1 vs 1.1 normal step.
- Render ~0.6 s/frame/core (18 min on 2 cores); seed search ~3 s per run.
- Feedback (v1): start the video when the stage is empty, so the loop starts and ends where the video does.
- v2: same run (seed 82), video time = run time: frame 0 is the empty stage as the four seeds drop and the crack ends at
  exactly 60 s. Kept the run by keeping its bell (21.4 s) and ring phase and moving the music grid to bar lines at 1.4 + 2k s
  (so the bell is still a bar line); every event moved 1.4 s later. Seam lessons: the stage was still lit (and reflected) by
  the full winner wall during the crack (fade glow + reflection with the tiles that have fallen); the 7 s hopper swing isn't a
  divisor of 60 (slide it home during the crack, when no sand falls); the win push-in and crowd pose must release before
  60; a bar sliding out crosses the safe-box edge (fade it in, cut it on the crack).
- Feedback (v2): the VS title card intro became a series rule (user, Oct 24, after #30's test).
- v3: bar 0 = the shared VS title card (`projects/ballfight/title.py`; #35's session moved it there minutes before this one
  would have; this episode added TROMBONE slide-tube + HORN coil panel patterns, #30's card checked pixel-identical on 12
  frames), 3-2-1 on bar 1, FIGHT! at 3.94 s, top caption dropped, HUD hidden while the card is shut; card audio: a unison
  band hit per word + slide blast / thunder crack / two-horn hunting call, taiko pickup as the doors open; chords moved a
  bar (Dm under the card, A for the count-in, loop from FIGHT!, A into the seam). Honest search at 2 bars: v2's 894
  didn't reach a K.O. (the drift the rule warns about); new seed 2200 (8 fits in 2,200, 51 / 49): TROMBONE wins at 50.72 s
  with 4.0 HP after three lead changes in 1.5 s and a 3.7 s standoff at 4.0 vs 1.9 HP.
- v3 lessons: check origin right before moving a shared file (another session had just moved title.py; adopt theirs,
  add only what's missing). With the rules unchanged the median K.O. moves ~2 s later, so fewer runs land in 49-53 s
  (8 fits took 2,200 runs vs 974 at 1 bar); the solo now stops at the reassembly if a late K.O. leaves less room. The
  title slam makes a very clean seam (0.5x render, 1.2x mp4). Render 11.1 min.
- Feedback (v3): the trombone model is inaccurate (user, Oct 25).
- v4: the Pixel Pitch trombone rebuilt from reference photos found in Chrome (Yamaha YSL-891Z side view, measured from
  the image's pixels at ~0.41 mm/px using its 20 cm bell; Jupiter student tenor 3/4 view): the old slide plane was
  turned ~100 deg so the slide read as one rod, the bell sat 65 % of the way along a short slide (real: 40 %), and an
  F-attachment ellipse hung below the instrument. Now: slide ~40 deg out of the bell plane about the shared lower line,
  bell rim 31 cm / crook 78 cm ahead of the slide lock, bow 38 cm behind, Bessel bell fitted to the photo, nickel crook
  + bumper + stockings, tuning-slide braces + medallion, bell brace at the lock, a 4 cm mouthpiece, plain straight
  tenor. Every series scene still builds; the posted Pixel Pitch video is unchanged. Same fight + soundtrack; one full
  render (the trombone is on screen in every frame, so no patch render).
- Lessons (v4): read a reference photo's pixels in the page (canvas getImageData on the upload.wikimedia.org image
  tab) for real measurements instead of eyeballing a screenshot; check an instrument model in a plain orthographic
  side view next to the product photo before judging it at sprite size.
- Feedback (v4): (pending)
