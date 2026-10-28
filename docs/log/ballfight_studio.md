# Log: BALLFIGHT Studio (`docs/studio/PLAN.md`)

Append new entries at the end (newest last).

## Step 1, session 1: porting fighters + moods into the kit (session_01CzTfqdSsfsbYNYgocwEbMx, Oct 27, stopped after TUBA)
- Ported, each checked with a kit example that rebuilds its episode (`projects/ballfight/examples/`, in the replay
  guard's KIT_EPISODES): **DRUMS** + mood **balkan_fire** (#28, `drums_vs_violin`), **VIOLIN** (as last delivered in
  #42) + **VIOLA** + mood **orchestral_drill** (#42, `viola_vs_violin`), and TUBA + mood brass_house were being verified at the stop (see the hand-off). ORGAN and PIANO v2 came from the
  #45 session in parallel. The rest, with the method, the harness and the order: `docs/studio/PORT_HANDOFF.md`.
- #42 (kit-lineage): hashes identical on all five guard seeds; 17 test frames pixel-identical once two gaps were closed
  (the HUD list's hold for MELODY AT LAST / VIOLA JOKE; VIOLA JOKE's rope wobble, which #42 keyed to the victim's index
  so the ropes shook at t = 1 s: the port shakes them when the note lands; with #42's timing the two affected frames
  are identical too); audio.wav bit-identical from 3.0 to 59.5 s, every pre-master bus identical except the last bar's
  cello / violin notes (vibrato phase, below).
- #28 (before the card, the v2 effects, the HUD list and the loudness rule): the kit replays it with the new legacy
  timeline option `FIGHT_BAR = 1` (+ `REASSEMBLE_AT`), hashes identical; the DRUMS' 'kit' and 'solo' buses and the
  bed's bass / accordion / pad / cimbalom buses sample-identical; the DRUMS layers (rings, cymbal, beater, reach ring)
  match by eye. balkan_fire got new VS-card sections (#28 had no card). DRUMS has no v2 effects of its own yet.
- Lesson: the Pixel Pitch violin / viola / cello (and accordion, drums, organ, piano) synths draw from a module-level
  RNG, so a note's samples depend on how many notes of that synth were made before it: the order the fighters'
  parts are scored matters (`Sound.play_first`; #42 scored the violin first).
- Kit hooks added (all optional, defaults = the old output; the four existing kit examples stayed frame- and
  sample-identical after each): Look `ball_tint`, `sprite_lit`, `laugh`, `flashes`, `spark_kinds`, `overlay`, rope()
  -> (t, k); Sound `buses`, `ko`, `own_block`, `play_first`; a fighter module's `DEFAULT_SOLO`; HUD holds for
  'melody' / 'joke'; timeline `T_PICKUP`, `CARD`, legacy `FIGHT_BAR` / `REASSEMBLE_AT`; mood `doors_open_gain`,
  `count_whoosh_gain`. The check harness is in `tools/ballfight_port/`.
- The container restarted every 20-35 min from ~15:30 (running processes and background agents stop; files survive),
  which is why the session stopped after TUBA.
