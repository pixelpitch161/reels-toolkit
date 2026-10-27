# Log: `projects/ballfight_drums_vs_violin`

Append new entries at the end (newest last).

## BALLFIGHT #2: Drums vs Violin (`projects/ballfight_drums_vs_violin`)
- Second BALLFIGHT episode (#28) on #27's engine, ring, HUD and count-in (imported; new folder with its own timeline,
  sim, tune, seed search, renderer copy and audio). DRUMS pulse a drumhead ring out on every beat (new weapon kind
  `pulse`: reach grows per landed pulse; haste rolls it on the 16ths), VIOLIN spins its bow (blade + new `spin_grow`:
  2.4 -> 14 rad/s). Abilities: DRUM ROLL / CRASH CYMBAL (the bow bounces off) / DOUBLE KICK (beater taps an 8th early,
  then stomps); PIZZICATO / TREMOLO / SCREECH. Mood B dark Balkan fire: D harmonic minor, 141.64 BPM (36 bars).
- Honest run, seed 568 (6 fits in 932; DRUMS 57 % / VIOLIN 43 % over 767 decided): Drums edge the opening, the bow
  takes over at 18.2 s (62-26 at ~36 s), DOUBLE KICK at 37.7 starts the comeback, SCREECH at 43.6, DRUMS win on the
  last hit at 50.91 s with 2.8 HP (the second lead change is the K.O. itself).
- Engine lessons: keep additions opt-in so #27 replays bit-identically (hash every recorded array + the event list
  before and after: seed 88 + four others). A new 16th-grid loop inside `run()` first shadowed the loop's `step`
  counter (events out of order, K.O. at 4 s) - name new locals carefully in shared loops.
- Rules lessons: both weapons grow on landed hits, so leads snowball; lead changes stay rare (~0.5 per run, #27 ~0.7)
  and smaller growth didn't help. With the hard windows the fit rate is ~0.6 %, still fine for an early-stopping
  search (~1 min for 932 runs). `tune.py` here takes generic overrides (`D.dmg=..`, `V.spin_grow=..`, `scale=..`).
- Sound lessons: the bow's quarter turns sampling a 64-slot melody give the "slow strokes -> frantic spiccato" arc for
  free (1 note/s at the start, a full 16th line at 14 rad/s). First balance had the violin ~24 dB over the bed
  (accordion inaudible); target the fighters ~8-13 dB over the bed in 300-5000 Hz. One transient set the master's
  normalisation (early fight at -21 dBFS): normalise on the 99.97th percentile and let the soft clipper take the rest.
- Mid-production direction: the ideas chat gave every BALLFIGHT instrument its own colour (DRUMS orange, VIOLIN
  violet). Applied by patching the imported scene at runtime (corner cushions, middle rope per side, neutral skirt band,
  corner lights, LED sides, crowd tints) so the shared scene.py is untouched; full re-render (7 min) + mux.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 1.8x);
  flash 0 general; opening -17.8 dBFS, build +3.2 dB. Render 0.23 s/frame/core (7 min) + 3 min mux.
- Feedback: (pending)
