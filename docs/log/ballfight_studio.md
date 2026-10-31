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
- **EUPHONIUM (#38): ported by #48 Euphonium vs Trombone** (session_01CzTfqaiKkq, Oct 28) into `kit/fighters/euphonium.py`
  (#38's BELL MORTAR styles + render layers + its audio part, unchanged; the key-bound calls follow the config's KEY, C
  minor = #38's notes). Check config `examples/bassclarinet_vs_euphonium` (seed 1269, #38's DMG_SCALE / SERIES /
  SHARD_SEED / FX_SEED / TITLE_LINES, LINES = #38's MELODY + BASS_8THS): replay hash identical to #38's own sim for
  1269/1/2/3/500 (now in `tools/ballfight_replay_hash.py`); 10 frames pixel-identical in a matched harness (#38 with the
  bass clarinet's drawn layers and the UI off; the kit with a stub bass clarinet carrying only #38's bass-clarinet effects,
  run in #38's particle / flare order): shells + landing circles, MARCHING ORDERS, both IDENTITY CRISIS bars (the amber
  bass clarinet in its ball), the NOT A TUBA rain, the shatter; the euphonium bus bit-identical to #38's (max diff 0).
  Kit additions (additive, defaults unchanged): the HUD holds for `barrage` / `copy`, a Look hook `ball_sprite` (the
  enemy's model in its ball) and `extra_sprites` (those views in the sprite cache). New with the port: what IDENTITY
  CRISIS copies is drawn by the enemy's own look module bound to the euphonium's side (so in amber), copied kinds only;
  #38's copies of the bass clarinet (slap, twin, shadow) still sound as #38 played them, their pictures wait for the
  BASS CLARINET port. Not ported: #38's dark-drumline bed (a mood still to port) and the BASS CLARINET.
- **BASSOON (#39 v4): ported by #52 Oboe vs Bassoon** (session_01CzTfqc, Oct 29) into `kit/fighters/bassoon.py`
  (#39's TIPTOE / BOCAL HOOK / MOUNTAIN KING styles + render layers + its audio part, unchanged; the key-bound calls
  follow the config's KEY, E minor = #39's notes; new: while OUT OF TUNE its notes play flat against a sour twin, the
  Grieg quote included, silent in #39 where nothing detunes). Check config `examples/bassoon_vs_barisax` (seed 77,
  #39's DMG_SCALE / DMG_SIDE / SERIES / SHARD_SEED / FX_SEED, LINES = #39's BASSOON_8THS): replay hash identical to
  #39's own sim for 77/1/2/3/500 (now in `tools/ballfight_replay_hash.py`); matched harness (#39 with the bari's drawn
  layers, effects and the UI off; the kit with a stub bari, #39's beat pulse): 6 of 10 frames pixel-identical (corner
  bob, mid-hop stretch, backstab slashes, STACCATO, landing squash, MOUNTAIN KING's dance), the bassoon's own layers
  identical in all 10 (crook + swirl, the champion's solo hop); the other 4 differ only in the bari's GROWL rope shake
  (2), its brass shatter (1) and faint canvas light on old claw marks at 47.0 s (the marks themselves identical). Kit additions (additive,
  defaults unchanged, replay guard identical): draw_ball squash / stretch via optional state keys `sx` / `sy`, an
  airborne depth bias `air_bias` and painter height `paint_h`, Look attribute `speed_lift`, HUD holds for `hook` /
  `mountain`. Not ported: #39 v2's leap over a swinging WRECKING BALL (waits for the BARI SAX port), #39's ska-punk bed.
- **BASS CLARINET (#38, as #40 v2 fought it): ported by #55 Bass Clarinet vs Bassoon** (session_01CzTfqb, Oct 31) into
  `kit/fighters/bass_clarinet.py` (#38's ENDPIN ANCHOR / SLAP TONGUE / MULTIPHONICS / LIGHTS OUT styles copied unchanged,
  #40's render layers as Look hooks, #40's audio part as Sound; key-bound notes follow the config's CHORDS, octave by role:
  'bass' = the line down at the bass, keeps its fundamental, pumps the root into the slam; any other role = #40's counter-line
  an octave up). Check config `examples/barisax_vs_bassclarinet` (seed 128, #40's DMG_SIDE / SERIES / SHARD_SEED / FX_SEED,
  LINES = #40's COUNTER_8THS, the bari's later `steer` pinned off in the config as #40's sim.py does): replay hash identical
  to #40's own sim for 128/1/2/3/500; the bass clarinet's picture state (whirl, sink, twin, slap paddle, dim, glow)
  identical to #40's on all 1,411 fight frames; 6 frames (whirl, SLAP TONGUE, MULTIPHONICS, LIGHTS OUT sunk + dim, the
  eruption, a late slap) compared with #40's renderer: its own layers pixel-identical, the differences all on the bare
  bari (sprite, harness, crane, smash dust, its HUD hold, crowd flashes) plus faint canvas dither from its missing coral
  flare; its raw bus bit-identical to #40's `bcl` bus. Kit additions (additive, defaults unchanged, replay guard
  identical): draw_ball's optional state key `sink` (#38 LIGHTS OUT: lowered, shrunk, darkened), HUD holds for `twin` /
  `lights_out`, `ANC` / `LO` in story.REC_KEYS; a Look may expose `dim_keep(F)` to stay lit under LIGHTS OUT's dim.
  Not ported: #40's construction-site bed (and the BARI SAX).
- **TUBA (#32 v3, as #36 fought it): ported by #56 Bass Drum vs Tuba** (session_01CzTfqd, Oct 31) into
  `kit/fighters/tuba.py` (#32's bell / suction cone / BRASS WALL dome / SUBSONIC ripple styles and #28's `mask_of`
  copied unchanged, #36's render layers as Look hooks - the enemy gulped, held inside the bell with its HP number hidden,
  spat into the ropes; the bulge - and #36's audio part as Sound: oom = the root, pah = the fifth, OOMPAH doubling). Check
  config `examples/trombone_vs_tuba` (seed 358, #36's DMG_SIDE / SERIES / key / chords / solos / shard + fx seeds; on
  `big_top`, since #36's brass house isn't a kit mood): replay hash identical to #36's own sim for 358/1/2/3/500; 10
  fight frames (inhale, gulp, held, spit, the cannonball into the ropes, BRASS WALL, SUBSONIC, the final gulp, the
  shards) and the VS card pixel-identical to #36's renderer with the UI on and off; the tuba and solo buses
  bit-identical. Kit addition (additive, replay guard identical): opt-in Look hook `canvas_first` (light under both
  fighters' decals: #36's suction cone was lit before the trombone's). Added to the replay guard's KIT_EPISODES.
