# Log: `projects/ballfight_trombone_vs_horn`

Append new entries at the end (newest last).

## BALLFIGHT #8: Trombone vs French Horn (`projects/ballfight_trombone_vs_horn`)
- Episode #34 on #27's engine, ring, HUD and count-in (imported; renderer copied from #28). TROMBONE (left, mint) aims its
  slide and shoots it out like a lance on every beat (new blade `mode='thrust'`: reach 0.95 -> 2.5, +0.08 per landed hit,
  swats arrows it passes through); HORN (right, copper) fires curving hunting-call arrows every 2 beats (#29's `curve` +
  new `curve_grow`: 52 -> 137 deg/s; +half an arrow per landed volley). Abilities: SAD TROMBONE (shockwave + new `slow`:
  0.6x for 1 s) / POWER SLIDE (thrusts on 8ths) / PEDAL TONE (a giant flat-shaded brass bell drops in and stomps);
  STOPPED HORN (copper bell-rim shield) / HORN RIP (arc of 7 curving arrows, new arc `curve=True`) / TALLY-HO (chevron
  wave). Mood: cinematic brass trailer, D minor at 121.97 BPM; the bed has no brass (strings, braams, taiko).
- Honest run, seed 1261 (8 fits in 2,848; TROMBONE 51 % / HORN 49 % over 2,776 decided): lance leads early, arrows drag
  it level by 32 s, 4 lead changes, PEDAL TONE lands 45.3, TROMBONE wins at 50.21 s with 4.8 HP.
- Parallel-production lessons: six other sessions pushed engine additions while this one was tuning; two rebases with
  conflicts (keep both sides, then verify with tools/ballfight_replay_hash.py against the upstream file before pushing).
  #29 had already added a `curve` volley (homing with a lifetime), so this episode dropped its own homing arrays and only
  added growth + curving arcs on top: check the latest engine for an existing option before adding a parallel one. New
  recorded arrays (EXT, SLW, CRV) only exist when a fighter uses the options, so the shared hash guard stays identical.
- Rules lessons: homing vs a lance is touchy both ways (strong homing won every run by ~20 s; slow arrows + a parrying
  lance lost every run); the lance's parry is the counterplay that balances them. Desperation moves are damage-scaled too:
  at dmg_scale 0.47 they needed raw 24 / 20 to swing anything. 80 seeds read 49 % but 812 read 61 %: check balance on a
  few hundred seeds (tune.py now uses seeds 1.. like the search; it runs at scale 1.0 unless `scale=` is passed).
- Look lessons: the giant bell as stacked translucent discs read as a mushroom; a flat-shaded surface of revolution
  (painter-sorted quads) reads as a bell, but brass under the spot needs a base ~0.5x BRASS or bloom turns it white.
- Loop lesson: this fight changes little per frame (median step 1.15-1.4), so the downbeat on frame 0 (corner lights,
  LEDs, ropes) stood out at 2.6x; a 120 ms pre-beat ramp in timeline.beat_pulse (was 70 ms) -> 1.5x (render), 1.9x (mp4).
