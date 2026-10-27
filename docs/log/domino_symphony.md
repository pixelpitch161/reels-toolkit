# Log: `projects/domino_symphony`

Append new entries at the end (newest last).

## Domino Symphony (`projects/domino_symphony`)
- Satisfying-first reel: one paw tap -> 64-ray rainbow fan (out of step) -> onion-dome funnel locks into 16ths -> bell lane
  (every 4th rings) -> 4 chord lanes -> 4,096-domino field reveals a pixel music note in a chevron sweep -> 2-lane spiral
  (16ths -> triplets -> 32nds) -> red holdout on a plank ledge teeters -> 1.5x staircase booms D C# B A G F# E -> 1.28 m giant
  slams on the drop -> 3+3 frame lanes race round the layout playing the song on chime bars -> rewind into the loop.
- No physics sim: `layout.py` gives every box a strike time from the grid and `boxes.py` tips it about its front edge; the
  same table drives picture, clicks, chimes and the counter. The funnel's domino count is solved so the total is exactly 10,000.
- Lessons:
  - A 1 -> 64 binary fan that funnels back: route the funnel as polar "meridians" around the fan origin whose lateral spread
    (r * h(r)) halves between merge rows; adjacent lanes never cross and pairs merge where they would get too close. Offset the
    merging pair across the PARENT lane's own direction (outer lanes curve sideways). A cheap footprint SAT audit plus a
    mm-scale relaxation pass took standing overlaps to 0; fallen overlaps at Y junctions are normal.
  - Painted-face reveals: a domino's painted back faces the camera while it stands whenever the camera is on that side (all
    the views from the start of the layout), so keep field dominoes cream on every face until they tip.
  - 10k boxes at ~0.5 s/frame: vectorise tilt/corners/normals/shading for all boxes, splat anything under ~2.4 px as pixels
    (depth-tested), draw the rest as PIL polygons in painter chunks of <= 12 per lane; give every big/special box (holdout,
    staircase) its own chunk or same-lane order puts the step behind over the holdout. Sun shadows: project sun-facing faces
    to the floor into a screen-space mask; tiny dominoes cast dotted streaks.
  - Exposure: window-beam volumetrics fill the whole room when the windows are tall, so keep the density very low
    (acc/46, x0.28) or every frame goes yellow-hazy; floor sun x1.7, bloom threshold 236.
  - Audio: 1,000 clicks/s are peaky; a fast compressor on the click bus stops the master limiter pumping the whole field
    section down. reelkit pad() needed ~+16 dB (music bus gain 4.5) to be heard at all. Full-band RMS undersells a click-heavy
    section; check approximate K-weighted loudness (`loudness_check.py`). A tom-led build keeps the 2-6 kHz band for the clicks.
  - Rewind: one scene clock tau (56 -> -0.3 s eased, then real time) rewinds dominoes, the cat and the counter together; a
    tape-reversed final chord fills the gap before tau reaches the last dominoes.
- Checks: subject CoC 0 on all 1800 frames; domino band +8..+43 dB over the music wherever clicks play; seam diff 1.9.
- Feedback: (pending)
