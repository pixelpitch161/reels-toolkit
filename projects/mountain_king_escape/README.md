# The Mountain King Escape (IDEAS #19)

A glowing ball is trapped inside 12 concentric, counter-rotating crystal rings floating in the Mountain King's
cavern. Every bounce off a ring wall plays the next note of Grieg's *In the Hall of the Mountain King*; the tempo
accelerates (152 -> 265 BPM), so the ball visibly speeds up with the music. On each phrase ending the ball slips
through the current ring's gap and the ring shatters into shards that pile up on the floor. Ring 12 breaks on the
final chord, the ball rockets up the moon shaft and out of the summit, the camera pulls back to show the whole
mountain is floating inside a giant ring with a gap; the ball bounces off it (BONG, "TRAPPED AGAIN?"), the shards fly
back into 12 rings, every gap lines up and the ball drops straight back into the innermost ring = frame 0 (loops).

Satisfying-first. The bounces are timed to the score (scripted, standard for the format); nothing claims a real
physics simulation.

## Music source

Grieg's composition is public domain (1875/1888; Grieg died 1907). The notes were transcribed from Grieg's own piano
arrangement in the public-domain (CC0) Mutopia engraving `Mutopia-2013/12/07-1888` (typeset from The University
Society, New York, 1918; scan IMSLP #00581), parsed from its LilyPond source and re-written in our own format in
`score.py`. `python verify_score.py` re-downloads that source and diffs it bar by bar. Our arrangement and every sound
are synthesised here; no recording or modern arrangement was used.

| m (s) | phrase | section | ring breaks |
|---|---|---|---|
| -3.0 (video 57.0) | A | low pizzicato + bass (the restart frame 0 continues) | |
| 3.3 | A | + bassoon | ring 1 at 8.84 |
| 9.6 | B (up a fifth) | + strings, tempo climbing | ring 2 at 14.96 |
| 15.7 | B | | ring 3 at 18.22 |
| 21.4 | A | octave doubling, timpani | rings 4, 5 at 23.71, 26.01 |
| 26.7 | A | + brass, running bass | ring 6 at 31.03 |
| 31.6 | A | | ring 7 at 33.72 |
| 36.3 | B | full orchestra | ring 8 at 38.29 |
| 40.8 | B# (D#) | | ring 9 at 42.60 |
| 44.9 | A | prestissimo, snare rolls | rings 10, 11 at 46.67, 48.38 |
| 48.9 | coda | two stabs (G# A A# grace notes), the run | ring 12 on the final chord, 52.13 |

Near-misses (ball skims the gap edge, sting = the grace-note figure): 7.7, 21.5, 29.7, 37.2 + 37.6 (a pair), 44.4, 51.0 s.

## Build

```bash
python timeline.py                # print the plan (notes, ring breaks, captions)
python path.py                    # solve the ball path -> path.npz (cached; checks clearances)
python audio.py                   # -> audio.wav + levels and bounce-vs-music margins
python focus_check.py             # rings + ball CoC on all 1800 frames (expect 0)
python sheet_par.py test/sheet.png 0.5 6.4 8.86 20 38 46 50.5 52.25 54.4 56.6 58.5
python -m reelkit.video render.py audio.wav mountain_king_escape.mp4
python flash_check.py mountain_king_escape.mp4
python engagement_check.py audio.wav
python verify_score.py            # transcription vs the public-domain engraving
```

| file | what |
|---|---|
| `score.py` | the transcription (phrases A, B, B#, coda; bass, running bass, harmony) with source notes |
| `timeline.py` | tempo map, every note (= bounce), ring breaks, near-misses, loop-tail times, captions, HUD values |
| `geom.py` | ring radii/gaps/speeds, ball size, the drop-in; ring speeds chosen so no two neighbouring gaps line up at a hand-off; ring 12's gap is on top at the final chord |
| `path.py` | dynamic programme over wall angles per ring (long chords, clear of the moving gap, near-misses skim it, last chord ends in the gap), ball position at any time, the scripted tail spline |
| `shards.py` | closed-form shard flights (blast, tumble, hop, lie flat) and the reform back into rings |
| `scene.py` | the cavern (walls, roof + crack chimney, ledges, torches, crystal clusters, troll statues, stalactites), the cut-away front face, the mountain outside, the island, the giant ring's placement |
| `render.py` | sky / giant ring / exterior -> shell -> per-pixel floor (moon pool, live ring shadows, torch/crystal/ball light) -> props, rings, shards, ball -> per-pixel cut face (strata, snowline) -> locked DOF -> shaft, dust, moths, glows, trail, sparks -> post -> HUD + captions |
| `audio.py` | the arrangement and every instrument (pizzicato, bassoon, strings, brass, flute, crystal tink, timpani, bass drum, snare, cymbals, glass shatter, bell sting, gong), loop-aware master |
| `focus_check.py`, `flash_check.py`, `engagement_check.py`, `verify_score.py` | checks |
