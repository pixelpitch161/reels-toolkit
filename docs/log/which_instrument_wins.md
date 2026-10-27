# Log: `projects/which_instrument_wins`

Append new entries at the end (newest last).

## Which Instrument Wins? (`projects/which_instrument_wins`)
- Trend-format reel (#26, "Who Will Win? 16 entrants"), mood A dark tango: 16 glass balls, each holding a Pixel Pitch
  instrument model, drop into a crimson-and-gold ring turning over a pit in a sunset colosseum (stands collapsed on
  the sunset side so the sun sits low behind the ring, two braziers, podium torches, a crowd of pixel dots bobbing on
  the beat, pennants, pigeons, far ridges and an aqueduct). Fall out through the gap = out: the ball drifts toward the
  camera, drops into the pit on one last dull note and the 4 x 4 roster dims it with its place. Seed 406 (2 fits in
  600 honest runs): DRUMS out first at 3.6 s after rattling in the gap, BAGPIPES stuck in the gap 6.5-8.9 s, PIANO and
  TRUMPET out together at 15.4/15.7, UKULELE 9th after three near-misses, VIOLIN survives five edge hits, final 3 at
  40.3, VIOLIN vs CELLO from 43.1 with 7 close calls, CELLO wins at 49.41. The ring seals on the win, the cello floats
  out to the spotlight and plays an original 4-bar solo in its own voice while the stands light up; at 58.4 s the 15
  lift out of the pit (each plays one note of a rising cascade) and everyone eases into formation, at rest on 61.0 =
  frame 0 (they drop again).
- Music: every wall bounce plays the ball's own Pixel Pitch voice (pitch = chord tone by where it hit, left low ->
  right high, in its register; loudness = impact speed; 32nd grid), gap-edge hits ring a bell (rattles), glass clinks
  once fewer than 12 are left; bed = D-minor tango at 118.03 BPM (habanera bass, staccato strings in quarters then
  eighths from bar 10, dominant pedal + tremolo + roll for the duel, the win resolves it with a tonic hit).
- New: `reelkit.ui.roster_grid` (4 x 4 elimination scoreboard; columns sized to their longest name, places in the chip slot).
- Lessons:
  - Pixel Pitch models reuse cleanly read-only: import the video modules, run pp_framework.setup() for their events,
    call build()/synth() with a fake scene. Their display stands must go (runtime patches during the build: the stand
    helpers become no-ops, floor-anchored rods and floor discs are skipped); with stands the instrument filled a third
    of the ball. Thin instruments read only with a fixed best view: per-instrument yaw centre, +-swing and a screen
    roll (violin, guitar, flute tilted ~30 deg), never a full spin (a spinning cello read as a stick). A light glass
    interior is what makes black models (piano, clarinet, accordion) readable.
  - Ring pacing: with gravity, a gap barely wider than a ball still drains piles fast (a ball wedged between the two end
    caps is squeezed outward). What paces it is how briefly the gap sits under the pile: 20 turns in 61 s (2.06 rad/s,
    also loop-safe), gap 0.303 -> 0.48 rad, wall restitution 1.02 with a speed cap. Rules were tuned once on median
    elimination times over 10 seeds (40 random rule sets, `check/rulesearch.py`), then frozen for the seed search.
  - Duels with only two balls rarely end on their own (exits need the pile pushing): most runs had 10-17 s duels; the
    window "3rd-last out 41.5-46.5 s, win 48-50.6 s" kept only ~0.3 % of runs.
  - A white-hot sun under top_scrim() turns grey: draw the disc after the grade on sky pixels only (banded rings).
    A low visible sun means grazing light: shadows of the ring and balls fall behind the camera, so the floor drama
    comes from the stands' long shadows; the stands and the podium must be broken low on the sun's side or the arena
    sits in shade.
  - The ball names hidden during the scripted fly-back popped back on frame 0 (seam 3.0x); showing them from 60.25 s
    (balls nearly home) -> 1.5x.
  - Loop encode: the toolkit mux trims the soundtrack to 61.0 s, AAC pads the last frame with silence and
    `loopcheck out.mp4` failed (7.7 dB step) although the render check passed. Building the audio as 2860 AAC frames
    and re-muxing through projects/maze_race/mux.py (imported, not copied) -> PASS (seam 1.8x, 2.5 dB). Two projects
    now need it: worth moving into reelkit.video.
- Checks: safe zone OK; subject CoC 0 on all 1830 frames; flash 0 general flashes; bounces 7-15 dB over the bed in
  400-4000 Hz; opening -18.7 dBFS, build +4.1 dB. Render ~0.26 s/frame/core (7.8 min on 2 cores) + 3 min mux.
  Time: ~3.5 h, mostly the ring-pacing rules and the colosseum look.
- Feedback (v1): no "ITS STORY: / ON OUR PAGE" at the end; move the roster to the loosened safe zone.
- v2: the cross-promo caption is gone ("WHO DID YOU / PICK?" holds to 58.25 s); roster_grid called with its new
  defaults (bottom of the zone, left of the button column); ball names stay above the roster and out of the button
  column (safe_overshoot); ring recentred on x 180 so it sits under the frame-centred captions. Full re-render + mux.
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
