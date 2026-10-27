# Log: `projects/ballfight_bagpipes_vs_accordion`

Append new entries at the end (newest last).

## BALLFIGHT #4: Bagpipes vs Accordion (`projects/ballfight_bagpipes_vs_accordion`)
- Fourth BALLFIGHT episode (#30), produced in parallel with #29–#35 on the shared engine. Two new weapon kinds, both on
  their own 16th counter (additive: `grid16_fire`, `aura_tick`, `fire_bellows`, `suction_step`): BAGPIPES' `aura` (a
  drone ring that hurts on every 8th while the opponent is inside; +reach per contact) and ACCORDION's `bellows` (a
  stroke every 2 beats alternating a push ring and a collapsing suction ring that yanks the opponent in). Abilities: WAR
  CRY / DOUBLE DRONE (ticks on the 16ths) / BAG BURST; POLKA (a stroke every beat) / SQUEEZEBOX / FULL BELLOWS (a giant
  accordion with its keyboard on top, squeezed shut on the predicted spot). Mood: dark sea shanty, A dorian, 121.97 BPM.
- Honest run, seed 1581 (6 fits in 1854; BAGPIPES 51 % / ACCORDION 49 % over 1637 decided): the accordion leads from
  16.9 s, the bagpipes retake it at 28.8 s, FULL BELLOWS misses at 39.9 s, BAG BURST at 44.3, 8-7 at 48 s, BAGPIPES
  win on the last tick at 50.55 s with 2.1 HP.
- Parallel-production lessons: added `tools/ballfight_replay_hash.py` (hash every finished episode's honest runs before
  and after an engine change; other sessions picked it up). Engine commits rebased onto five other sessions' engine
  commits; every conflict was "keep both" (new kinds side by side in `__init__`, `run()`'s grid blocks and step calls).
  Give each new grid weapon its own counter instead of widening the shared pulse condition: no existing line changes.
- Rules lessons: a small aura (0.35 beyond the ball) almost never bit; base 0.9, growth per contact (not per tick) and a
  1.9 cap keep it from snowballing. The brief's "push one bar, suck the next" left the accordion idle ~2 s at a time;
  every 2 beats reads as breathing (flagged to the user with the early sheet). A longer suction (pull_k 1.7) raised lead
  changes 0.5 -> 0.8 a run, because every suck can drag the accordion into the drone. Captions are the tight window
  (first uses cluster): ~27 % of runs have 5 captions >= 4 s apart.
- Look lessons: the aura as added canvas light read as a pale white ring; painting it as a tartan band (albedo blend, lit
  like the canvas) reads as the bagpipes at a glance. A round stacked-disc "bellows" read as a cake; a square pleated box
  with the keyboard on top reads as an accordion instantly.
- Sound lessons: the drone is synthesised at frequencies nudged (< 0.01 Hz) to whole cycles in the file, so it loops
  sample-perfectly, and its level is automated: under the mix, swelling only while the aura lands (a leaky integrator of
  the landed ticks), back to the opening level before the seam. Chanter via the Highland module's own `chanter()` (the
  bridge's `voice()` adds the whole drone to every note); accordion chords via `reed()` with the 'golden' tone (the
  showcase 'morino' scene adds a dotted-8th echo).
- Feedback: (pending)

## BALLFIGHT #4 v2: Bagpipes vs Accordion (BALLFIGHT v2 redesign)
- v2 of #30 on the user's approved redesign: BAG INFLATE (the bag swells to ~1.6x size and mass over 2 beats, then
  rockets around like a released balloon, zig-zagging and squealing) vs BELLOWS BITE (the end boards fly apart with the
  pleats as jaws and snap shut); WAR CRY / HIGHLAND FLING / BAG BURST; POLKA / SQUEEZEBOX / FULL BELLOWS (a ring-wide
  clamp). Effects on reelkit.fx. Hook, captions, colours, mood and tempo unchanged.
