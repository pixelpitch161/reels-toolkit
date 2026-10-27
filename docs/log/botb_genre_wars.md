# Log: `projects/botb_genre_wars`

Append new entries at the end (newest last).

## Battle of the Bands: Genre Wars (`projects/botb_genre_wars`)
- BotB episode on #20's engine, imported unchanged (`episode.py` appends `battle_of_the_bands/` to `sys.path` so the
  episode's own `sim.py`, `scene.py`, ... win): ORCHESTRA = SAND, ROCK = BALL, EDM = RIPPLE, JAZZ = VINE, new names +
  colours only. One original song (A minor, | Am | F | Dm | E |, 118.03 BPM, 8-bar melody) arranged four ways
  (drums / bass / harmony / lead per genre: timpani + galloping cellos + violin pad/brass + horns; kit + pick bass +
  3-3-2 power chords + distorted lead; four-on-the-floor + off-beat sub + supersaw stabs + saw lead; swung ride/brush +
  walking upright + rootless comping (Am9 Fmaj9 Dm9 E7b9) + tenor sax). Every layer gated by its genre's moves (#20
  rule); a stinger in the new leader's style on each lead change; the stage mood (wash, ROCK pyro on each bar, EDM
  lasers stepping on the beat, JAZZ blue haze + spot cones, ORCHESTRA gold motes) follows the leader.
- Seed: the 61 s search (bell on bar 10 = 20.33 s, ring period = one beat, hopper sweep 61/9 s) fits less often than
  #20 (5 in 192); RIPPLE/EDM wins most runs (98 of 164 decided). Seed 189 picked over the top score (63) because three
  genres lead (the song changes genre three times) and its first knockout isn't early.
- Lessons (sound):
  - "The song follows the leader" needs per-layer curves: bass + harmony follow share gently (everyone audible), drums
    share^2.2 and the melody share^4 (the groove and the tune belong to the leader). Normalise gains equal-power
    (sum of squares = 1): the #20-style (1/sum)^0.55 went above 1 when one genre dominated, so the win dropped in level.
  - Calibrate layers on a phone-weighted band (150 Hz - 8 kHz): on raw RMS the EDM stem won with sub energy phones
    can't play. The opening's free-note voices need their own calibration (the jazz piano came in ~10 dB hot).
  - Gating density differs per genre (rings claim in bursts, the ball in streaks), so a leading ROCK sounded quieter
    than EDM. A per-genre up-only leveler (toward the level its share calls for, ~1.5 s smoothing) makes the loudest
    genre the leader again while stutters stay audible.
  - The tape-stop into the crack left the file's last 20 ms silent (-40 dBFS) against the boom on frame 0: a riser +
    snare roll that builds right up to the seam (6 ms tail, no fade) passes level and click.
  - mp4 audio seam: same AAC padding as #22 (640 samples). The container signals the audio as exactly 61.000 s
    (duration_ts 2928000); only ffmpeg's decoder appends the padding, which `loopcheck out.mp4` reads.
- Lessons (look): pyro cannons at x +-2.8 were off-frame (the ground stacks sit at the frame edges): +-1.95 on the
  front edge. A series-title line inside the scoreboard panel clipped the bottom of the wall; it became an impact
  caption right after the hook. Caption scheduler: event captions with priorities and minimum holds, lead changes
  wait for a more important caption, fillers fill fight stretches with a blank > 2.4 s or a hold > 5.2 s.
- Checks: safe zone OK (0.25 s steps); wall + props CoC 0 on all 1830 frames; flash 0 general flashes; opening -14.3
  dBFS, build +3.1 dB; loopcheck (render) PASS. Render 0.35 s/frame/core (10.8 min on 2 cores). Time: ~55 min to v1.
- Feedback: (pending)
