# Log: `projects/marble_waterfall_organ`

Append new entries at the end (newest last).

## Marble Waterfall Organ (`projects/marble_waterfall_organ`)
- Satisfying-first (user direction), abstract void (no room, no ground). A brass-and-glass marble machine floats on a
  stone plinth: central spiral-screw lift in a cage, 8 tiers (kick drum + toms, snare + 3 glass discs, 10 tuned pipes,
  7 glass bowls, 7 tuned bars, a 1.25-turn 16-bar spiral ramp, crash + ride, 10 golden bell bars), basin ring of ~640
  marbles. Opens with every gate open (~120 hits/s of random D-major notes, escapees streak away weightless); gates
  shut one by one on the downbeat (6, 10, 14, 18, 20, 24, 32 s), the melody gate sticks (35-40 s, two failed attempts,
  half-beat silent gap), snaps shut = drop at 40 s; lever at 57 s springs every gate open back into frame 0.
- One cached list (`sim.py` -> `marbles.npz`) drives marbles, element flashes, basin splashes and every note: routed
  marbles roll down a chute then take an exact ballistic hop (v0 = d/T + g T/2) onto the element at the note time;
  sprayed marbles take 1-3 random contacts (each contact = that element's note) then escape. Spiral: one marble per bar
  rings all 16 ramp bars on 16ths. Full song ~33 marble hits/s. Music original, 120 BPM, | Bm | G | D | A | from bar 8
  so the drop and the final chord both land on Bm.
- Lessons:
  - A lat-long nebula texture has a seam: index it with az/2pi + 0.5 so the seam sits behind the camera (it showed as a
    hard vertical band right behind the machine at az 0).
  - Peak-normalising the master flattened the build (wash = drop at -9 dBFS). Set the drop's RMS to a target (-10.5) and
    only limit peaks: wash -15, kick -13.5, bass -12.6, drop -10, back to -15 at the seam.
  - Loop-seam click: the reverb tail of the last seconds was added twice at the start. Fold every bus into a periodic
    60 s buffer first, then circular reverb (`reverb(concat(send, send))[NS:]`) and circular filters (1 s wrapped pre-roll).
  - Simultaneous chord notes on one chute rolled as one blob: stagger their hop times (x(1 + 0.32 j)).
  - Visible orb vs shadows: draw the orb low beside the crown (el 7) but shade with a higher key (el 28) so the machine's
    shadows land on the plinth top.
  - Holdout framing: from the front over the bell arc (pitch ~16) so the instrument arc doesn't hide the stuck flap;
    red open flaps + lamps, sparks on the failed attempts, a light burst on every gate shut.
  - Flash check: 9 general flashes on the first render. Causes: bar-line void bursts with a white core, a nebula pulse on
    every bar, and fast camera moves over the textured nebula (fly-down, an 84 deg whip pan). Chord-tinted burst ring with a
    3-frame rise, palettes of similar luminance, pulse 5 %, gentler pans -> 2 (motion only, worst 1 per second).
  - Speed: 2,195 faces + ~250 moving marbles + pile + lift, ~0.47 s/frame, ~7 min on 2 cores.
- Feedback: (pending)