- Engine (small separate commits, all additive): guarded per-ball radius / mass / speed + clash hooks + a RAD record in
  move() / snap(); weapon kinds `inflate` and `bite`; ability kind `clamp`. Proof of "no replay changes": hash every
  guarded episode with the upstream fight.py and with mine (45 runs identical), as well as #27 / #28 vs the morning's
  snapshot. Four rebases onto other sessions' engine commits; every conflict was keep-both, except one where the
  keep-both landed my `elif` after another session's new `if` block (a syntax error caught by `ast.parse` before
  committing): always parse after resolving.
- Rules: the accordion first won every run (bites ~60 % on target, the bag rarely landed); ~7 damage per landed flight
  balanced it; size growth +0.015 per hit up to 1.65x keeps the brief's ~1.6x. Seed 1442 (6 fits in 1466, 52 / 48 %):
  the bagpipes lead by 20, FULL BELLOWS lands at 43.8 s, three late lead changes, ACCORDION wins on 1.8 HP.
- Look: the tartan stretched over the swelling glass reads as "the bag" at a glance; a full-height FULL BELLOWS wall
  along the axis hid both fighters (the axis runs through them by construction), so it is now low (0.32) and drawn
  under the balls, with tall end boards. The flight ribbon needed width 7 (internal px) to read under the puffs.
- Seam: v2's calmer fight lowered the median frame step (1.56 -> 1.18), so the same count-in seam failed (2.5x); the
  flying "3" now reaches its landing size on the last frame (frame 0 repeats it): 2.1x render, 2.4x mp4.
- Feedback: (pending)

## BALLFIGHT #4 v3: the VS title card intro test (#30 only)
- User direction (Oct 24): a better hook than the bare 3-2-1. Bar 0 is now a fighting-game VS card (`title.py`):
  diagonal panels in the fighters' colours with each ball big and close, BAGPIPES / VS / ACCORDION slamming in on
  beats 1-3 (overshoot, streaks, shake, dust, local light; drone blast / thunder crack as VS splits the divider /
  accordion chord), doors sliding apart along the crack on beat 4; bar 1 = the 3-2-1, FIGHT! at 3.94 s; the old top
  caption is gone. Words at the biggest pixel scales that fit the safe zone (6 / 12 / 5; ACCORDION left of the button
  column); only VS pulses on beat 4 (a name pulsing by 5 % went 1 px out of the zone).
- Seed: 1442 did not replay identically with the fight one bar later (positions drift ~1e-7 from float rounding of
  the absolute substep times, and the chaos amplifies it from ~31 s: K.O. 51.41 instead of ~52.4). Re-ran the honest
  search at 2 bars (rules unchanged): 1442's new run ranked first anyway (same first 30 s, a closer ending: ACCORDION
  wins on 0.24 HP). Lesson: shifting t_fight is not a pure time shift for this engine; test, don't assume.
- Loop: the title slam is the frame-0 reset. The doors slide shut over the last 0.30 s (each step ~1/9 of the change, so
  loopcheck reads them as rhythm, not a reset) and BAGPIPES falls in as a ghost that lands exactly on frame 0's word.
  Two seam traps: the slam's light / streaks / ball glow / shake started ON frame 0 (seam 5.2x) - they now rise from the
  next frame; and a size factor of 2.2500000000000044 vs 2.25 rounded the word one pixel taller (banker's rounding of
  94.5) - snap the ghost to the exact frame-0 value. Seam 0.9x render, 1.5x mp4; audio: the first 20 ms (the slam) was
  7.7 dB over the last 20 ms until the doors' whoosh ramps to full into the seam (3.9 dB).
- Flash: one up/down pair (doors shut / doors open), worst 1 per second; the panels are never white.
- The HUD is skipped while the card is shut (its notes overlapped the caption in reelkit.safezone even though hidden).
- Feedback: (pending)
