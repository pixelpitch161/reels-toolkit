# Log: `projects/rock_paper_scissors_orchestra`

Append new entries at the end (newest last).

## Rock Paper Scissors Orchestra (`projects/rock_paper_scissors_orchestra`)
- Battle-family reel in mood A (ominous cinematic orchestra): 120 low-poly tokens (40 stones, 40 folded sheets, 40 pairs of
  scissors) chase on the round stage of a low-poly opera house (curtains, gilded boxes with pawn patrons, crystal chandelier
  key light, footlights, orchestra pit with a conductor beating time, the stalls). A catch converts the caught piece and
  steps the hunter's section (ROCK timpani/taiko, PAPER pizzicato + tremolo, SCISSORS brass stabs); volume = headcount.
  Seed 60: ROCK crashes to 5 pieces at 30 s, swells to 110, wipes out SCISSORS at 38.5 s and dooms itself; PAPER wins at
  46.6 s. The twist caption ("ROCK KILLED / SCISSORS... / AND DOOMED ITSELF") lands its last line on a low brass sting.
- Loop: frame 0 = the clump in three colour wedges bursting out; from 57.2 s the winner spirals into the same slots (Hungarian
  assignment) and turns to face outward, arriving at exactly 61.0 s. Only the colours change at the seam (picture seam 2.2x).
- Engine (`rps.py`): seek nearest prey / flee nearest predator (inside a sight radius, sliding along the rim when cornered),
  wander, damping, speed cap, elastic disc collisions, conversion on touch with a short immunity. Plain seek/flee eliminated
  a team by ~15-20 s with 2-4 s mop-ups; halving the speed got the timing right and a "panic sprint" (+30 % cap with a
  predator in sight, for everyone) produced the near-death comebacks (teams dip to 1-6 pieces and recover). Fits are rare
  (~3 % of seeds), so `seed_search.py` logs every run's counts (`check/raw_*.jsonl`) to re-score without re-running.
  Balance over 102 runs: ROCK 24, PAPER 23, SCISSORS 26 wins.
- Lessons:
  - With catches on from frame 0 the packed clump converts instantly; starting catches at bar 1 makes the hook the three
    wedges fanning out (one word per beat, each team lit as its word lands).
  - A rich caption line only has base / hi / alt colours: the three-word hook is three one-line captions with their own y0,
    each landing on its beat (draw_captions(..., y0=...), ui.IMPACT_STAGGER set per caption for the twist).
  - The win "blaze" washed the boards to pale yellow and hid the white paper tokens: blaze the chandelier, not the floor.
  - Section automation before the compressor was flattened (build 0 dB); applying it after the normalisation gave +3 dB.
  - AAC frames are 1024 samples: 61.0 s at 48 kHz pads the last frame with 640 samples of silence. ffmpeg-based decoders
    play it, so `loopcheck out.mp4` sees a level step at the seam even though the wav passes (Pixel Pitch avoids it with
    1840-frame / 61.333 s files). Open question for the user.
- Render 0.25 s/frame/core (7.5 min on 2 cores). Time: ~45 min to v1.
- Feedback: (pending)
