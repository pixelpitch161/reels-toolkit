# Log: `projects/lowest_note`

Append new entries at the end (newest last).

## The Lowest Note in the Universe (`projects/lowest_note`)
- Perseus cluster as a floating low-poly island: a basin of glowing hot gas, the black hole (dark sphere
  + hot disc) with jets above it, bubbles that launch pressure-wave rings, observatory + lamp, dishes,
  crystals, orbiting member galaxies, a Chandra-style satellite. First video built on `reelkit`.
- Octave counter -57 -> 0 on a slide-up HUD. One list of crest launches drives both the rings on screen
  and the kicks in the mix: 55 -> 109 -> 219 BPM, flutter 7.3 -> 14.6 Hz, then the same pulse train at
  29.1 Hz is heard as B-flat0; B-flat1..4 stack (colour band per octave) under an original melody at
  109.3 BPM (= the note 8 octaves down). Ends on "HUM A B-FLAT", rewinds and loops (frame 0 vs 60 s
  differ by 0.4/255 on average).
- What worked: additive glows sit on top of everything, so a "black hole" glow must be painted *before*
  the dark sphere or the dark centre disappears. Galaxies read better as dithered glow balls / arm-only
  spirals than as solid emissive meshes (those looked like popcorn or pizzas).
- Phone check: sub kicks vanish on phone speakers, so each crest carries a short pitched B-flat4 "thock"
  and a click; short-term peaks above 250 Hz then land within ~4.5 dB of the climax.
- Focus audit over all 1800 frames: black hole + inner rings CoC = 0. Orbit phases chosen so no
  foreground galaxy crosses the black hole on screen.
- Runtime glyphs (`♭`, `≈`, `→`) added to `reelkit.font.GLYPHS` from the project, not in reelkit.
- Feedback on v1: looks cool, neat fact, but near-silent for most of it (no beat until 29 s), so it won't hold viewers.
- v2 re-cut (`lowest_note_v2.mp4`, v1 kept as `*_backup.py`): cold open on the climax, tape-stop to real speed,
  beat-synced spin-up (year/day/hour/minute slams + calendar/clock widget), drop at 11.7 s, then the v1 arc
  compressed; "SPED UP ...×" HUD line + log-frequency ruler with a "YOUR HEARING" window; camera shake, a
  screen-space ripple per crest, crystals/dishes/dust hop on the kicks; last bar slams into the cold open.
- Lessons:
  - One grid for everything: 1 beat = the hole's note 8 octaves down, so octaves -9..-4 are exactly 2 beats,
    1, 1/2, 1/4, 1/8, 1/16. Placing the drop 88 beats before 60 s makes the loop point a bar line.
  - A scene clock (`timeline.clock`) separate from video time handles "freeze" and "time-lapse": it stops in
    the tape-stop and whips round in the spin-up, adding exactly one extra lap over 60 s so orbits still loop.
  - Never-quiet check on phones: RMS above 250 Hz per 2 s window, not just full-band RMS. Fixes: open the
    "muffled" filter from ~340 Hz (not 160), crackle clicks over rumble, a longer B-flat4 thock on sparse crests.
  - A few sub hits (thud, slams) set the peak and pulled the whole mix down 4 dB: a fast peak limiter at the
    99.5th percentile of 2 ms peaks before normalising fixed it (sections now -13 to -16 dBFS RMS).
  - Loop-aware mastering: wrap anything past 60 s onto the start and give the limiter/reverb a pre-roll of the
    song's end; render the cold-open B-flat stack with t0 = 60 s so its phase continues from the last bar.
  - Sparse-click checks: measure the band level at each crest onset (15 ms), not a p98 of 10 ms frames, which
    misses kicks that are 1% of the time.
- Feedback: (pending)
