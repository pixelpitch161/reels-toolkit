# Log: BALLFIGHT Studio (`docs/studio/PLAN.md`)

Append new entries at the end (newest last).

## Porting log (PLAN step 1: every fighter and mood into the kit)
Episode sessions port a fighter when a crossover first needs it; those below are done, so the porting session skips them
(the full list is in `projects/ballfight/README.md`, "Ported so far", and `kit/fighters/`).
- **TRUMPET (#29 v3): ported by #46 Trumpet vs Clarinet** (session_01CzTfqa, Oct 28) into `kit/fighters/trumpet.py`
  (#29's styles + render layers + its audio part, unchanged). Check config `examples/trumpet_vs_sax` (seed 191, FIGHT!
  on bar 1 via the new kit option `FIGHT_BAR`, #29's SERIES / SHARD_SEED / FX_SEED): replay hash identical to #29's own
  sim for seeds 191/1/2/3/500 (now in `tools/ballfight_replay_hash.py`); 12 frames pixel-identical in a matched harness
  (#29 with the sax's drawn layers and the UI off; the kit with a stub sax carrying only #29's sax effects, #29's beat
  pulse and #29's real-time flare clock): combos, FANFARE, DOUBLE TONGUE, the HIGH C charge + blast, the shatter; the
  trumpet bus matches #29's to 1e-16. Not ported: #29's noir-jazz bed (a mood still to port) and the retired SAX.
