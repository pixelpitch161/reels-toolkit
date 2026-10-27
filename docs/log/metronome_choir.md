# Log: `projects/metronome_choir`

Append new entries at the end (newest last).

## The Metronome Choir (`projects/metronome_choir`)
- Sequel to #1 on `reelkit`: 199 metronomes on 5 tiered risers (4 x 10, one empty seat = the soloist's) + a golden
  soloist on a podium, low-poly concert hall lit by a big round skylight (sun = key), organ facade, stage wings, warm
  truss spots, footlights, a ginger cat on the grand piano, timpani, pawn listeners. #1's metronome model, tip colours
  and soda cans kept for series recognition; caps and board edges in section colours.
- Physics: two-layer Kuramoto. Each riser couples its own 40 once it settles on its cans; the platform on giant cans
  couples everyone (and surges, as in #1) and drags in the soloist (15 % fast) last. Calibration is exact because the
  model is rotation invariant: per-riser rotations set the 5 sections' flams before the platform (+-70-150 ms), a
  global rotation puts left-end ticks on whole seconds (bar lines on even seconds, drop at 38.0 = a downbeat), the
  soloist's start phase is searched (it barely moves the crowd, so it is integrated alone against the crowd's mean
  field) so it joins at 37.8 s, and the jolt resets every phase to theta(0) - omega(60 - 58): frame 1800 = frame 0.
- Sound: every tick is a bell, built as impulse trains (fractional sample positions) convolved once per note: ~24k
  ticks in seconds. Section pairs by swing side stack into D major (left) / Asus2 (right); unsynced = short quiet
  bells, locked = long bells at 1/N gain so a locked riser's line rises out of the wash. The soloist's carousel steps
  one melody note per own tick, slot 0 on the drop, so the last 2 bars are a pickup that drags against the beat.
- Lessons:
  - Bell partials: the classic 2.76 partial turned 200 stacked bells into a clang; vibraphone-like 1/2/3/4 (+ a hint
    of 2.76) keeps the chords clean. Avoid C# in melody/stabs over the ringing D-major ticks (A chords kept sus2).
  - Locked sections get ~6 dB louder when the global coupling surges (spread 22 ms -> 0.5 ms = fully coherent sum):
    a natural build into the drop; balance the soloist above it.
  - Photosensitivity: a fast camera sweep across bright boards / dark steps flagged 30 % frame changes for 2.5 s
    (worst 3/s, at the limit). A static push with each section lighting up in turn reads better and flags 0.
  - A stage truss at y 13.6 sat right in front of the wide frontal cameras (a black band across the frame): keep rigging
    above the cameras' top rays (y 16.7 here).
  - LED show on lit cream bodies washed out to pastel; darken "off" pixels (body front * 0.32) and paint "on" pixels
    with the LED colour over the whole front of the body: the 10 x 20 display (tiers stack in a frontal view) reads.
  - Per-pixel sun tests against ~40 AABBs were 55 % of frame time: test each surface only against the boxes that can
    shadow it and evaluate the floor on a 2 x 2 grid (1.8 -> 1.3 s/frame).
- Feedback: (pending)
