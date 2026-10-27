# Log: `projects/dice_waltz`

Append new entries at the end (newest last).

## Mozart's Dice Waltz (`projects/dice_waltz`)
- The 1792 waltz dice table (Simrock, Berlin; sold as Mozart's). Each roll's sum picks a bar, which lands on a
  parchment staff while a self-playing fortepiano plays it. Back half: ten cups' waltzes stack on different
  timbres and the wall histogram locks into the 2-12 triangle.
- Note data: 2-of-3 vote over three transcriptions (13 bars disputed), stored in our own format in `waltz_table.py`.
  Column harmony verified from the table (every column shares one harmony), so stacking stays consonant.
- Look: cut-away salon. The room shell (floor, walls, window) is ray-traced per pixel. The key light is the moon
  through an arched window; the moon is pinned in the arch as a painted backdrop, because a high camera can't see
  the real sky direction.
- Lessons:
  - A portrait frame only shows about 0.43 x distance in width, so keep rooms narrow (3.4 units here).
  - Use the camera's `sy` (screen y of the target) to keep readouts below the caption band.
  - A wall overlay must sort nearer than its board in painter order, or the board covers it.
  - A volumetric beam at 0.55 strength washed every frame with diagonal bands; 0.2 reads as atmosphere.
  - Seamless loop: reset everything before 60 s (staff clears, chart rolls back up, histogram sinks).
  - Audio: carve about -8 dB at 2.5-6 kHz in the music bus and keep high layers out of that band, so the dice
    clacks sit about 10 dB above even ten stacked waltzes.
- Feedback: (pending)
