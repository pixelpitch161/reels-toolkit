# Log: `projects/perfect_loop`

Append new entries at the end (newest last).

## The Perfect Loop (`projects/perfect_loop`)
- 15 uncoupled pendulums (n = 51..65 swings per minute, exact analytic motion), so the 60 s video is one
  physics cycle and frame 1800 == frame 0. Clockmaker's attic at sunset: a cut-away gable with a round window
  behind the rig, so the low sun throws every bob's shadow forward across the floor (the wave is drawn twice).
- One trigger list (t = 60 m / n_k) drives the flashes, the coloured floor light and the plucks: 3 octaves of
  C pentatonic, longest = lowest = red. The middle pendulum (n = 58) is exactly the 58 BPM beat, so bass, pad
  and kick lock to the physics; groups become eighths/triplets/sixteenths by themselves.
- Lessons:
  - Window glazing bars cast stripes across whatever the beam is aimed at: a rose window with a clear centre
    (ring + spokes outside it) keeps the subject's light clean and still draws a pattern on the floor.
  - A top beam directly above a row of bobs hides them from high cameras near the row axis: keep high shots
    at yaw 30-55 (the end-on view only for "2 teams", where the halves sit left and right of the beam).
  - First sun/beam pass was one orange wash: cool ambient (blue-violet) + warm key keeps sunset colour with range.
  - Group tie-lines read as extra strings until dashed, 2 px and tinted per group.
  - `reelkit.audio.pad()` is ~20 dB quieter than a pluck at equal gain; measure per-bus RMS before balancing.
    Plucks sit 11-12 dB above the music in 300-3000 Hz in the back half.
  - Loop audit: seam diff 1.4 with the frame-0 flash disabled vs 3.4 for a normal frame step; the only jump is
    the intended unison flash. Focus audit: 0 blur on every bob in all 1800 frames (`focus_check.py`).
- Feedback (v1): scale the concept up (more pendulums) -> v2.
- v2 re-cut (`perfect_loop_v2.mp4`, v1 kept as `*_backup.py`, which import each other so v1 still rebuilds). Feedback on
  v1: scale it up. Now 100 pendulums (n = 51..150, 34.4 -> 4.0 cm, 10 050 plucks ~167/s) in a long gallery lit by four round
  windows in the left wall; physics = video + 8 s, so it opens mid-pattern (15 groups at frame 0), the ladder lands every 2-5 s
  and all 100 line up at 52 s; frame 1800 = physics 68 = 8 = frame 0. Every milestone is exact and on the 120 BPM grid
  (pendulum n = 120 plucks on every beat); hat subdivision = current group count. HUD counter races 60 / (52 - t): 8 -> 100 -> 1.
- Lessons (v2):
  - A 2.5 m row in portrait reads best near end-on (yaw 5-10, pitch 40-47): row up the frame, swing sideways. At yaw ~20 the
    swing partly runs along the row on screen and group structure vanishes; side views turn the swing into depth.
    (v1's yaw 30-55 rule was for 15 big bobs; with 100 the swing direction matters more than beam occlusion.)
  - 100 overlapping glow sprites + white-hot cores + additive trails sum to white: saturated cores, glows only on a pluck,
    and trails composited "strongest sample wins" with a screen blend keep the rainbow.
  - Groups in a 100-bob row are faint in stills: bold 2 px tie-lines in fixed group colours plus matching markers on the
    HUD phase-ring clumps make the count readable; the ring alone already shows clean clumps.
  - Wider release at the long (far) end (8.5 -> 2.0 cm, capped 30 deg) evens out the on-screen swing in perspective.
  - Focus audit only over on-screen bobs (a glide past the front end puts bobs behind the lens).
  - Audio: the in-phase 100-voice strum sets the peak normalisation and left the mix ~4 dB quiet; a 24 ms lookahead limiter
    (+4.5 dB drive) brought it to -18 -> -11 dBFS with the strum still loudest. A saw pad is what eats the pluck band:
    dropping it an octave, lp 700 Hz and -4 dB took the margin from 4 to 9-10 dB (17-20 dB before the band enters).
  - The rolled last-pluck sweeps (60 - j*T_k) converge visibly in the spectrogram: the physics makes its own riser.
  - Seam diff 3.2/255 vs 3.0 for a normal frame step; subject blur 0 on all 1800 frames.
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