- Sound: thrusts = the ostinato (glissando into each stab via a time-warp), landed lances = accents with a 5-semitone
  glide, volleys = the horn's call (horn fifths when leading); fighters 8-13 dB over the bed after giving the trombone
  bus 1.5x (its voice is quieter in 300-5000 Hz than the horn's).
- Checks: safe zone OK; subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4; flash 0 general (max 8 %);
  opening -19.0 dBFS, build +3.8 dB. Render 0.27 s/frame/core (4.1 min) + mux; ~8.5 min total.
- Feedback (v1): the BALLFIGHT v2 redesign (user, Oct 24): attacks more unique, more visual character; the trombone's
  lance is kept, the horn gets THE HUNT.
- v2: HORN = THE HUNT (engine `pack` + ability `charge`): a horn call every 2 beats brands the trombone (copper target
  mark) and releases a ghost hound (pack 2 -> 5); hounds flank, bite on the 8ths, heel, guard (STOPPED HORN), sprint
  (HORN RIP), charge in one line (TALLY-HO). Re-tuned (damage scale 0.60, bite 2.8; 50 / 50 over 873 decided runs), new
  seed 894: the pack leads 83-58 at 26 s, the lance levels by 35 s, TALLY-HO 42.8, PEDAL TONE answers, TROMBONE wins
  50.72 s with 4.0 HP. Effects from reelkit.fx (brass sparks, copper embers, lights, bursts, marks, shimmer, moments).
- v2 lessons: hounds spawned toward the enemy ran straight into the lance's line (50 knocked out a fight, pack never
  grew); fanning them out of the bell and flanking (approach from 110 deg off the lance line) fixed it (~2.5 hounds
  alive). Engine pushes raced six other sessions: a script (rebase with --autostash, keep-both on adjacent insertions,
  replay guard vs the upstream files, push, retry) - a plain `git pull --rebase` silently fails on an unstaged file and
  every push then looks like a race. Hounds as top-down silhouettes read as rats; 2.5D side-profile sprites (long and
  low, drop ears, sabre tail, gallop) read as hounds. The new attack lights blew PEDAL TONE's brass bell out again:
  base 0.42x BRASS and the stomp's flare at canvas height. The flash test needs `rel_lum` on 0..255 frames.
- Checks (v2): safe zone OK; CoC 0 (hounds included); loopcheck PASS render 1.5x / mp4 1.8x; flash 0 general (max
  15.5 %); opening -19.3 dBFS, build +4.3 dB. Render 10.8 min on 2 cores (fx ~2.6x v1).
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
- Feedback (v4): the user replaced the HORN's attacks (Oct 25): "HORN v3" = ECHO CALL + FINAL FLICK, plus the HUD
  ability list (series rule).
- v5: new opt-in engine kinds, each in its own commit (replay guard identical for the other 9 episodes; THE HUNT's
  `pack` stays): weapon `echo` (a blast ring on every beat; the ball's path is kept every substep and up to 4 ghosts sit
  j beats back on it, each repeating the call j beats later, so the repeat lands where the call was sounded: a delay in
  space; +1 ghost per landed blast) and ability `flick` (a glove winds up a beat tracking the enemy, flicks out up to 2.4
  beyond the surface, flings it straight across the ring; a rope hit within 0.9 s adds damage, like the tuba's spit).
  HORN roster v3 in its own commit; TROMBONE unchanged. Re-tuned once (damage scale 0.72, echo 4.3, start with no
  ghosts: 400 seeds 49 / 51 % decided); honest search at 2 bars: seed 140 (8 fits in 719): TROMBONE wins at 49.22 s with
  6.8 HP after FINAL FLICK flings it into the ropes at 47.2 s and PEDAL TONE answers at 48.7 s.
- Lessons (v5): starting with a ghost (n0 = 1) let the echoes front-load damage (the horn led at 15 s in ~86 % of
  runs); starting with none evened the opening (the trombone led at 15 s in 20-40 %) and gave more lead changes (1.0-1.4
  per run vs 0.3-1.1). A ring that hits "when its front crosses the enemy" misses an enemy already inside a ghost's starting radius, so
  the echo kind hits when the ring reaches the enemy's near surface instead. The cartoon glove reads best flat on the
  balls' plane (back of the hand up, three stitch lines, ink outline) on a zig-zag copper spring; at 1x it looked
  small next to the ball, so it is 1.45x (~1.9 units cuff to fingertip). Render 4.4 min on 2 cores (the effects are cheaper than v4's hounds).
- Checks (v5): safe zone OK; CoC 0 (ghosts + glove included); loopcheck PASS render 0.5x / mp4 1.2x (audio 2.1 dB, no
  click); flashcheck PASS (1 = the doors opening; FINAL FLICK's impact changes <= 10 % of the frame); fighters 9-15 dB
  over the bed in 600-2500 Hz.
- Feedback (v5): (pending)
