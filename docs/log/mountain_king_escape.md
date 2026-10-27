# Log: `projects/mountain_king_escape`

Append new entries at the end (newest last).

## The Mountain King Escape (`projects/mountain_king_escape`)
- Satisfying-first "can it escape before it ends?" reel: a glowing ball in 12 counter-rotating crystal rings inside the
  Mountain King's cavern (cut-away diorama: moon shaft through a roof crack, torches, crystal clusters, troll statues,
  stalactites). Every melody note of Grieg's theme is one wall hit; phrase endings are escapes (ring shatters into shards
  that pile up on the floor). Ring 12 breaks on the final chord, the ball goes up the shaft and out of the snow-capped
  summit, the camera pulls back: the floating mountain sits inside a giant ring with a gap (BONG, "TRAPPED AGAIN?"); the
  shards fly back into rings, all 12 gaps line up, the ball drops through into ring 1 = frame 0.
- Music: transcribed from the CC0 Mutopia engraving of Grieg's own piano arrangement (University Society 1918), parsed from
  its LilyPond source (`verify_score.py` diffs it bar by bar). The engraving's right hand in bars 63-64 repeats bars 59-60
  (G natural, D natural) while its left hand has D#: read as a copy-paste slip, so B# uses D# and the run uses G#.
- One timeline: 11 phrases (A A B B A A A B B# A coda), tempo 152 for 32 beats then 152 -> 265 (8.8 bounces/s at the end);
  the instrument climbs pizz -> bassoon -> strings -> octaves -> brass -> tutti -> prestissimo -> coda stabs.
- Lessons:
  - Ball path = a dynamic programme per ring over wall angles on a 0.5 deg grid anchored on the escape angle: long chords
    (138/158/172 deg for eighth/quarter/longer gaps), one circulation direction per ring, every hit clear of the gap at
    that note's time, near-misses restricted to a skim window, last chord ends exactly in the gap. Exact and instant
    (0.4 s for 267 notes), unlike random search.
  - Hand-off rule: if ring k+1's gap is near ring k's gap at the escape, the ball would fly through both. Pick each ring's
    spin (outermost first) as close to plan as gives >= 0.9 rad between them; ring 12's spin is an integer number of turns
    so its gap is on top at the final chord (straight up the shaft).
  - "Two near-misses in a row" can't be consecutive hits (the gap barely moves between them): skim, cross, skim the other
    edge (hits i and i+2).
  - Loop with a scripted tail: rings get their phase from music time m = t - 60 once they re-form, and every gap is set to
    point up at the moment the falling ball passes it, so the drop-in threads all 12 gaps and frame 0 continues it.
  - The cut-away face (per-pixel strata) dominated wide shots with wide wavy bands; finer, closer-toned strata, a raised
    front ceiling and a snowline under a jagged silhouette made it read as a mountain.
  - Glows need an occlusion test even for the hero: the ball's glow showed through the rock while it was inside the chimney.
  - 15 Hz camera shake over bright ring bands risks flash-test failures; ~6 Hz, ~1.5 px is enough.
  - The ring-1 break in close-up (all ring hues stepping at once + burst) is the only general flash (28 %); a slower hue step
    would remove it if needed.
  - Checks: CoC 0 on all frames; bounce voice 10-26 dB over the accompaniment (1.5-6 kHz); -9.9 LUFS; seam 4.8 vs 5.6.
- Feedback: (pending)
