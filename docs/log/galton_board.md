# Log: `projects/galton_board`

Append new entries at the end (newest last).

## Chaos Makes a Perfect Curve (`projects/galton_board`)
- Galton board (12 rows, 4,096 paths, 13 bins) as a giant brass-and-walnut machine on the stage of a sunlit
  Victorian lecture hall: three arched plate-glass windows (low sun from front-left = key light, painted sun pinned
  in the first window), gallery with standing spectators and a pigeon, chalkboard ("GALTON, 1889"), bookcase,
  gas lamps, tiered benches with pawn spectators. Built to the new engagement rule: cold-open torrent with sound
  from frame 0, never quiet, lock-in at the drop.
- One cached simulation (`sim.py` -> `balls.npz`, `motion.py`) drives picture and sound: honest seeded runs (main
  1,000 balls seed 1889, cold-open torrent seed 4096) plus one staged all-left golden ball shown as a "what if",
  never counted, dissolved after landing. Real run: 0, 8, 9, 48, 119, 194, 229, 198, 116, 58, 20, 0, 1 vs ideal
  0.2 ... 225.6 ... (one real ball reached the right edge bin; kept, it's honest).
- Sound: each contact pings its column on C pentatonic (C5 centre), so every ball plays a 12-note random walk;
  decay and gain scale with ping density (melody -> rain -> hail, ~1,400 pings/s at the flood); landings = tock +
  bin note; each bin holds one note of a wide C6/9 chord and is as loud as the bin is full; after the drop a
  marimba arpeggio picks bin notes with probability = the real counts; Pascal rows ping left to right as they light.
- Lessons:
  - Raking sun on a deep board: full-depth bin dividers shadow most of each bin and long pegs streak. Shallow
    pegs (3.4 cm), balls 2.1 cm off the backplate, dividers only as deep as the balls, sun az 226 (not 238).
  - A light maple backplate + warm lamps washed out the subject; a mid cherry-walnut board with the lamps kept
    small (R ~1 m, no wrap light on planes facing away) makes rainbow balls and their shadows read.
  - Painter order with a ray-traced backplate: draw "behind" objects, then the backplate pass, then the machine.
    A base box drawn after the balls hid the bottom rows (its top face behind the backplate): draw base first,
    and hide draining balls once they pass the bin floor.
  - Tiny balls need a 1 px dark outline to separate from pegs; the hopper pile reads better without it.
  - Path trails (contact to contact, cool = left, warm = right) explain "coin flips" and the golden staircase better
    than arrows alone.
  - Hook caption fully on at frame 0: start it at -0.5 s and re-type it at 59.5 s, so the loop seam is clean.
  - Checks: focus CoC 0 on all 1800 frames (`focus_check.py`); science band 300-3000 Hz 9-25 dB over the music in
    every section with balls; quietest 0.5 s window -22.7 dBFS (never quiet).
- Feedback: (pending)
